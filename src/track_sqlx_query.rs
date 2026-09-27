//! Test-only tracing layer that records SQL emitted by sqlx (`target = "sqlx::query"`).
//!
//! [`install`] / [`watch_sqlx_calls`] run the closure under a `track_sqlx_query` tracing span.
//! Only sqlx events whose span context is that tree (or a child, including sqlite worker
//! threads) are stored in that scope's buffer.

use std::cell::RefCell;
use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex, Once};

use tracing::Subscriber;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing_subscriber::{
    Layer, layer::Context, prelude::*, registry::LookupSpan, util::SubscriberInitExt,
};

const CAPTURE_SPAN_NAME: &str = "track_sqlx_query";

static GLOBAL_INIT: Once = Once::new();
static CAPTURE_SCOPE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

thread_local! {
    static PENDING_BUFFERS: RefCell<Vec<Arc<Mutex<Vec<String>>>>> = RefCell::new(Vec::new());
}

fn push_pending_buffer(buffer: Arc<Mutex<Vec<String>>>) {
    PENDING_BUFFERS.with(|stack| stack.borrow_mut().push(buffer));
}

fn take_pending_buffer() -> Option<Arc<Mutex<Vec<String>>>> {
    PENDING_BUFFERS.with(|stack| stack.borrow_mut().pop())
}

fn ensure_global_subscriber() {
    GLOBAL_INIT.call_once(|| {
        let _ = tracing_subscriber::registry()
            .with(SqlxQueryLayer)
            .with(tracing_subscriber::filter::LevelFilter::DEBUG)
            .try_init();
    });
}

#[derive(Clone)]
struct Capture {
    queries: Arc<Mutex<Vec<String>>>,
}

struct SqlxQueryLayer;

struct SqlxQueryVisitor {
    summary: Option<String>,
    db_statement: Option<String>,
}

impl Visit for SqlxQueryVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "summary" => self.summary = Some(value.to_owned()),
            "db.statement" => self.db_statement = Some(value.to_owned()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "summary" && self.summary.is_none() {
            self.summary = Some(format!("{value:?}").trim_matches('"').to_owned());
        }
    }
}

fn capture_sql(summary: Option<String>, db_statement: Option<String>) -> Option<String> {
    let stmt = db_statement.as_deref().unwrap_or("").trim();
    if !stmt.is_empty() {
        return Some(stmt.to_owned());
    }
    summary.filter(|s| !s.is_empty())
}

fn capture_from_ctx<S>(
    event: &tracing::Event<'_>,
    ctx: Context<'_, S>,
) -> Option<Arc<Mutex<Vec<String>>>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let mut current = ctx.event_span(event)?;
    loop {
        if let Some(cap) = current.extensions().get::<Capture>() {
            return Some(cap.queries.clone());
        }
        current = current.parent()?;
    }
}

impl<S> Layer<S> for SqlxQueryLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != CAPTURE_SPAN_NAME {
            return;
        }
        let buffer =
            take_pending_buffer().unwrap_or_else(|| Arc::new(Mutex::new(Vec::new())));
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(Capture { queries: buffer });
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        if event.metadata().target() != "sqlx::query" {
            return;
        }
        let Some(buffer) = capture_from_ctx(event, ctx) else {
            return;
        };

        let mut visitor = SqlxQueryVisitor {
            summary: None,
            db_statement: None,
        };
        event.record(&mut visitor);

        let Some(sql) = capture_sql(visitor.summary, visitor.db_statement) else {
            return;
        };

        buffer
            .lock()
            .expect("track_sqlx_query mutex poisoned")
            .push(sql);
    }
}

/// Handle for the current test's sqlx query capture scope.
///
/// Call [`Actions::take`] immediately after each `client.exec` you want to assert on.
/// [`Actions::clear`] is for setup helpers that should discard schema/migration SQL
/// before the test body runs.
#[derive(Clone)]
pub struct Actions {
    queries: Arc<Mutex<Vec<String>>>,
}

impl Actions {
    /// Discard all SQL captured so far in this [`install`] / [`watch_sqlx_calls`] scope.
    pub fn clear(&self) {
        self.queries
            .lock()
            .expect("track_sqlx_query mutex poisoned")
            .clear();
    }

    /// Take all SQL captured since the last [`Actions::clear`] or [`Actions::take`], and
    /// reset the buffer.
    pub fn take(&self) -> Vec<String> {
        take_queries(Arc::clone(&self.queries))
    }

    /// Spawn a future on the Tokio runtime, inheriting the current capture span.
    #[allow(dead_code)]
    pub fn spawn<F>(&self, fut: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        use tracing::Instrument;
        tokio::spawn(fut.instrument(tracing::Span::current()))
    }
}

/// Drop connection setup statements that may appear once per pool connection.
#[allow(dead_code)]
pub fn without_pragma(statements: Vec<String>) -> Vec<String> {
    statements
        .into_iter()
        .filter(|statement| !statement.trim_start().starts_with("PRAGMA "))
        .collect()
}

#[cfg(test)]
#[allow(dead_code)]
pub fn assert_sql_eq(actual: Vec<String>, expected: Vec<String>) {
    pretty_assertions::assert_eq!(without_pragma(actual), without_pragma(expected));
}

fn take_queries(queries: Arc<Mutex<Vec<String>>>) -> Vec<String> {
    std::mem::take(&mut *queries.lock().expect("track_sqlx_query mutex poisoned"))
}

/// Run a synchronous closure in an isolated sqlx-query capture scope.
///
/// Returns `(captured_sql, closure_result)`.
#[allow(dead_code)]
pub fn install<F, R>(f: F) -> (Vec<String>, R)
where
    F: FnOnce(Actions) -> R,
{
    let _lock = CAPTURE_SCOPE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    ensure_global_subscriber();
    let queries = Arc::new(Mutex::new(Vec::new()));
    push_pending_buffer(Arc::clone(&queries));
    let actions = Actions {
        queries: Arc::clone(&queries),
    };

    let span = tracing::info_span!(CAPTURE_SPAN_NAME);
    let _enter = span.enter();
    let result = f(actions);
    drop(_enter);
    let _ = take_pending_buffer();

    (take_queries(queries), result)
}

/// Run an async closure in an isolated sqlx-query capture scope.
///
/// Assert SQL with [`Actions::take`] inside the closure, immediately after each
/// `client.exec`. Do not rely on the return value of this function for query checks.
pub async fn watch_sqlx_calls<F, Fut, R>(f: F) -> R
where
    F: FnOnce(Actions) -> Fut,
    Fut: Future<Output = R>,
{
    use tracing::Instrument;

    let _lock = CAPTURE_SCOPE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    ensure_global_subscriber();
    let queries = Arc::new(Mutex::new(Vec::new()));
    push_pending_buffer(Arc::clone(&queries));
    let actions = Actions {
        queries: Arc::clone(&queries),
    };

    let result = async move { f(actions).await }
        .instrument(tracing::info_span!(CAPTURE_SPAN_NAME))
        .await;

    let _ = take_pending_buffer();

    let _ = take_queries(queries);

    result
}
