use anyhow::Result;
use havoc::{
    discord::{AssetCache, AssetsExt, Branch, FeAsset, FeAssetType, FeBuild, RootScript},
    scrape::extract_assets_from_chunk_loader,
};
use sqlx::{sqlite::SqliteRow, Row, Sqlite};

#[derive(Clone)]
pub struct Db {
    pub pool: sqlx::SqlitePool,
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
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Fetch the last known build hash on a branch.
    pub async fn last_known_build_hash_on_branch(&self, branch: Branch) -> Result<Option<String>> {
        Ok(sqlx::query(
            "SELECT build_id
            FROM build_deploys
            WHERE branch = ?1
            ORDER BY detected_at DESC
            LIMIT 1",
        )
        .bind(branch.to_string().to_lowercase())
        .fetch_optional(&self.pool)
        .await?
        .map(|row: SqliteRow| row.get(0)))
    }

    pub async fn catalog_and_extract_assets(
        &self,
        build: &FeBuild,
        cache: &mut AssetCache,
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;

        for stylesheet in build
            .manifest
            .assets
            .iter()
            .filter_by_type(FeAssetType::Css)
        {
            insert_asset(
                &mut transaction,
                stylesheet,
                DetectedAssetKind::Surface,
                None,
            )
            .await?;
            associate_asset(&mut transaction, build, stylesheet).await?;
        }

        // TODO: Insert module IDs from surface and deep scripts.

        let chunks = extract_assets_from_chunk_loader(&build.manifest, cache).await?;
        for (chunk_id, chunk_asset) in chunks.iter() {
            insert_asset(
                &mut transaction,
                chunk_asset,
                DetectedAssetKind::Deep,
                Some(*chunk_id),
            )
            .await?;
            associate_asset(&mut transaction, &build, chunk_asset).await?;
        }

        for (script, detected_kind) in build
            .manifest
            .assets
            .iter()
            .filter_by_type(FeAssetType::Js)
            .zip(RootScript::assumed_ordering().into_iter())
        {
            insert_asset(
                &mut transaction,
                script,
                DetectedAssetKind::SurfaceScript(detected_kind),
                None,
            )
            .await?;
            associate_asset(&mut transaction, build, script).await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    /// Check whether a build hash is present in the database.
    pub async fn build_hash_is_catalogued(&self, build_hash: &str) -> Result<bool> {
        Ok(
            sqlx::query("SELECT build_id FROM builds WHERE build_id = ?1")
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

        tracing::debug!(?build.number, ?build.manifest.hash, ?branch, "inserting build");

        sqlx::query(
            "INSERT INTO builds (build_id, build_number)
            VALUES (?1, ?2)
            ON CONFLICT DO NOTHING",
        )
        .bind(&hash)
        .bind(number)
        .execute(&mut transaction)
        .await?;

        sqlx::query(
            "INSERT INTO build_deploys (build_id, branch)
            VALUES (?1, ?2)",
        )
        .bind(&hash)
        .bind(branch.to_string().to_lowercase())
        .execute(&mut transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }
}

async fn insert_asset(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    asset: &FeAsset,
    kind: DetectedAssetKind,
    chunk_id: Option<u32>,
) -> Result<()> {
    let surface_script_type = match kind {
        DetectedAssetKind::Deep | DetectedAssetKind::Surface => None,
        DetectedAssetKind::SurfaceScript(kind) => Some(format!("{:?}", kind).to_lowercase()),
    };

    tracing::debug!(
        asset = asset.filename(),
        ?kind,
        ?chunk_id,
        "inserting asset"
    );

    sqlx::query(
        "INSERT INTO assets (name, surface, surface_script_type, script_chunk_id)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT DO NOTHING",
    )
    .bind(asset.filename())
    .bind(kind.is_surface())
    .bind(surface_script_type)
    .bind(chunk_id)
    .execute(transaction)
    .await?;

    Ok(())
}

async fn associate_asset(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    build: &FeBuild,
    asset: &FeAsset,
) -> Result<()> {
    tracing::debug!(?build.number, asset = asset.filename(), "associating asset");

    sqlx::query(
        "INSERT INTO build_assets (build_id, asset_name)
        VALUES (?1, ?2)
        ON CONFLICT DO NOTHING",
    )
    .bind(&build.manifest.hash)
    .bind(asset.filename())
    .execute(transaction)
    .await?;

    Ok(())
}
