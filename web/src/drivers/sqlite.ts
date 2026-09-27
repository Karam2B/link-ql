import {
  normalizeSqlJsExec,
  type QueryResult,
  type SqlClient,
} from "../sql-client";

export async function createSqliteClient(): Promise<SqlClient> {
  // Dynamic import: sql.js browser entry has no ESM default export under Vite.
  const [sqlJs, wasmUrl] = await Promise.all([
    import("sql.js/dist/sql-wasm.js"),
    import("sql.js/dist/sql-wasm.wasm?url"),
  ]);
  const initSqlJs = sqlJs.default ?? sqlJs;
  const SQL = await initSqlJs({ locateFile: () => wasmUrl.default });
  const db = new SQL.Database();

  return {
    driver: "sqlite (sql.js)",
    query(sql: string): QueryResult {
      return normalizeSqlJsExec(db.exec(sql));
    },
  };
}
