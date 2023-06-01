import "server-only";
import SQLite3 from "better-sqlite3";

let db: SQLite3.Database;

const SQLITE_URL = process.env.SQLITE_URL;
if (SQLITE_URL == null) {
  throw new Error("No $SQLITE_URL specified");
}

db = new SQLite3(SQLITE_URL);
db.pragma("foreign_keys = ON");
db.pragma("journal_mode = WAL");

export default db;
