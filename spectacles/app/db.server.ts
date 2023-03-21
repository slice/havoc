import { Pool } from 'pg';

let pg: Pool;

declare global {
  var __db__: Pool;
}

if (process.env.NODE_ENV === 'production') {
  pg = new Pool();
  pg.connect();
} else {
  if (!global.__db__) {
    global.__db__ = new Pool();
    global.__db__.connect();
  }
  pg = global.__db__;
}

export default pg;
