BEGIN;

-- Instances of a Discord build detected on a specific branch.
CREATE TABLE IF NOT EXISTS detected_builds_on_branches (
  build_number INTEGER NOT NULL,
  branch TEXT NOT NULL,
  detected_at INTEGER NOT NULL
) STRICT;

COMMIT;
