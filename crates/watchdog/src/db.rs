use anyhow::Result;
use havoc::{
    discord::{AssetCache, AssetsExt, Branch, FeAsset, FeAssetType, FeBuild, RootScript},
    scrape::extract_assets_from_chunk_loader,
};
use sqlx::{postgres::PgRow, Postgres, Row};

#[derive(Clone)]
pub struct Db {
    pub pool: sqlx::Pool<Postgres>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DetectedAssetKind {
    // TODO: Scan for deep assets.
    #[allow(dead_code)]
    Deep,

    Surface,
    SurfaceScript(RootScript),
}

impl DetectedAssetKind {
    fn is_surface(&self) -> bool {
        use DetectedAssetKind::*;
        matches!(self, Surface | SurfaceScript(..))
    }
}

impl Db {
    pub fn new(pool: sqlx::Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Fetch the last known build hash on a branch.
    pub async fn last_known_build_hash_on_branch(&self, branch: Branch) -> Result<Option<String>> {
        Ok(sqlx::query(
            "SELECT build_id
            FROM detected_builds_on_branches
            WHERE branch = $1::discord_branch
            ORDER BY detected_at DESC
            LIMIT 1",
        )
        .bind(branch.to_string().to_lowercase())
        .fetch_optional(&self.pool)
        .await?
        .map(|row: PgRow| row.get(0)))
    }

    pub async fn detected_assets(&self, build: &FeBuild, cache: &mut AssetCache) -> Result<()> {
        let mut transaction = self.pool.begin().await?;

        async fn detected_asset(
            transaction: &mut sqlx::Transaction<'_, Postgres>,
            build: &FeBuild,
            asset: &FeAsset,
            kind: DetectedAssetKind,
        ) -> Result<()> {
            sqlx::query(&format!(
                "INSERT INTO detected_assets (build_id, surface, determined_surface_script_type, name)
                VALUES ($1, $2, {determined_surface_script_type}, $3)
                ON CONFLICT DO NOTHING",
                determined_surface_script_type = match kind {
                    DetectedAssetKind::Deep | DetectedAssetKind::Surface => "NULL".to_owned(),
                    DetectedAssetKind::SurfaceScript(kind) =>
                        format!("'{:?}'", kind).to_lowercase() + "::surface_script_type",
                }
            ))
            .bind(&build.manifest.hash)
            .bind(kind.is_surface())
            .bind(asset.filename())
            .execute(transaction)
            .await?;

            Ok(())
        }

        for stylesheet in build
            .manifest
            .assets
            .iter()
            .filter_by_type(FeAssetType::Css)
        {
            detected_asset(
                &mut transaction,
                build,
                stylesheet,
                DetectedAssetKind::Surface,
            )
            .await?;
        }

        let chunks = extract_assets_from_chunk_loader(&build.manifest, cache).await?;
        for (chunk_id, chunk_asset) in chunks.iter() {
            detected_asset(
                &mut transaction,
                build,
                chunk_asset,
                DetectedAssetKind::Deep,
            )
            .await?;

            sqlx::query(
                "INSERT INTO asset_chunk_ids (build_id, name, chunk_id)
                VALUES ($1, $2, $3)",
            )
            .bind(&build.manifest.hash)
            .bind(chunk_asset.filename())
            .bind(i32::try_from(*chunk_id).expect("chunk id doesn't fit in an i32"))
            .execute(&mut transaction)
            .await?;
        }

        for (script, detected_kind) in build
            .manifest
            .assets
            .iter()
            .filter_by_type(FeAssetType::Js)
            .zip(RootScript::assumed_ordering().into_iter())
        {
            detected_asset(
                &mut transaction,
                build,
                script,
                DetectedAssetKind::SurfaceScript(detected_kind),
            )
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    /// Check whether a build hash is present in the database.
    pub async fn build_hash_is_catalogued(&self, build_hash: &str) -> Result<bool> {
        Ok(
            sqlx::query("SELECT build_id FROM detected_builds WHERE build_id = $1")
                .bind(build_hash)
                .fetch_optional(&self.pool)
                .await?
                .is_some(),
        )
    }

    /// Log an instance of a build being present on a branch, inserting the
    /// build into the database if necessary.
    pub async fn detected_build_change_on_branch(
        &self,
        build: &FeBuild,
        branch: Branch,
    ) -> Result<()> {
        let number: i32 = build
            .number
            .try_into()
            .expect("build number doesn't fit in i32");
        let hash = build.manifest.hash.clone();

        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO detected_builds (build_id, build_number)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING",
        )
        .bind(&hash)
        .bind(number)
        .execute(&mut transaction)
        .await?;

        sqlx::query(
            "INSERT INTO detected_builds_on_branches (build_id, branch)
            VALUES ($1, $2::discord_branch)",
        )
        .bind(&hash)
        .bind(branch.to_string().to_lowercase())
        .execute(&mut transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }
}
