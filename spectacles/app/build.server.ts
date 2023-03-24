import { Branch, DetectedBuild } from './build';
import pg from './db.server';

export async function latestBuildOnBranch(
  branch: Branch
): Promise<DetectedBuild> {
  const result = await pg.query(
    `
    SELECT build_id, branch, detected_at, build_number
    FROM detections
    WHERE branch = $1::discord_branch
    ORDER BY detected_at DESC
    LIMIT 1;
  `,
    [branch]
  );
  const row = result.rows[0];

  return {
    number: row.build_number,
    id: row.build_id,
    branch: row.branch,
    detectedAt: row.detected_at,
  };
}
