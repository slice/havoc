import { Branch, Build } from './build';
import pg from './db.server';

export async function latestBuildOnBranch(branch: Branch): Promise<Build> {
  const result = await pg.query(
    `
    SELECT bob.build_id, bob.detected_at, db.build_number
    FROM detected_builds_on_branches bob
    INNER JOIN detected_builds db ON db.build_id = bob.build_id
    WHERE branch = $1::discord_branch
    ORDER BY bob.detected_at DESC
    LIMIT 1;
  `,
    [branch]
  );
  const row = result.rows[0];

  return {
    buildNumber: row.build_number,
    buildId: row.build_id,
    detectedAt: row.detected_at,
  };
}
