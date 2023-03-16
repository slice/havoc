BEGIN;

-- Detected frontend builds.
CREATE TABLE IF NOT EXISTS detected_builds (
  -- A number that seemingly increments for every build Discord creates,
  -- present in the client scripts.
  build_number INTEGER PRIMARY KEY,

  -- A unique hash/identifier for the build. Currently entirely in hexadecimal,
  -- present in the client scripts and as the `X-Build-ID` header.
  build_id TEXT UNIQUE NOT NULL

  -- We don't have a `detected_at`/`first_detected_at` column because that
  -- information can be determined from the `detected_builds_on_branches` table
  -- in a more consistent manner that makes it clear on which branch we saw it
  -- appear first.
  --
  -- When detecting a build for the first time, however, it must be inserted
  -- into this table before it can be inserted into
  -- `detected_builds_on_branches`.
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

-- Assets (CSS/JS/etc. files) associated with a build.
--
-- TODO: We should probably scan the surface assets for deep ones (Twemoji,
-- artwork, icons, etc.).
CREATE TABLE IF NOT EXISTS detected_assets (
  build_id TEXT NOT NULL REFERENCES detected_builds(build_id),

  -- A "surface" asset is exposed directly in the app HTML, and not within an
  -- asset itself. Surface assets solely consist of the stylesheets and scripts
  -- necessary to boot the client, and are what the browser fetches first.
  surface BOOLEAN NOT NULL,

  -- What purpose this asset serves, given it's a surface script. The scripts
  -- that appear directly in the app HTML serve distinct purposes, and it's
  -- useful to detect and store this information.
  --
  -- In practice, we assign the type of surface scripts through the order they
  -- appear in the HTML. However, this is fragile and may break in the future,
  -- necessitating the implementation of more resilient heuristics.
  determined_surface_script_type surface_script_type DEFAULT NULL,

  -- The filename of the asset, including file extension. This can be fetched
  -- from `discord.com/assets/...`.
  --
  -- Unsure if this should be `UNIQUE`; I haven't personally witnessed Discord
  -- reuse an asset between builds nor have I verified that they have never
  -- done this, but we shouldn't blow up if they decide to do that in the
  -- future or have already done so.
  name TEXT NOT NULL
);

-- Instances of a Discord build detected on a specific branch.
--
-- A single build can appear on multiple branches, although not necessarily
-- at the same time.
CREATE TABLE IF NOT EXISTS detected_builds_on_branches (
  build_id TEXT NOT NULL REFERENCES detected_builds(build_id),
  branch discord_branch NOT NULL,
  detected_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMIT;
