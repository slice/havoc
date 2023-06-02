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
            FROM build_deploys
            WHERE branch = $1::discord_branch
            ORDER BY detected_at DESC
            LIMIT 1",
        )
        .bind(branch.to_string().to_lowercase())
        .fetch_optional(&self.pool)
        .await?
        .map(|row: PgRow| row.get(0)))
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
            let mut c = Cataloger::new(stylesheet);
            c.insert(&mut transaction).await?;
            c.associate(&mut transaction, build).await?;
        }

        let chunks = extract_assets_from_chunk_loader(&build.manifest, cache).await?;
        for (chunk_id, chunk_asset) in chunks.iter() {
            let chunk_id = (*chunk_id)
                .try_into()
                .expect("chunk id couldn't fit into i32");

            let mut c = Cataloger::new(chunk_asset)
                .kind(DetectedAssetKind::Deep)
                .chunk_id(Some(chunk_id));
            c.insert(&mut transaction).await?;
            c.associate(&mut transaction, build).await?;
        }

        for (script, detected_kind) in build
            .manifest
            .assets
            .iter()
            .filter_by_type(FeAssetType::Js)
            .zip(RootScript::assumed_ordering().into_iter())
        {
            let mut c =
                Cataloger::new(script).kind(DetectedAssetKind::SurfaceScript(detected_kind));
            c.insert(&mut transaction).await?;
            c.associate(&mut transaction, build).await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    /// Check whether a build hash is present in the database.
    pub async fn build_hash_is_catalogued(&self, build_hash: &str) -> Result<bool> {
        Ok(
            sqlx::query("SELECT build_id FROM builds WHERE build_id = $1")
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
            "INSERT INTO builds (build_id, build_number)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING",
        )
        .bind(&hash)
        .bind(number)
        .execute(&mut transaction)
        .await?;

        sqlx::query(
            "INSERT INTO build_deploys (build_id, branch)
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

struct Cataloger<'a> {
    asset: &'a FeAsset,
    chunk_id: Option<i32>,
    kind: DetectedAssetKind,
}

impl<'a> Cataloger<'a> {
    fn new(asset: &'a FeAsset) -> Self {
        Self {
            asset,
            chunk_id: None,
            kind: DetectedAssetKind::Surface,
        }
    }

    fn kind(mut self, kind: DetectedAssetKind) -> Self {
        self.kind = kind;
        self
    }

    fn chunk_id(mut self, chunk_id: Option<i32>) -> Self {
        self.chunk_id = chunk_id;
        self
    }

    async fn insert(&mut self, transaction: &mut sqlx::Transaction<'_, Postgres>) -> Result<()> {
        let surface_script_type = match self.kind {
            DetectedAssetKind::Deep | DetectedAssetKind::Surface => "NULL".to_owned(),
            DetectedAssetKind::SurfaceScript(kind) => {
                format!("'{:?}'", kind).to_lowercase() + "::surface_script_type"
            }
        };

        sqlx::query(&format!(
            "INSERT INTO assets (name, surface, surface_script_type, script_chunk_id)
            VALUES ($1, $2, {determined_surface_script_type}, $3)
            ON CONFLICT DO NOTHING",
            determined_surface_script_type = surface_script_type
        ))
        .bind(self.asset.filename())
        .bind(self.kind.is_surface())
        .bind(self.chunk_id)
        .execute(transaction)
        .await?;

        Ok(())
    }

    async fn associate(
        &mut self,
        transaction: &mut sqlx::Transaction<'_, Postgres>,
        build: &FeBuild,
    ) -> Result<()> {
        tracing::debug!(?build.number, asset = self.asset.filename(), "associating asset");

        sqlx::query(
            "INSERT INTO build_assets (build_id, asset_name)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING",
        )
        .bind(&build.manifest.hash)
        .bind(self.asset.filename())
        .execute(transaction)
        .await?;

        Ok(())
    }
}
