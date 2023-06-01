BEGIN IMMEDIATE TRANSACTION;

-- Every frontend build that has been witnessed.
CREATE TABLE IF NOT EXISTS builds (
  -- A number that seemingly increments for every build Discord creates,
  -- present in the client scripts.
  build_number INTEGER PRIMARY KEY,

  -- A unique hash/identifier for the build. Currently entirely in hexadecimal,
  -- present in the client scripts and as the `X-Build-ID` header.
  build_id TEXT UNIQUE NOT NULL

  -- We don't have a `detected_at`/`first_detected_at` column because that
  -- information can be determined from the `build_deploys` table in a more
  -- consistent manner that makes it clear on which branch we saw it appear
  -- first.
  --
  -- When detecting a build for the first time, however, it must be inserted
  -- into this table before it may be inserted into `build_deploys`.
);

-- Witnessed frontend assets (CSS/JS/etc. files).
CREATE TABLE IF NOT EXISTS assets (
  -- The filename of the asset, including file extension. This can be fetched
  -- from `discord.com/assets/...`.
  name TEXT PRIMARY KEY NOT NULL,

  -- A "surface" asset is exposed directly in the app HTML, and not within an
  -- asset itself. Surface assets solely consist of the stylesheets and scripts
  -- necessary to boot the client, and are what the browser fetches first.
  surface BOOLEAN NOT NULL DEFAULT FALSE,

  -- What purpose this asset serves, given that it's a surface script. The
  -- scripts that appear directly in the app HTML serve distinct purposes, and
  -- it's useful to detect and store this information.
  --
  -- In practice, we assign the type of surface scripts through the order they
  -- appear in the HTML. However, this is fragile and may break in the future,
  -- necessitating the implementation of more resilient heuristics.
  --
  -- Possible values: "chunkloader", "classes", 'vendor", "entrypoint"
  surface_script_type TEXT,

  -- The Webpack chunk ID associated with this asset, assuming it is a "deep"
  -- (non-surface) script.
  script_chunk_id INTEGER
);

-- Witnessed frontend assets associated with a frontend build.
CREATE TABLE IF NOT EXISTS build_assets (
  build_id TEXT NOT NULL,
  asset_name TEXT NOT NULL,

  FOREIGN KEY (build_id) REFERENCES builds(build_id),
  FOREIGN KEY (asset_name) REFERENCES assets(name),
  PRIMARY KEY (build_id, asset_name)
);

CREATE TABLE IF NOT EXISTS module_ids (
  -- The name of the script asset containing Webpack modules. We assume that
  -- assets are immutable: once they are built and uploaded to Discord's CDN,
  -- we can parse module IDs out of the script and be done with the work
  -- forever.
  name TEXT NOT NULL,

  -- The Webpack module ID contained in this asset.
  module_id INTEGER NOT NULL,

  FOREIGN KEY (name) REFERENCES assets(name)
);

-- Instances of a Discord build detected on a specific branch.
--
-- A single build can appear on multiple branches, although not necessarily
-- at the same time.
CREATE TABLE IF NOT EXISTS build_deploys (
  build_id TEXT NOT NULL,
  -- Values: "canary", "ptb", "stable", "development"
  branch TEXT NOT NULL,
  detected_at INTEGER NOT NULL DEFAULT (strftime('%s')),

  FOREIGN KEY (build_id) REFERENCES builds(build_id)
);

-- A view that includes the build number alongside the build ID. Useful, since
-- we also want the build number a lot of the time.
CREATE VIEW IF NOT EXISTS detections AS
  SELECT
    builds.build_id,
    builds.build_number,
    build_deploys.branch,
    build_deploys.detected_at
  FROM build_deploys
  INNER JOIN builds ON builds.build_id = build_deploys.build_id;

COMMIT;
