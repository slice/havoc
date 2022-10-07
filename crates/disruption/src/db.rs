use anyhow::Result;
use havoc::discord::{Branch, FeBuild};
use tokio::sync::oneshot;

const DB_SCHEMA: &str = include_str!("./schema.sql");

pub struct Db {
    sender: tokio::sync::mpsc::Sender<DbCall>,
}

pub struct DbCall(Box<dyn FnOnce(&mut rusqlite::Connection) + Send + Sync>);

impl std::fmt::Debug for DbCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbCall").finish_non_exhaustive()
    }
}

fn sqlite_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("can't tell the time")
        .as_millis()
        .try_into()
        .expect("it's too far into the future")
}

fn handle_message(conn: &mut rusqlite::Connection, DbCall(call): DbCall) {
    call(conn);
}

impl Db {
    pub fn new(mut conn: rusqlite::Connection) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<DbCall>(32);

        tokio::spawn(async move {
            conn.execute_batch(DB_SCHEMA)
                .expect("failed to execute initial schema");

            conn.pragma_update(None, "foreign_keys", "ON")
                .expect("failed to enable foreign keys");

            while let Some(msg) = rx.recv().await {
                handle_message(&mut conn, msg);
            }
        });

        Self { sender: tx }
    }

    pub async fn call<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut rusqlite::Connection) -> T + Send + Sync + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        self.sender
            .send(DbCall(Box::new(|conn| {
                let _ = tx.send(f(conn));
            })))
            .await?;

        Ok(rx.await?)
    }

    /// Fetch the last known build hash on a branch.
    pub async fn last_known_build_hash_on_branch(&self, branch: Branch) -> Result<Option<String>> {
        self.call(move |conn| {
            conn.query_row(
                "SELECT build_id
                FROM detected_builds_on_branches
                WHERE branch = ?
                ORDER BY detected_at DESC
                LIMIT 1",
                [branch.to_string()],
                |row| row.get::<_, String>(0),
            )
            .ok()
        })
        .await
    }

    /// Log an instance of a build being present on a branch, inserting the
    /// build into the database if necessary.
    pub async fn detected_build_change_on_branch(
        &self,
        build: &FeBuild,
        branch: Branch,
    ) -> Result<()> {
        let number = build.number;
        let hash = build.manifest.hash.clone();

        self.call(move |conn| -> rusqlite::Result<()> {
            let tx = conn.transaction()?;

            tx.execute(
                "INSERT OR IGNORE INTO detected_builds (build_id, build_number)
                VALUES (?, ?)",
                (&hash, number),
            )?;

            tx.execute(
                "INSERT INTO detected_builds_on_branches (build_id, branch, detected_at)
                VALUES (?, ?, ?)",
                (&hash, branch.to_string(), sqlite_now()),
            )?;

            tx.commit()
        })
        .await??;

        Ok(())
    }
}
