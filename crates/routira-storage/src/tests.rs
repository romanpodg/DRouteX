use super::*;
use routira_config::{InputFormat, ReferenceCatalog, RevisionProvenance};
use std::{error::Error, future::Future, str::FromStr};

mod secrets;

fn typed<T: FromStr>(number: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{number:012x}")
        .parse()
        .unwrap()
}

fn catalog() -> ReferenceCatalog {
    ReferenceCatalog {
        devices: [typed(120)].into(),
        upstreams: [typed(90)].into(),
        secrets: [typed(99)].into(),
        filter_revisions: [typed(101)].into(),
        service_revisions: [typed(100)].into(),
    }
}

fn run(test: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(20), test)
                .await
                .unwrap();
        });
}

fn empty_write(number: usize) -> ConfigurationWrite {
    ConfigurationWrite::parse(
        typed(number),
        b"{\"schemaVersion\":1}".to_vec(),
        InputFormat::Json,
        &ReferenceCatalog::default(),
        vec![],
    )
    .unwrap()
}

const FIXTURE: &[u8] = include_bytes!("../../routira-config/tests/fixtures/configuration-v1.json");

fn revision(number: usize, parent: Option<usize>) -> DraftRevision {
    DraftRevision::from_unix_millis(
        typed(number),
        parent.map(typed),
        typed(700),
        1234,
        typed(800),
    )
    .unwrap()
}

async fn memory_storage() -> Storage {
    use sqlx::ConnectOptions;
    let mut connection = sqlx::sqlite::SqliteConnectOptions::new()
        .in_memory(true)
        .disable_statement_logging()
        .connect()
        .await
        .unwrap();
    migrations::migrations()
        .run_direct(None, &mut connection, false)
        .await
        .unwrap();
    let (sender, receiver) = mpsc::channel(QUEUE_CAPACITY);
    let lock = File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    Storage {
        repository: Repository { sender },
        task: Some(tokio::spawn(writer(connection, receiver, lock))),
    }
}

#[test]
fn concurrent_drafts_have_one_winner_and_preserve_recorded_provenance() {
    run(async {
        let storage = memory_storage().await;
        let first = storage.repository();
        let second = first.clone();
        assert_eq!(first.draft_head().await.unwrap(), None);
        first
            .save_configuration(empty_write(500), revision(500, None))
            .await
            .unwrap();
        let a_repository = first.clone();
        let a = tokio::spawn(async move {
            a_repository
                .save_configuration(empty_write(501), revision(501, Some(500)))
                .await
        });
        let b = tokio::spawn(async move {
            second
                .save_configuration(empty_write(502), revision(502, Some(500)))
                .await
        });
        let (a, b) = (a.await.unwrap(), b.await.unwrap());
        let winner = match (a, b) {
            (Ok(()), Err(error)) if error.code() == StorageErrorCode::Conflict => 501,
            (Err(error), Ok(())) if error.code() == StorageErrorCode::Conflict => 502,
            other => panic!("concurrent edits did not produce one conflict: {other:?}"),
        };
        assert_eq!(first.draft_head().await.unwrap(), Some(typed(winner)));
        let stored = first
            .load_configuration(typed(winner))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.id(), typed(winner));
        assert_eq!(
            stored.provenance(),
            RevisionProvenance::Recorded(revision(winner, Some(500)))
        );
        let loser = if winner == 501 { 502 } else { 501 };
        assert!(
            first
                .load_configuration(typed(loser))
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            first
                .save_configuration(empty_write(503), revision(504, Some(winner)))
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::InvalidProvenance
        );
        assert_eq!(first.draft_head().await.unwrap(), Some(typed(winner)));
        storage.close().await.unwrap();
    });
}

#[test]
fn opening_finishes_initialization_after_caller_timeout_or_cancellation() {
    for cancel in [false, true] {
        run(async {
            let (started, started_rx) = oneshot::channel();
            let (release, release_rx) = oneshot::channel();
            let (completed, completed_rx) = oneshot::channel();
            let opening = tokio::spawn(Storage::open_with_initialization(async move {
                started.send(()).unwrap();
                release_rx.await.unwrap();
                completed.send(()).unwrap();
                Err(StorageError::new(StorageErrorCode::Unavailable))
            }));
            started_rx.await.unwrap();
            if cancel {
                opening.abort();
                assert!(matches!(opening.await, Err(error) if error.is_cancelled()));
            } else {
                assert_eq!(
                    opening.await.unwrap().err().unwrap().code(),
                    StorageErrorCode::DeadlineOutcomeUnknown
                );
            }
            assert!(
                release.send(()).is_ok(),
                "caller timeout/cancellation dropped unfinished initialization (cancel={cancel})"
            );
            completed_rx.await.unwrap();
        });
    }
}

#[test]
fn automatic_rollback_closes_writer_before_a_following_write_is_acknowledged() {
    run(async {
        use sqlx::ConnectOptions;
        let mut connection = sqlx::sqlite::SqliteConnectOptions::new()
            .in_memory(true)
            .disable_statement_logging()
            .connect()
            .await
            .unwrap();
        migrations::migrations().run(&mut connection).await.unwrap();
        database::save(&mut connection, empty_write(500), revision(500, None))
            .await
            .unwrap();
        sqlx::query("PRAGMA max_page_count=1")
            .execute(&mut connection)
            .await
            .unwrap();
        let mut input = b"{\"schemaVersion\":1}".to_vec();
        input.extend(std::iter::repeat_n(b' ', 200_000));
        let write = ConfigurationWrite::parse(
            typed(501),
            input,
            InputFormat::Json,
            &ReferenceCatalog::default(),
            vec![],
        )
        .unwrap();
        let (sender, receiver) = mpsc::channel(QUEUE_CAPACITY);
        // In-memory actor test; filesystem access control is tested separately on Linux.
        let lock = File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
        let task = tokio::spawn(writer(connection, receiver, lock));
        let repository = Repository { sender };
        assert_eq!(
            repository
                .save_configuration(write, revision(501, Some(500)))
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::DiskFull
        );
        assert!(
            matches!(repository.load_configuration(typed(501)).await, Err(error) if error.code() == StorageErrorCode::Closed),
            "writer reused a connection with stale transaction depth after SQLITE_FULL"
        );
        assert_eq!(
            repository
                .save_configuration(empty_write(502), revision(502, Some(500)))
                .await
                .unwrap_err()
                .code(),
            StorageErrorCode::Closed
        );
        task.await.unwrap();
    });
}

#[test]
fn input_link_validation_and_debug_errors_do_not_echo_values() {
    let canary =
        b"{\"schemaVersion\":1,\"unknown-disposable-canary\":\"disposable-private-value\"}";
    let error = ConfigurationWrite::parse(
        typed(500),
        canary.to_vec(),
        InputFormat::Json,
        &catalog(),
        vec![],
    )
    .unwrap_err();
    assert_eq!(error.code(), StorageErrorCode::InvalidConfiguration);
    assert!(error.configuration_error().is_some());
    assert!(error.source().is_none());
    assert!(!format!("{error:?} {error}").contains("disposable"));
    let mapped = error::database(sqlx::Error::Protocol(
        "disposable-private-database-value".to_owned(),
    ));
    assert!(mapped.source().is_none());
    assert!(!format!("{mapped:?} {mapped}").contains("disposable"));
    let valid = CredentialVerifierLink {
        credential: typed(20),
        reference: typed(99),
    };
    let write = ConfigurationWrite::parse(
        typed(500),
        FIXTURE.to_vec(),
        InputFormat::Json,
        &catalog(),
        vec![valid],
    )
    .unwrap();
    assert_eq!(format!("{write:?}"), "ConfigurationWrite([REDACTED])");
    for links in [
        vec![valid, valid],
        vec![CredentialVerifierLink {
            credential: typed(999),
            ..valid
        }],
        vec![CredentialVerifierLink {
            reference: typed(999),
            ..valid
        }],
    ] {
        let error = ConfigurationWrite::parse(
            typed(500),
            FIXTURE.to_vec(),
            InputFormat::Json,
            &catalog(),
            links,
        )
        .unwrap_err();
        assert_eq!(error.code(), StorageErrorCode::InvalidCredentialLink);
    }
}

#[test]
fn queue_overflow_is_immediate_and_never_retried() {
    run(async {
        let (sender, mut receiver) = mpsc::channel(QUEUE_CAPACITY);
        let repo = Repository { sender };
        let mut replies = Vec::new();
        for _ in 0..QUEUE_CAPACITY {
            let (reply, response) = oneshot::channel();
            repo.sender
                .try_send(Command {
                    deadline: Instant::now() + RESPONSE_TIMEOUT,
                    operation: Operation::Save {
                        write: Box::new(empty_write(500)),
                        revision: revision(500, None),
                        reply,
                    },
                })
                .unwrap_or_else(|_| panic!("queue unexpectedly full"));
            replies.push(response);
        }
        let error = repo.load_configuration(typed(500)).await.err().unwrap();
        assert_eq!(error.code(), StorageErrorCode::QueueFull);
        assert_eq!(receiver.len(), QUEUE_CAPACITY);
        receiver.close();
        let error = repo.load_configuration(typed(500)).await.err().unwrap();
        assert_eq!(error.code(), StorageErrorCode::Closed);
        drop(replies);
    });
}

#[test]
fn public_async_boundaries_report_missing_runtime_without_panicking() {
    use std::task::{Context, Poll, Waker};
    let (sender, _receiver) = mpsc::channel(QUEUE_CAPACITY);
    let repository = Repository { sender };
    let mut context = Context::from_waker(Waker::noop());
    let mut query = Box::pin(repository.load_configuration(typed(500)));
    let Poll::Ready(Err(error)) = query.as_mut().poll(&mut context) else {
        panic!("missing runtime must fail immediately");
    };
    assert_eq!(error.code(), StorageErrorCode::RuntimeUnavailable);
    drop(query);
    let storage = Storage {
        repository,
        task: None,
    };
    let mut close = Box::pin(storage.close());
    let Poll::Ready(Err(error)) = close.as_mut().poll(&mut context) else {
        panic!("missing runtime must fail immediately");
    };
    assert_eq!(error.code(), StorageErrorCode::RuntimeUnavailable);
    let mut open = Box::pin(Storage::open(PathBuf::from("must-not-be-created")));
    let Poll::Ready(Err(error)) = open.as_mut().poll(&mut context) else {
        panic!("missing runtime must fail immediately");
    };
    assert_eq!(error.code(), StorageErrorCode::RuntimeUnavailable);
}

#[test]
fn response_deadline_and_cancelled_receivers_do_not_promise_rollback() {
    run(async {
        let (sender, mut receiver) = mpsc::channel(QUEUE_CAPACITY);
        let repo = Repository { sender };
        let first = repo.clone();
        let task = tokio::spawn(async move { first.load_configuration(typed(500)).await });
        let command = receiver.recv().await.unwrap();
        let result = task.await.unwrap();
        assert_eq!(
            result.err().unwrap().code(),
            StorageErrorCode::DeadlineOutcomeUnknown
        );
        assert!(command.operation.abandoned());
        assert!(receiver.try_recv().is_err());
        let task = tokio::spawn(async move { repo.load_configuration(typed(500)).await });
        let command = receiver.recv().await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(command.operation.abandoned());
    });
}

#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_filesystem_permission_model_fails_before_creating_files() {
    run(async {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/unsupported-storage-must-not-exist");
        assert!(!path.exists());
        let error = Storage::open(path.clone()).await.err().unwrap();
        assert_eq!(error.code(), StorageErrorCode::UnsupportedPlatform);
        assert!(!path.exists());
    });
}

#[cfg(target_os = "linux")]
mod linux;
