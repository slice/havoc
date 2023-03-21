import { Branch, DetectedBuild } from './build';
import pg from './db.server';

export async function latestBuildOnBranch(
  branch: Branch
): Promise<DetectedBuild> {
  const result = await pg.query(
    `
    SELECT dbob.build_id, dbob.branch, dbob.detected_at, db.build_number
    FROM detected_builds_on_branches dbob
    INNER JOIN detected_builds db ON db.build_id = dbob.build_id
    WHERE branch = $1::discord_branch
    ORDER BY dbob.detected_at DESC
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
