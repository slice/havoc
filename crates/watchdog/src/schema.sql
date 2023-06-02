BEGIN;

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

CREATE TYPE discord_branch AS ENUM (
  'development',
  'canary',
  'ptb',
  'stable'
);

CREATE TYPE surface_script_type AS ENUM (
  -- The script that handles kickstarting the loading of other Webpack chunks
  -- that aren't surface level assets.
  'chunkloader',

  -- The Webpack chunk containing CSS class mappings.
  'classes',

  -- The Webpack chunk assumed to contain various vendor packages, such as
  -- Sentry.
  'vendor',

  -- The Webpack chunk containing the bulk of the application code.
  'entrypoint'
);

-- WItnessed frontend assets (CSS/JS/etc. files).
CREATE TABLE IF NOT EXISTS assets (
  -- The filename of the asset, including file extension. This can be fetched
  -- from `discord.com/assets/...`.
  name TEXT PRIMARY KEY,

  -- A "surface" asset is exposed directly in the app HTML, and not within an
  -- asset itself. Surface assets solely consist of the stylesheets and scripts
  -- necessary to boot the client, and are what the browser fetches first.
  surface BOOLEAN NOT NULL DEFAULT FALSE,

  -- What purpose this asset serves, given it's a surface script. The scripts
  -- that appear directly in the app HTML serve distinct purposes, and it's
  -- useful to detect and store this information.
  --
  -- In practice, we assign the type of surface scripts through the order they
  -- appear in the HTML. However, this is fragile and may break in the future,
  -- necessitating the implementation of more resilient heuristics.
  surface_script_type surface_script_type,

  -- The Webpack chunk ID associated with this asset, assuming that it's a
  -- "deep" (non-surface) script.
  script_chunk_id INTEGER
);

-- Witnessed frontend assets associated with a frontend build.
CREATE TABLE IF NOT EXISTS build_assets (
  build_id TEXT NOT NULL REFERENCES builds(build_id),
  asset_name TEXT NOT NULL REFERENCES assets(name),

  PRIMARY KEY (build_id, asset_name)
);

CREATE TABLE IF NOT EXISTS module_ids (
  -- The name of the script asset containing Webpack modules. We assume that
  -- assets are immutable: once they are built and uploaded to Discord's CDN,
  -- we can parse module IDs out of the script and be done with the work
  -- forever.
  name TEXT NOT NULL REFERENCES assets(name),

  -- The Webpack module ID contained in this asset.
  module_id INTEGER NOT NULL,

  UNIQUE (name, module_id)
);

-- Instances of a Discord build detected on a specific branch.
--
-- A single build can appear on multiple branches, although not necessarily
-- at the same time.
CREATE TABLE IF NOT EXISTS build_deploys (
  build_id TEXT NOT NULL REFERENCES builds(build_id),
  branch discord_branch NOT NULL,
  detected_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- A view that includes the build number alongside the build ID. Useful, since
-- we also want the build number a lot of the time.
CREATE VIEW detections AS
  SELECT
    builds.build_id,
    builds.build_number,
    deploys.branch,
    deploys.detected_at
  FROM build_deploys deploys
  INNER JOIN builds ON deploys.build_id = builds.build_id;

COMMIT;
