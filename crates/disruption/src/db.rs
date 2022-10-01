use anyhow::Result;
use havoc::discord::Branch;
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

    pub async fn last_known_build_on_branch(&self, branch: Branch) -> Result<Option<u32>> {
        self.call(move |conn| {
            conn.query_row(
                "SELECT build_number
                FROM detected_builds_on_branches
                WHERE branch = ?
                ORDER BY detected_at DESC
                LIMIT 1",
                [branch.to_string()],
                |row| row.get::<_, u32>(0),
            )
            .ok()
        })
        .await
    }

    pub async fn detected_build_change_on_branch(
        &self,
        build_number: u32,
        branch: Branch,
    ) -> Result<()> {
        self.call(move |conn| {
            conn.execute(
                "INSERT INTO detected_builds_on_branches (build_number, branch, detected_at)
                VALUES (?, ?, ?)",
                (build_number, branch.to_string(), sqlite_now()),
            )
        })
        .await??;

        Ok(())
    }
}
