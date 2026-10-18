use sqlx::migrate::{Migration, MigrationType, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{ConnectOptions, Connection, SqlSafeStr, SqliteConnection};
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Database(PathBuf);

impl Database {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/probes")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn options(&self) -> SqliteConnectOptions {
        SqliteConnectOptions::new()
            .filename(self.0.join("qualification.db"))
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(150))
            .command_buffer_size(8)
            .row_buffer_size(8)
            .disable_statement_logging()
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn run(test: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(15), test)
                .await
                .unwrap()
        });
}

fn database_code(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned())
}

async fn scalar(connection: &mut SqliteConnection, query: &'static str) -> i64 {
    sqlx::query_scalar(query)
        .fetch_one(connection)
        .await
        .unwrap()
}

#[test]
fn native_version_wal_foreign_keys_and_extension_boundary() {
    let db = Database::new();
    run(async {
        let mut c = db.options().connect().await.unwrap();
        let version: String = sqlx::query_scalar("SELECT sqlite_version()")
            .fetch_one(&mut c)
            .await
            .unwrap();
        assert_eq!(version, "3.53.2");
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&mut c)
            .await
            .unwrap();
        assert_eq!(mode, "wal");
        assert_eq!(scalar(&mut c, "PRAGMA foreign_keys").await, 1);
        let extension = sqlx::query("SELECT load_extension(?)")
            .bind("synthetic-unavailable-extension")
            .execute(&mut c)
            .await
            .unwrap_err();
        assert_eq!(database_code(&extension).as_deref(), Some("1"));
        assert!(
            extension
                .as_database_error()
                .unwrap()
                .message()
                .contains("not authorized")
        );
        c.close().await.unwrap();
    });
}

#[test]
fn bound_parameters_and_constraint_failures_preserve_data() {
    let db = Database::new();
    run(async {
        let mut c = db.options().connect().await.unwrap();
        sqlx::raw_sql("CREATE TABLE parent (id INTEGER PRIMARY KEY); CREATE TABLE child (parent INTEGER REFERENCES parent(id), value TEXT UNIQUE);").execute(&mut c).await.unwrap();
        sqlx::query("INSERT INTO parent VALUES (1)")
            .execute(&mut c)
            .await
            .unwrap();
        let canary = "'); DROP TABLE parent; --";
        sqlx::query("INSERT INTO child VALUES (?, ?)")
            .bind(1_i64)
            .bind(canary)
            .execute(&mut c)
            .await
            .unwrap();
        let stored: String = sqlx::query_scalar("SELECT value FROM child")
            .fetch_one(&mut c)
            .await
            .unwrap();
        assert_eq!(stored, canary);
        let fk = sqlx::query("INSERT INTO child VALUES (2, 'unrelated')")
            .execute(&mut c)
            .await
            .unwrap_err();
        assert_eq!(database_code(&fk).as_deref(), Some("787"));
        let unique = sqlx::query("INSERT INTO child VALUES (1, ?)")
            .bind(canary)
            .execute(&mut c)
            .await
            .unwrap_err();
        assert_eq!(database_code(&unique).as_deref(), Some("2067"));
        assert_eq!(scalar(&mut c, "SELECT count(*) FROM parent").await, 1);
        assert_eq!(scalar(&mut c, "SELECT count(*) FROM child").await, 1);
        c.close().await.unwrap();
    });
}

fn migration(version: i64, sql: &'static str) -> Migration {
    Migration::new(
        version,
        "qualification".into(),
        MigrationType::Simple,
        sql.into_sql_str(),
        false,
    )
}

#[test]
fn fresh_upgrade_failed_migration_and_checksum_mismatch() {
    let db = Database::new();
    run(async {
        let mut c = db.options().connect().await.unwrap();
        let first = || migration(1, "CREATE TABLE records (id INTEGER PRIMARY KEY);");
        Migrator::with_migrations(vec![first()])
            .run(&mut c)
            .await
            .unwrap();
        let second = || migration(2, "ALTER TABLE records ADD COLUMN label TEXT;");
        Migrator::with_migrations(vec![first(), second()])
            .run(&mut c)
            .await
            .unwrap();
        sqlx::query("INSERT INTO records VALUES (1, 'preserved')")
            .execute(&mut c)
            .await
            .unwrap();
        let broken = migration(
            3,
            "CREATE TABLE incomplete (id INTEGER); INSERT INTO missing_table VALUES (1);",
        );
        assert!(
            Migrator::with_migrations(vec![first(), second(), broken])
                .run(&mut c)
                .await
                .is_err()
        );
        assert_eq!(
            scalar(
                &mut c,
                "SELECT count(*) FROM sqlite_master WHERE name = 'incomplete'"
            )
            .await,
            0
        );
        assert_eq!(
            scalar(&mut c, "SELECT max(version) FROM _sqlx_migrations").await,
            2
        );
        let changed = migration(1, "CREATE TABLE records (id TEXT PRIMARY KEY);");
        let mismatch = Migrator::with_migrations(vec![changed, second()])
            .run(&mut c)
            .await
            .unwrap_err();
        assert!(matches!(
            mismatch,
            sqlx::migrate::MigrateError::VersionMismatch(1)
        ));
        assert_eq!(scalar(&mut c, "SELECT count(*) FROM records").await, 1);
        c.close().await.unwrap();
    });
}

#[test]
fn wal_reader_snapshot_and_dropped_transaction_rollback() {
    let db = Database::new();
    run(async {
        let mut writer = db.options().connect().await.unwrap();
        sqlx::raw_sql("CREATE TABLE records (value INTEGER); INSERT INTO records VALUES (1);")
            .execute(&mut writer)
            .await
            .unwrap();
        let mut reader = db.options().connect().await.unwrap();
        let mut snapshot = reader.begin().await.unwrap();
        assert_eq!(scalar(&mut snapshot, "SELECT value FROM records").await, 1);
        sqlx::query("UPDATE records SET value = 2")
            .execute(&mut writer)
            .await
            .unwrap();
        assert_eq!(scalar(&mut snapshot, "SELECT value FROM records").await, 1);
        snapshot.rollback().await.unwrap();
        assert_eq!(scalar(&mut reader, "SELECT value FROM records").await, 2);
        let mut abandoned = writer.begin().await.unwrap();
        sqlx::query("UPDATE records SET value = 3")
            .execute(&mut *abandoned)
            .await
            .unwrap();
        drop(abandoned);
        assert_eq!(scalar(&mut writer, "SELECT value FROM records").await, 2);
        reader.close().await.unwrap();
        writer.close().await.unwrap();
    });
}

#[test]
fn busy_writer_is_bounded_and_recovers_after_rollback() {
    let db = Database::new();
    run(async {
        let mut first = db.options().connect().await.unwrap();
        sqlx::raw_sql("CREATE TABLE records (value INTEGER); INSERT INTO records VALUES (1);")
            .execute(&mut first)
            .await
            .unwrap();
        let mut second = db.options().connect().await.unwrap();
        let mut held = first.begin_with("BEGIN IMMEDIATE").await.unwrap();
        sqlx::query("UPDATE records SET value = 2")
            .execute(&mut *held)
            .await
            .unwrap();
        let started = Instant::now();
        let error = sqlx::query("UPDATE records SET value = 3")
            .execute(&mut second)
            .await
            .unwrap_err();
        assert_eq!(database_code(&error).as_deref(), Some("5"));
        assert!(started.elapsed() < Duration::from_secs(3));
        assert_eq!(scalar(&mut second, "SELECT value FROM records").await, 1);
        held.rollback().await.unwrap();
        sqlx::query("UPDATE records SET value = 4")
            .execute(&mut second)
            .await
            .unwrap();
        assert_eq!(scalar(&mut first, "SELECT value FROM records").await, 4);
        second.close().await.unwrap();
        first.close().await.unwrap();
    });
}

#[test]
fn sqlite_full_rolls_back_and_reopens_consistently() {
    let db = Database::new();
    run(async {
        let mut c = db.options().connect().await.unwrap();
        sqlx::raw_sql("CREATE TABLE records (payload BLOB); INSERT INTO records VALUES (x'01'); PRAGMA max_page_count=8;").execute(&mut c).await.unwrap();
        let mut tx = c.begin().await.unwrap();
        let error = sqlx::query("INSERT INTO records VALUES (zeroblob(131072))")
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(database_code(&error).as_deref(), Some("13"));
        drop(tx);
        assert_eq!(scalar(&mut c, "SELECT count(*) FROM records").await, 1);
        c.close().await.unwrap();
        let mut reopened = db
            .options()
            .create_if_missing(false)
            .connect()
            .await
            .unwrap();
        assert_eq!(
            scalar(&mut reopened, "SELECT sum(length(payload)) FROM records").await,
            1
        );
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&mut reopened)
            .await
            .unwrap();
        assert_eq!(integrity, "ok");
        reopened.close().await.unwrap();
    });
}

#[test]
fn pool_deadlines_cancelled_transaction_and_readonly_boundary() {
    let db = Database::new();
    run(async {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            // This also covers cold SQLite worker/WAL startup on loaded CI hosts.
            // Keep the saturated-pool timeout and cancellation assertions below.
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(db.options())
            .await
            .unwrap();
        sqlx::query("CREATE TABLE records (id INTEGER)")
            .execute(&pool)
            .await
            .unwrap();
        let held = pool.acquire().await.unwrap();
        assert!(matches!(
            pool.acquire().await,
            Err(sqlx::Error::PoolTimedOut)
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(10), pool.acquire())
                .await
                .is_err()
        );
        drop(held);
        let (ready, wait) = tokio::sync::oneshot::channel();
        let owned = pool.clone();
        let task = tokio::spawn(async move {
            let mut tx = owned.begin().await.unwrap();
            sqlx::query("INSERT INTO records VALUES (1)")
                .execute(&mut *tx)
                .await
                .unwrap();
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
            tx.commit().await.unwrap();
        });
        wait.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM records")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(pool.size(), 1);
        pool.close().await;
        let mut readonly = db
            .options()
            .create_if_missing(false)
            .read_only(true)
            .connect()
            .await
            .unwrap();
        let error = sqlx::query("INSERT INTO records VALUES (2)")
            .execute(&mut readonly)
            .await
            .unwrap_err();
        assert_eq!(database_code(&error).as_deref(), Some("8"));
        readonly.close().await.unwrap();
    });
}
