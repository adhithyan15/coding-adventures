//! Closed renderer-to-native command facade.

use crate::{
    identity::create_document_identity,
    preview::PreviewServer,
    storage::{NativeProjectStore, StoreError, MAX_PROJECT_BYTES},
    target::{NativeTargets, TargetError, TargetReview},
    worker::{NativeProductWorker, WorkerError},
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

const MAX_BASE64_PROJECT_BYTES: usize = MAX_PROJECT_BYTES.div_ceil(3) * 4;
const SHA256_REVISION_LENGTH: usize = "sha256:".len() + 44;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectCompareAndSwapRequest {
    pub expected_revision: Option<String>,
    pub bytes_base64: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLoadResponse {
    pub bytes_base64: String,
    pub revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCompareAndSwapResponse {
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewBuildRequest {
    pub revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewBuildResponse {
    pub outcome: &'static str,
    pub revision: String,
    pub build_id: String,
    pub diagnostics: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetPublishRequest {
    pub target_id: String,
    pub revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetPublishResponse {
    pub outcome: &'static str,
    pub revision: String,
    pub manifest_sha256: Option<String>,
    pub target_id: String,
    pub diagnostics: Vec<CommandDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CommandDiagnostic {
    pub severity: &'static str,
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CommandError {
    pub code: &'static str,
}

impl CommandError {
    pub const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

#[derive(Debug)]
pub struct NativeCommands {
    store: NativeProjectStore,
    lifecycle: Arc<WorkspaceLifecycle>,
}

#[derive(Debug)]
pub struct NativePreviewCommands {
    worker: NativeProductWorker,
    server: PreviewServer,
    targets: NativeTargets,
    unavailable: AtomicBool,
    lifecycle: Arc<WorkspaceLifecycle>,
}

#[derive(Debug, Default)]
pub struct WorkspaceLifecycle {
    accepting: AtomicBool,
    storage_gate: Mutex<()>,
    product_gate: Mutex<()>,
}

impl WorkspaceLifecycle {
    fn open() -> Self {
        Self {
            accepting: AtomicBool::new(true),
            storage_gate: Mutex::new(()),
            product_gate: Mutex::new(()),
        }
    }

    fn ensure_open(&self) -> Result<(), CommandError> {
        if self.accepting.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(CommandError::new("WORKSPACE_POISONED"))
        }
    }

    fn admit_storage(&self) -> Result<MutexGuard<'_, ()>, CommandError> {
        let guard = self
            .storage_gate
            .lock()
            .map_err(|_| CommandError::new("WORKSPACE_POISONED"))?;
        self.ensure_open()?;
        Ok(guard)
    }

    fn admit_product(&self) -> Result<MutexGuard<'_, ()>, CommandError> {
        let guard = self
            .product_gate
            .lock()
            .map_err(|_| CommandError::new("WORKSPACE_POISONED"))?;
        self.ensure_open()?;
        Ok(guard)
    }
}

impl NativeCommands {
    pub fn open(profile_root: PathBuf) -> Result<Self, CommandError> {
        Ok(Self {
            store: NativeProjectStore::open(profile_root).map_err(map_store_error)?,
            lifecycle: Arc::new(WorkspaceLifecycle::open()),
        })
    }

    pub fn lifecycle(&self) -> Arc<WorkspaceLifecycle> {
        Arc::clone(&self.lifecycle)
    }

    pub fn project_load(&self) -> Result<Option<ProjectLoadResponse>, CommandError> {
        let _guard = self.lifecycle.admit_storage()?;
        self.store
            .load()
            .map(|stored| {
                stored.map(|value| ProjectLoadResponse {
                    bytes_base64: BASE64.encode(value.bytes),
                    revision: value.revision,
                })
            })
            .map_err(map_store_error)
    }

    pub fn project_compare_and_swap(
        &self,
        request: ProjectCompareAndSwapRequest,
    ) -> Result<ProjectCompareAndSwapResponse, CommandError> {
        if request.bytes_base64.is_empty()
            || request.bytes_base64.len() > MAX_BASE64_PROJECT_BYTES
            || request
                .expected_revision
                .as_deref()
                .is_some_and(|value| !valid_revision(value))
        {
            return Err(CommandError::new("INVALID_REQUEST"));
        }
        let bytes = BASE64
            .decode(request.bytes_base64.as_bytes())
            .map_err(|_| CommandError::new("INVALID_REQUEST"))?;
        if bytes.len() > MAX_PROJECT_BYTES {
            return Err(CommandError::new("INVALID_REQUEST"));
        }
        let _guard = self.lifecycle.admit_storage()?;
        let revision = self
            .store
            .compare_and_swap(request.expected_revision.as_deref(), &bytes)
            .map_err(map_store_error)?;
        Ok(ProjectCompareAndSwapResponse { revision })
    }

    pub fn identity_create(&self) -> Result<String, CommandError> {
        let _guard = self.lifecycle.admit_storage()?;
        Ok(create_document_identity())
    }

    pub fn with_current_project<T>(
        &self,
        revision: &str,
        operation: impl FnOnce(serde_json::Value) -> Result<T, CommandError>,
    ) -> Result<T, CommandError> {
        if !valid_revision(revision) {
            return Err(CommandError::new("INVALID_REQUEST"));
        }
        let _guard = self.lifecycle.admit_storage()?;
        self.store
            .with_project_at_revision(revision, operation)
            .map_err(map_store_error)?
    }
}

impl NativePreviewCommands {
    pub fn open(worker: NativeProductWorker) -> Result<Self, CommandError> {
        let server = PreviewServer::bind().map_err(|_| CommandError::new("PREVIEW_FAILED"))?;
        let targets = NativeTargets::new(Vec::new()).map_err(map_target_error)?;
        Ok(Self::open_with_server_and_targets(worker, server, targets))
    }

    pub fn open_with_server(worker: NativeProductWorker, server: PreviewServer) -> Self {
        Self::open_with_server_and_targets(
            worker,
            server,
            NativeTargets::new(Vec::new()).expect("empty target exclusions are valid"),
        )
    }

    pub fn open_with_server_and_targets(
        worker: NativeProductWorker,
        server: PreviewServer,
        targets: NativeTargets,
    ) -> Self {
        Self::open_with_server_targets_and_lifecycle(
            worker,
            server,
            targets,
            Arc::new(WorkspaceLifecycle::open()),
        )
    }

    pub fn open_with_server_targets_and_lifecycle(
        worker: NativeProductWorker,
        server: PreviewServer,
        targets: NativeTargets,
        lifecycle: Arc<WorkspaceLifecycle>,
    ) -> Self {
        Self {
            worker,
            server,
            targets,
            unavailable: AtomicBool::new(false),
            lifecycle,
        }
    }

    pub fn preview_url(&self) -> String {
        self.server.url()
    }

    pub fn preview_build(
        &self,
        project: serde_json::Value,
        request: PreviewBuildRequest,
    ) -> Result<PreviewBuildResponse, CommandError> {
        if !valid_revision(&request.revision) {
            return Err(CommandError::new("INVALID_REQUEST"));
        }
        let _guard = self.lifecycle.admit_product()?;
        self.ensure_available()?;
        let run = self
            .worker
            .build(&project, &request.revision)
            .map_err(|error| {
                if error != WorkerError::Build {
                    self.unavailable.store(true, Ordering::Release);
                }
                map_worker_error(error)
            })?;
        let revision = run.output().revision.clone();
        let build_id = run.output().build_id.clone();
        self.server.publish_run(run).map_err(|error| {
            self.unavailable.store(true, Ordering::Release);
            map_worker_error(error)
        })?;
        Ok(PreviewBuildResponse {
            outcome: "ready",
            revision,
            build_id,
            diagnostics: Vec::new(),
        })
    }

    pub fn target_configure_path(&self, path: PathBuf) -> Result<TargetReview, CommandError> {
        let _guard = self.lifecycle.admit_product()?;
        self.ensure_available()?;
        self.targets.configure_path(path).map_err(map_target_error)
    }

    pub fn target_list(&self) -> Result<Vec<TargetReview>, CommandError> {
        let _guard = self.lifecycle.admit_product()?;
        self.ensure_available()?;
        self.targets.list().map_err(map_target_error)
    }

    pub fn target_publish(
        &self,
        project: serde_json::Value,
        request: TargetPublishRequest,
    ) -> Result<TargetPublishResponse, CommandError> {
        if !valid_revision(&request.revision) {
            return Err(CommandError::new("INVALID_REQUEST"));
        }
        let _guard = self.lifecycle.admit_product()?;
        self.ensure_available()?;
        let run = match self.worker.build(&project, &request.revision) {
            Ok(run) => run,
            Err(error @ (WorkerError::Build | WorkerError::Timeout)) => {
                if error == WorkerError::Timeout {
                    self.unavailable.store(true, Ordering::Release);
                }
                return Ok(publish_failure(&request, "failed", "E_BUILD_FAILED"));
            }
            Err(error) => {
                self.unavailable.store(true, Ordering::Release);
                return Err(map_worker_error(error));
            }
        };
        let revision = run.output().revision.clone();
        let receipt = match self.targets.publish(&request.target_id, &run) {
            Ok(receipt) => receipt,
            Err(TargetError::Failed | TargetError::Unsafe) => {
                if run.retire().is_err() {
                    self.unavailable.store(true, Ordering::Release);
                    return Ok(publish_failure(
                        &request,
                        "indeterminate",
                        "E_PUBLISH_RECONCILE",
                    ));
                }
                return Ok(publish_failure(&request, "failed", "E_PUBLISH_FAILED"));
            }
            Err(TargetError::Indeterminate) => {
                self.unavailable.store(true, Ordering::Release);
                let _ = run.retire();
                return Ok(publish_failure(
                    &request,
                    "indeterminate",
                    "E_PUBLISH_RECONCILE",
                ));
            }
            Err(TargetError::Invalid) => {
                if run.retire().is_err() {
                    self.unavailable.store(true, Ordering::Release);
                    return Err(CommandError::new("WORKSPACE_POISONED"));
                }
                return Err(CommandError::new("TARGET_INVALID"));
            }
        };
        if run.retire().is_err() {
            self.unavailable.store(true, Ordering::Release);
            return Ok(TargetPublishResponse {
                outcome: "indeterminate",
                revision,
                manifest_sha256: Some(receipt.manifest_sha256),
                target_id: receipt.target_id,
                diagnostics: vec![diagnostic(
                    "E_PUBLISH_RECONCILE",
                    "Publication cleanup could not be confirmed; reload before retrying.",
                )],
            });
        }
        Ok(TargetPublishResponse {
            outcome: "published",
            revision,
            manifest_sha256: Some(receipt.manifest_sha256),
            target_id: receipt.target_id,
            diagnostics: Vec::new(),
        })
    }

    pub fn workspace_dispose(&self) -> Result<(), CommandError> {
        self.lifecycle.accepting.store(false, Ordering::Release);
        let _storage_guard = self
            .lifecycle
            .storage_gate
            .lock()
            .map_err(|_| CommandError::new("WORKSPACE_POISONED"))?;
        let _product_guard = self
            .lifecycle
            .product_gate
            .lock()
            .map_err(|_| CommandError::new("WORKSPACE_POISONED"))?;
        self.unavailable.store(true, Ordering::Release);
        self.server.clear().map_err(map_worker_error)?;
        self.targets.clear().map_err(map_target_error)
    }

    fn ensure_available(&self) -> Result<(), CommandError> {
        if self.unavailable.load(Ordering::Acquire) {
            Err(CommandError::new("WORKSPACE_POISONED"))
        } else {
            Ok(())
        }
    }
}

fn publish_failure(
    request: &TargetPublishRequest,
    outcome: &'static str,
    code: &'static str,
) -> TargetPublishResponse {
    TargetPublishResponse {
        outcome,
        revision: request.revision.clone(),
        manifest_sha256: None,
        target_id: request.target_id.clone(),
        diagnostics: vec![diagnostic(
            code,
            if outcome == "failed" {
                "Publication did not commit and may be retried."
            } else {
                "Publication state must be reconciled before retrying."
            },
        )],
    }
}

const fn diagnostic(code: &'static str, message: &'static str) -> CommandDiagnostic {
    CommandDiagnostic {
        severity: "error",
        code,
        message,
    }
}

fn valid_revision(value: &str) -> bool {
    if value.len() != SHA256_REVISION_LENGTH || !value.starts_with("sha256:") {
        return false;
    }
    BASE64
        .decode(&value.as_bytes()["sha256:".len()..])
        .is_ok_and(|bytes| bytes.len() == 32)
}

fn map_store_error(error: StoreError) -> CommandError {
    CommandError::new(match error {
        StoreError::Conflict => "STORAGE_CONFLICT",
        StoreError::MalformedStore => "STORAGE_MALFORMED",
        StoreError::UnsafeProfile => "STORAGE_UNSAFE",
        StoreError::InvalidBytes => "INVALID_REQUEST",
        StoreError::Indeterminate => "STORAGE_INDETERMINATE",
        StoreError::Io => "STORAGE_FAILED",
    })
}

fn map_worker_error(error: WorkerError) -> CommandError {
    CommandError::new(match error {
        WorkerError::Identity => "WORKER_IDENTITY",
        WorkerError::Protocol => "WORKER_PROTOCOL",
        WorkerError::Output => "WORKER_OUTPUT",
        WorkerError::Build => "BUILD_FAILED",
        WorkerError::Timeout => "BUILD_TIMEOUT",
        WorkerError::Sandbox => "SANDBOX_UNAVAILABLE",
        WorkerError::Io => "WORKSPACE_POISONED",
    })
}

fn map_target_error(error: TargetError) -> CommandError {
    CommandError::new(match error {
        TargetError::Invalid => "TARGET_INVALID",
        TargetError::Unsafe => "TARGET_UNSAFE",
        TargetError::Failed => "PUBLISH_FAILED",
        TargetError::Indeterminate => "PUBLISH_INDETERMINATE",
    })
}
