//! Bounded control-plane SQLite persistence, independent of DNS/proxy packet paths.
//!
//! Revisions preserve the complete checked source document and transactional resource
//! indexes. Reading revalidates against current trusted references; saving never
//! publishes/activates policy. Only encrypted credential envelopes enter this API;
//! plaintext and keys belong to the separate security boundary.
//! Linux private-directory enforcement targets the accepted first MVP server.
//! Other platforms compile and qualify SQLx, but `Storage::open` fails closed until
//! their filesystem permission model is implemented and reviewed.

mod database;
mod error;
mod migrations;
mod model;
mod paths;
mod secrets;

pub use error::{StorageError, StorageErrorCode};
pub use model::{CatalogRevision, ConfigurationWrite, CredentialVerifierLink, StoredConfiguration};

use routira_common::ConfigurationRevisionId;
use routira_common::SecretRefId;
use routira_config::DraftRevision;
use routira_security::EncryptedSecret;
use sqlx::{Connection, SqliteConnection};
use std::{fs::File, future::Future, path::PathBuf, time::Duration};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::{Instant, timeout_at},
};

/// Owner-approved maximum pending commands; an executing operation is separate.
pub const QUEUE_CAPACITY: usize = 64;
/// Response deadline, not a promise that COMMIT was cancelled.
pub const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

type Reply<T> = oneshot::Sender<Result<T, StorageError>>;

enum Operation {
    Save {
        write: Box<ConfigurationWrite>,
        revision: DraftRevision,
        reply: Reply<()>,
    },
    DraftHead {
        reply: Reply<Option<ConfigurationRevisionId>>,
    },
    InsertSecret {
        record: EncryptedSecret,
        reply: Reply<()>,
    },
    ReplaceSecret {
        expected: EncryptedSecret,
        replacement: EncryptedSecret,
        reply: Reply<()>,
    },
    LoadSecret {
        id: SecretRefId,
        reply: Reply<Option<EncryptedSecret>>,
    },
    Load {
        id: ConfigurationRevisionId,
        reply: Reply<Option<StoredConfiguration>>,
    },
    RegisterCatalog {
        revision: CatalogRevision,
        sha256: [u8; 32],
        reply: Reply<()>,
    },
    CatalogDigest {
        revision: CatalogRevision,
        reply: Reply<Option<[u8; 32]>>,
    },
    Close {
        reply: Reply<()>,
    },
}

struct Command {
    deadline: Instant,
    operation: Operation,
}

impl Operation {
    fn abandoned(&self) -> bool {
        match self {
            Self::Save { reply, .. }
            | Self::InsertSecret { reply, .. }
            | Self::ReplaceSecret { reply, .. }
            | Self::RegisterCatalog { reply, .. }
            | Self::Close { reply } => reply.is_closed(),
            Self::Load { reply, .. } => reply.is_closed(),
            Self::DraftHead { reply } => reply.is_closed(),
            Self::CatalogDigest { reply, .. } => reply.is_closed(),
            Self::LoadSecret { reply, .. } => reply.is_closed(),
        }
    }

    fn reject(self, code: StorageErrorCode) {
        let error = StorageError::new(code);
        match self {
            Self::Save { reply, .. }
            | Self::InsertSecret { reply, .. }
            | Self::ReplaceSecret { reply, .. }
            | Self::RegisterCatalog { reply, .. }
            | Self::Close { reply } => {
                let _ = reply.send(Err(error));
            }
            Self::Load { reply, .. } => {
                let _ = reply.send(Err(error));
            }
            Self::DraftHead { reply } => {
                let _ = reply.send(Err(error));
            }
            Self::CatalogDigest { reply, .. } => {
                let _ = reply.send(Err(error));
            }
            Self::LoadSecret { reply, .. } => {
                let _ = reply.send(Err(error));
            }
        }
    }
}

/// Cloneable sender to one owner task/connection, never a database handle.
#[derive(Clone)]
pub struct Repository {
    sender: mpsc::Sender<Command>,
}

impl Repository {
    /// Persist a new opaque encrypted record. Logical updates require a new reference.
    /// Ciphertext is structurally validated; this boundary never decrypts/authenticates it.
    pub async fn insert_secret(&self, record: EncryptedSecret) -> Result<(), StorageError> {
        self.request(|reply| Operation::InsertSecret { record, reply })
            .await
    }

    /// Re-encrypt under the same reference/purpose only if the exact old record still exists.
    /// An in-flight replacement can commit after timeout/drop; reread its reference.
    pub async fn replace_secret(
        &self,
        expected: EncryptedSecret,
        replacement: EncryptedSecret,
    ) -> Result<(), StorageError> {
        if expected.context() != replacement.context() {
            return Err(StorageError::new(StorageErrorCode::InvalidSecret));
        }
        self.request(|reply| Operation::ReplaceSecret {
            expected,
            replacement,
            reply,
        })
        .await
    }

    pub async fn load_secret(
        &self,
        id: SecretRefId,
    ) -> Result<Option<EncryptedSecret>, StorageError> {
        self.request(|reply| Operation::LoadSecret { id, reply })
            .await
    }

    async fn request<T>(
        &self,
        build: impl FnOnce(Reply<T>) -> Operation,
    ) -> Result<T, StorageError> {
        tokio::runtime::Handle::try_current()
            .map_err(|_| StorageError::new(StorageErrorCode::RuntimeUnavailable))?;
        let deadline = Instant::now() + RESPONSE_TIMEOUT;
        let (reply, response) = oneshot::channel();
        self.sender
            .try_send(Command {
                deadline,
                operation: build(reply),
            })
            .map_err(|error| {
                StorageError::new(match error {
                    mpsc::error::TrySendError::Full(_) => StorageErrorCode::QueueFull,
                    mpsc::error::TrySendError::Closed(_) => StorageErrorCode::Closed,
                })
            })?;
        timeout_at(deadline, response)
            .await
            .map_err(|_| StorageError::new(StorageErrorCode::DeadlineOutcomeUnknown))?
            .map_err(|_| StorageError::new(StorageErrorCode::Closed))?
    }

    /// Append a complete revision and its indexes atomically. Existing IDs conflict.
    /// Trusted provenance is mandatory; its ID must match the document revision,
    /// and its parent must equal the current global draft head (None for a root).
    /// Stale edits fail Conflict; neither audit nor draft head advances on failure.
    /// On timeout/drop, an in-flight transaction may still commit; reread this ID.
    pub async fn save_configuration(
        &self,
        write: ConfigurationWrite,
        revision: DraftRevision,
    ) -> Result<(), StorageError> {
        self.request(|reply| Operation::Save {
            write: Box::new(write),
            revision,
            reply,
        })
        .await
    }

    /// Current draft only; never an active/published policy or runtime authority.
    pub async fn draft_head(&self) -> Result<Option<ConfigurationRevisionId>, StorageError> {
        self.request(|reply| Operation::DraftHead { reply }).await
    }

    pub async fn load_configuration(
        &self,
        id: ConfigurationRevisionId,
    ) -> Result<Option<StoredConfiguration>, StorageError> {
        self.request(|reply| Operation::Load { id, reply }).await
    }

    /// Persist an immutable claimed hash, not a verified/approved dataset payload.
    pub async fn register_catalog(
        &self,
        revision: CatalogRevision,
        sha256: [u8; 32],
    ) -> Result<(), StorageError> {
        self.request(|reply| Operation::RegisterCatalog {
            revision,
            sha256,
            reply,
        })
        .await
    }

    pub async fn catalog_digest(
        &self,
        revision: CatalogRevision,
    ) -> Result<Option<[u8; 32]>, StorageError> {
        self.request(|reply| Operation::CatalogDigest { revision, reply })
            .await
    }
}

/// Lifecycle owner. Call `close` to drain prior commands and close SQLite cleanly.
/// Dropping without `close` aborts the owner task; it does not promise rollback of COMMIT.
pub struct Storage {
    repository: Repository,
    task: Option<JoinHandle<()>>,
}

impl Storage {
    /// Requires a caller-owned Tokio runtime with time enabled and a trusted local Linux parent directory.
    /// Creates only the final 0700 directory and 0600 files; rejects unsafe existing
    /// modes, symlinks/hardlinks and another storage owner, without changing access.
    /// The service must run unprivileged; malicious same-UID filesystem mutation is
    /// outside this directory boundary. Secret records contain encrypted envelopes only.
    /// Timeout/cancellation stops waiting, not initialization: its task retains the
    /// lock, finishes the upgrade and closes if the result is abandoned. Keep the
    /// caller-owned runtime alive; reopening meanwhile returns Busy.
    pub async fn open(directory: PathBuf) -> Result<Self, StorageError> {
        Self::open_with_initialization(async move {
            let paths = tokio::task::spawn_blocking(move || paths::prepare(directory))
                .await
                .map_err(|_| StorageError::new(StorageErrorCode::Filesystem))??;
            let connection = migrations::initialize(&paths).await?;
            Ok((connection, paths.lock))
        })
        .await
    }

    async fn open_with_initialization(
        initialize: impl Future<Output = Result<(SqliteConnection, File), StorageError>>
        + Send
        + 'static,
    ) -> Result<Self, StorageError> {
        tokio::runtime::Handle::try_current()
            .map_err(|_| StorageError::new(StorageErrorCode::RuntimeUnavailable))?;
        let deadline = Instant::now() + RESPONSE_TIMEOUT;
        let (sender, receiver) = mpsc::channel(QUEUE_CAPACITY);
        let (ready, response) = oneshot::channel();
        // The task owns initialization even if its caller stops waiting mid-backup.
        let task = tokio::spawn(async move {
            let (connection, lock) = match initialize.await {
                Ok(initialized) => initialized,
                Err(error) => {
                    let _ = ready.send(Err(error));
                    return;
                }
            };
            if ready.send(Ok(())).is_ok() {
                writer(connection, receiver, lock).await;
            } else {
                let _ = connection.close().await;
                drop(lock);
            }
        });
        timeout_at(deadline, response)
            .await
            .map_err(|_| StorageError::new(StorageErrorCode::DeadlineOutcomeUnknown))?
            .map_err(|_| StorageError::new(StorageErrorCode::Unavailable))??;
        Ok(Self {
            repository: Repository { sender },
            task: Some(task),
        })
    }

    pub fn repository(&self) -> Repository {
        self.repository.clone()
    }

    pub async fn close(mut self) -> Result<(), StorageError> {
        tokio::runtime::Handle::try_current()
            .map_err(|_| StorageError::new(StorageErrorCode::RuntimeUnavailable))?;
        let deadline = Instant::now() + RESPONSE_TIMEOUT;
        let (reply, response) = oneshot::channel();
        let send = self.repository.sender.send(Command {
            deadline,
            operation: Operation::Close { reply },
        });
        timeout_at(deadline, send)
            .await
            .map_err(|_| StorageError::new(StorageErrorCode::DeadlineOutcomeUnknown))?
            .map_err(|_| StorageError::new(StorageErrorCode::Closed))?;
        let result = timeout_at(deadline, response)
            .await
            .map_err(|_| StorageError::new(StorageErrorCode::DeadlineOutcomeUnknown))?
            .map_err(|_| StorageError::new(StorageErrorCode::Closed))?;
        if let Some(task) = self.task.take() {
            task.await
                .map_err(|_| StorageError::new(StorageErrorCode::Unavailable))?;
        }
        result
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

async fn writer(
    mut connection: SqliteConnection,
    mut commands: mpsc::Receiver<Command>,
    _lock: File,
) {
    while let Some(command) = commands.recv().await {
        if let Operation::Close { reply } = command.operation {
            commands.close();
            let result = connection.close().await.map_err(error::database);
            let _ = reply.send(result);
            while let Some(queued) = commands.recv().await {
                queued.operation.reject(StorageErrorCode::Closed);
            }
            return;
        }
        if command.deadline <= Instant::now() || command.operation.abandoned() {
            command
                .operation
                .reject(StorageErrorCode::DeadlineOutcomeUnknown);
            continue;
        }
        let reusable = match command.operation {
            Operation::InsertSecret { record, reply } => {
                let result = secrets::insert(&mut connection, record).await;
                finish_operation(&mut connection, result, reply).await
            }
            Operation::ReplaceSecret {
                expected,
                replacement,
                reply,
            } => {
                let result = secrets::replace(&mut connection, expected, replacement).await;
                finish_operation(&mut connection, result, reply).await
            }
            Operation::LoadSecret { id, reply } => {
                let result = secrets::load(&mut connection, id).await;
                finish_operation(&mut connection, result, reply).await
            }
            Operation::Save {
                write,
                revision,
                reply,
            } => {
                let result = database::save(&mut connection, *write, revision).await;
                finish_operation(&mut connection, result, reply).await
            }
            Operation::DraftHead { reply } => {
                let result = database::draft_head(&mut connection).await;
                finish_operation(&mut connection, result, reply).await
            }
            Operation::Load { id, reply } => {
                let result = database::load(&mut connection, id).await;
                finish_operation(&mut connection, result, reply).await
            }
            Operation::RegisterCatalog {
                revision,
                sha256,
                reply,
            } => {
                let result = database::register_catalog(&mut connection, revision, sha256).await;
                finish_operation(&mut connection, result, reply).await
            }
            Operation::CatalogDigest { revision, reply } => {
                let result = database::catalog_digest(&mut connection, revision).await;
                finish_operation(&mut connection, result, reply).await
            }
            Operation::Close { .. } => false,
        };
        if !reusable {
            commands.close();
            break;
        }
    }
    let _ = connection.close().await;
    while let Some(queued) = commands.recv().await {
        queued.operation.reject(StorageErrorCode::Closed);
    }
}

async fn finish_operation<T>(
    connection: &mut SqliteConnection,
    result: Result<T, StorageError>,
    reply: Reply<T>,
) -> bool {
    // Drain queued rollback before inspecting SQLx depth: SQLite may have rolled back automatically.
    let reusable = connection.ping().await.is_ok() && !connection.is_in_transaction();
    let result = if reusable || result.is_err() {
        result
    } else {
        Err(StorageError::new(StorageErrorCode::Unavailable))
    };
    let _ = reply.send(result);
    reusable
}

#[cfg(test)]
mod tests;
