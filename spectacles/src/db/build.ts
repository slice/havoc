import type { Branch, Build, DetectedBuild } from "@/models/build";
import db from "@/db";

/** Fetches the latest build on a branch. */
export function latestBuildOnBranch(branch: Branch): DetectedBuild {
  let statement = db.prepare(`
    SELECT build_id, branch, detected_at, build_number
    FROM detections
    WHERE branch = $branch
    ORDER BY detected_at DESC
    LIMIT 1;
  `);
  let row = statement.get({ branch }) as any;

  return {
    number: row.build_number,
    id: row.build_id,
    branch: row.branch,
    detectedAt: new Date(row.detected_at * 1000),
  };
}

/** Fetches the latest build IDs on each branch. */
export function latestBuildIDs(): { [branch in Branch]?: string } {
  let statement = db.prepare(`
    SELECT DISTINCT
      branch,
      first_value(build_id) OVER (
        PARTITION BY branch
        ORDER BY detected_at DESC
      ) AS build_id
    FROM build_deploys;
  `);
  let rows = statement.all() as any[];

  return Object.fromEntries(rows.map((row) => [row.branch, row.build_id]));
}

/** Finds the last build that was detected on a branch before a certain date. */
export function findPreviousBuild(branch: Branch, before: Date): Build | null {
  let statement = db.prepare(`
    SELECT build_number, build_id
    FROM detections
    WHERE branch = ? AND detected_at < ?
    ORDER BY detected_at DESC
    LIMIT 1
  `);
  let row = statement.get(branch, before.getTime() / 1000) as any;

  return row != null ? { id: row.build_id, number: row.build_number } : null;
}

export interface Detection {
  branch: Branch;
  detectedAt: Date;
}

/** Fetches all times a build was detected on a branch. */
export function fetchDetections(buildID: string): Detection[] {
  let statement = db.prepare(`
    SELECT branch, detected_at
    FROM build_deploys
    WHERE build_id = ?
  `);
  let rows = statement.all(buildID) as any[];

  return rows.map((row) => ({
    branch: row.branch,
    detectedAt: new Date(row.detected_at * 1000),
  }));
}

/** Fetches a specific build by its ID or number. */
export function fetchBuild(
  by: { id: string } | { number: number }
): Build | null {
  let primaryExpr = "number" in by ? "build_number = ?" : "build_id = ?";

  let row = db
    .prepare(`SELECT build_number, build_id FROM builds WHERE ${primaryExpr}`)
    .get("number" in by ? by.number : by.id) as any;

  return row == null ? null : { id: row.build_id, number: row.build_number };
}
