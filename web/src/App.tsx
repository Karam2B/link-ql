import { createSignal, onMount, Show } from "solid-js";
import { createMockMysqlClient } from "./drivers/mock-mysql";
import { createSqliteClient } from "./drivers/sqlite";
import type { SqlClient } from "./sql-client";

type WasmBindings = {
  hello_world: () => string;
  execute_select_example: (client: SqlClient) => Promise<unknown>;
  execute_select_example_normalized: (client: SqlClient) => Promise<unknown>;
};

export default function App() {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [driver, setDriver] = createSignal<"sqlite" | "mysql">("sqlite");
  const [client, setClient] = createSignal<SqlClient | null>(null);
  const [wasm, setWasm] = createSignal<WasmBindings | null>(null);
  const [hello, setHello] = createSignal<string>("");
  const [rawResult, setRawResult] = createSignal<string>("");
  const [normalized, setNormalized] = createSignal<string>("");

  async function loadDriver(kind: "sqlite" | "mysql") {
    setLoading(true);
    setError(null);
    try {
      const sqlClient =
        kind === "sqlite"
          ? await createSqliteClient()
          : createMockMysqlClient();
      setClient(sqlClient);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }

  onMount(async () => {
    try {
      const mod = await import("./wasm-pkg/linked_sql_wasm.js");
      await mod.default();
      setWasm(mod as WasmBindings);

      const bindings = mod as WasmBindings;
      setHello(bindings.hello_world());
      await loadDriver("sqlite");
    } catch (e) {
      setError(
        e instanceof Error
          ? e.message
          : "Failed to load WASM. Run: npm run build:wasm",
      );
      setLoading(false);
    }
  });

  async function runWasmQuery() {
    const bindings = wasm();
    const sqlClient = client();
    if (!bindings || !sqlClient) return;

    try {
      const raw = await bindings.execute_select_example(sqlClient);
      setRawResult(JSON.stringify(raw, null, 2));

      const norm = await bindings.execute_select_example_normalized(sqlClient);
      setNormalized(JSON.stringify(norm, null, 2));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function onDriverChange(kind: "sqlite" | "mysql") {
    setDriver(kind);
    await loadDriver(kind);
    setRawResult("");
    setNormalized("");
  }

  return (
    <main>
      <h1>linked-sql WASM demo</h1>
      <p class="subtitle">
        SolidJS hosts a SQL client; Rust WASM awaits two{" "}
        <code>query(...)</code> calls in sequence.
      </p>

      <Show when={error()}>
        <div class="card error" role="alert">
          {error()}
        </div>
      </Show>

      <div class="row">
        <label>
          Driver
          <select
            value={driver()}
            onChange={(e) =>
              void onDriverChange(e.currentTarget.value as "sqlite" | "mysql")
            }
            disabled={loading()}
          >
            <option value="sqlite">SQLite (sql.js)</option>
            <option value="mysql">MySQL (mock)</option>
          </select>
        </label>
        <button type="button" onClick={() => void runWasmQuery()} disabled={!client() || !wasm()}>
          Run WASM query
        </button>
      </div>

      <div class="card">
        <h2>WASM hello_world()</h2>
        <pre class={hello() ? "" : "muted"}>{hello() || "…"}</pre>
      </div>

      <div class="card">
        <h2>Active client</h2>
        <pre class={client() ? "" : "muted"}>
          {client()?.driver ?? "loading…"}
        </pre>
      </div>

      <div class="card">
        <h2>execute_select_example() — raw JS return</h2>
        <pre class={rawResult() ? "" : "muted"}>
          {rawResult() || "(click Run WASM query)"}
        </pre>
      </div>

      <div class="card">
        <h2>execute_select_example_normalized()</h2>
        <pre class={normalized() ? "" : "muted"}>
          {normalized() || "(click Run WASM query)"}
        </pre>
      </div>
    </main>
  );
}
