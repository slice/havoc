import type { Branch, Build, DetectedBuild } from "@/models/build";
import db from "@/db";

/** Fetches the latest build on a branch. */
export async function latestBuildOnBranch(
  branch: Branch
): Promise<DetectedBuild> {
  let result = await db.query(
    `
      SELECT build_id, branch, detected_at, build_number
      FROM detections
      WHERE branch = $1::discord_branch
      ORDER BY detected_at DESC
      LIMIT 1;
    `,
    [branch]
  );
  let row = result.rows[0];

  return {
    number: row.build_number,
    id: row.build_id,
    branch: row.branch,
    detectedAt: row.detected_at,
  };
}

/** Fetches the latest build IDs on each branch. */
export async function latestBuildIDs(): Promise<{
  [branch in Branch]?: string;
}> {
  let result = await db.query(`
    SELECT DISTINCT
      branch,
      first_value(build_id) OVER (
        PARTITION BY branch
        ORDER BY detected_at DESC
      ) AS build_id
    FROM build_deploys;
  `);

  return Object.fromEntries(
    result.rows.map((row) => [row.branch, row.build_id])
  );
}

/** Finds the last build that was detected on a branch before a certain date. */
export async function findPreviousBuild(
  branch: Branch,
  before: Date
): Promise<Build | null> {
  let result = await db.query(
    `
      SELECT build_number, build_id
      FROM detections
      WHERE branch = $1 AND detected_at < $2
      ORDER BY detected_at DESC
      LIMIT 1
    `,
    [branch, before]
  );
  let row = result.rows[0];

  return row != null ? { id: row.build_id, number: row.build_number } : null;
}

export interface Detection {
  branch: Branch;
  detectedAt: Date;
}

/** Fetches all times a build was detected on a branch. */
export async function fetchDetections(buildID: string): Promise<Detection[]> {
  let result = await db.query(
    `
      SELECT branch, detected_at
      FROM build_deploys
      WHERE build_id = $1
    `,
    [buildID]
  );

  return result.rows.map((row) => ({
    branch: row.branch,
    detectedAt: row.detected_at,
  }));
}

/** Fetches a specific build by its ID or number. */
export async function fetchBuild(
  by: { id: string } | { number: number }
): Promise<Build | null> {
  let primaryExpr = "number" in by ? "build_number = $1" : "build_id = $1";

  let result = await db.query(
    `SELECT build_number, build_id FROM builds WHERE ${primaryExpr}`,
    ["number" in by ? by.number : by.id]
  );
  let row = result.rows[0];

  return row == null ? null : { id: row.build_id, number: row.build_number };
}
