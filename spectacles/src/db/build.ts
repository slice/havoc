import type { Branch, DetectedBuild } from "@/models/build";
import db from "@/db";

export function latestBuildOnBranch(branch: Branch): DetectedBuild {
  const statement = db.prepare(`
    SELECT build_id, branch, detected_at, build_number
    FROM detections
    WHERE branch = $branch
    ORDER BY detected_at DESC
    LIMIT 1;
  `);
  const row = statement.get({ branch }) as any;

  return {
    number: row.build_number,
    id: row.build_id,
    branch: row.branch,
    detectedAt: new Date(row.detected_at * 1000),
  };
}
