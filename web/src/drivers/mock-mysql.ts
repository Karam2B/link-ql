import type { QueryResult, SqlClient } from "../sql-client";

/** Placeholder showing how a MySQL (or other) adapter would look. */
export function createMockMysqlClient(): SqlClient {
  return {
    driver: "mysql (mock)",
    query(sql: string): QueryResult {
      if (/select\s+4\s+as\s+example/i.test(sql)) {
        return { columns: ["example"], rows: [[4]] };
      }
      return { columns: [], rows: [] };
    },
  };
}
