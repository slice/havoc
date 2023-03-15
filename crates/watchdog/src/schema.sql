BEGIN;

CREATE TABLE IF NOT EXISTS detected_builds (
  build_number INTEGER PRIMARY KEY,
  build_id TEXT UNIQUE NOT NULL
) STRICT;

-- Instances of a Discord build detected on a specific branch.
CREATE TABLE IF NOT EXISTS detected_builds_on_branches (
  build_id TEXT NOT NULL REFERENCES detected_builds(build_id),
  branch TEXT NOT NULL,
  detected_at INTEGER NOT NULL
) STRICT;

COMMIT;
