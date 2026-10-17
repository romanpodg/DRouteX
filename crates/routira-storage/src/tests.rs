use super::*;
use routira_config::{InputFormat, ReferenceCatalog};
use std::{error::Error, future::Future, str::FromStr};

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
