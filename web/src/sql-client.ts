/** Normalized row set returned by any SQL backend adapter. */
export type QueryResult = {
  columns: string[];
  rows: unknown[][];
};

/**
 * Minimal SQL client contract passed into linked-sql WASM.
 * Implement for SQLite (sql.js), MySQL (wasm mysql), PG, etc.
 */
export interface SqlClient {
  /** Driver label shown in the UI. */
  readonly driver: string;
  /** Run SQL; sync or async — WASM awaits Promises via JsFuture. */
  query(sql: string): QueryResult | Promise<QueryResult>;
}

export function normalizeSqlJsExec(
  execResult: { columns: string[]; values: unknown[][] }[],
): QueryResult {
  if (execResult.length === 0) {
    return { columns: [], rows: [] };
  }
  const { columns, values } = execResult[0];
  return { columns, rows: values };
}
