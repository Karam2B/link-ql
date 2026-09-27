// Browser WASM entry points. The host (SolidJS) provides a JS `SqlClient` object;
// Rust calls `query(sql)` on it instead of linking sqlx directly.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
pub fn hello_world() -> String {
    "Hello from linked-sql WASM!".into()
}

async fn call_query(sql_client: &JsValue, sql: &str) -> Result<JsValue, JsValue> {
    let query = Reflect::get(sql_client, &JsValue::from_str("query"))?;
    let query = query
        .dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str("sql_client.query must be a function"))?;

    let raw = query
        .call1(sql_client, &JsValue::from_str(sql))
        .map_err(|e| JsValue::from_str(&format!("query failed: {e:?}")))?;

    if raw.is_instance_of::<js_sys::Promise>() {
        JsFuture::from(
            raw.dyn_into::<js_sys::Promise>()
                .map_err(|_| JsValue::from_str("sql_client.query returned invalid Promise"))?,
        )
        .await
    } else {
        Ok(raw)
    }
}

/// Run two demo queries sequentially; returns `[first, second]` raw JS values.
#[wasm_bindgen]
pub async fn execute_select_example(sql_client: JsValue) -> Result<JsValue, JsValue> {
    let first = call_query(&sql_client, "SELECT 4 as example").await?;
    let second = call_query(&sql_client, "SELECT 5 as example").await?;
    let out = Array::new();
    out.push(&first);
    out.push(&second);
    Ok(out.into())
}

/// Same as `execute_select_example` but each result is normalized to `{ columns, rows }`.
#[wasm_bindgen]
pub async fn execute_select_example_normalized(sql_client: JsValue) -> Result<JsValue, JsValue> {
    let first = normalize_query_result(call_query(&sql_client, "SELECT 4 as example").await?)?;
    let second = normalize_query_result(call_query(&sql_client, "SELECT 5 as example").await?)?;
    let out = Array::new();
    out.push(&first);
    out.push(&second);
    Ok(out.into())
}

fn normalize_query_result(raw: JsValue) -> Result<JsValue, JsValue> {
    if raw.is_null() || raw.is_undefined() {
        return Ok(Object::new().into());
    }

    // Already `{ columns, rows }`
    if Reflect::has(&raw, &JsValue::from_str("columns"))?
        && Reflect::has(&raw, &JsValue::from_str("rows"))?
    {
        return Ok(raw);
    }

    // sql.js `db.exec` shape: `[{ columns, values }]`
    if Array::is_array(&raw) {
        let arr = Array::from(&raw);
        if arr.length() == 0 {
            return Ok(js_sys::JSON::parse(r#"{"columns":[],"rows":[]}"#).map_err(|e| e)?);
        }
        let first = arr.get(0);
        let columns = Reflect::get(&first, &JsValue::from_str("columns"))?;
        let values = Reflect::get(&first, &JsValue::from_str("values"))?;
        let out = Object::new();
        Reflect::set(&out, &JsValue::from_str("columns"), &columns)?;
        Reflect::set(&out, &JsValue::from_str("rows"), &values)?;
        return Ok(out.into());
    }

    Err(JsValue::from_str("unrecognized query() return shape"))
}
