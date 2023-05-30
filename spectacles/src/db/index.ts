import "server-only";
import { Pool, Client } from "pg";

let pg: Pool | Client;

const globalForPostgres = global as unknown as {
  pg: Pool | Client | undefined;
};

if (process.env.NODE_ENV === "production") {
  console.log("[database] Creating Postgres pool (production).");
  pg = new Pool();
  pg.connect();
} else {
  if (!globalForPostgres.pg) {
    // Don't bother creating a connection pool, because it'll be recreated on
    // every request: https://github.com/vercel/next.js/issues/44330
    globalForPostgres.pg = new Client();
    globalForPostgres.pg.connect();
  }
  pg = globalForPostgres.pg;
}

export default pg;
