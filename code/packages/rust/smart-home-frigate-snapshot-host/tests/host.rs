use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::{InitOptions, SealedStore};
use smart_home_camera_media::{
    CameraMediaClock, CameraMediaConnectionTarget, CameraMediaCredentialRegistry,
    CameraMediaExecution, CameraMediaExecutionError, CameraMediaExecutionResult,
    CameraMediaExecutor, CameraMediaKind, CameraMediaNonceError, CameraMediaNonceSource,
    CameraMediaPolicy, CameraMediaPrincipalSource,
};
use smart_home_core::{
    AgentId, BridgeId, CapabilityGrant, CapabilityGrantId, PrivilegeTier, VaultRef,
};
use smart_home_frigate_integration::{
    install_snapshot, FrigateCameraStats, FrigateConfig, FrigateSnapshot,
};
use smart_home_frigate_snapshot_host::{
    encode_frigate_credentials, FrigateCredentialSource, FrigateCredentialSourceError,
    FrigateExecutorCredentialError, FrigateExecutorCredentials, FrigateSealedStoreCredentialSource,
    FrigateSnapshotEndpoint, FrigateSnapshotExecutor, FrigateSnapshotHost,
    FrigateSnapshotHostError, FrigateSnapshotRequest, FRIGATE_VAULT_NAMESPACE,
    FRIGATE_VAULT_REF_PREFIX,
};
use smart_home_runtime::SmartHomeRuntime;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::io::{Cursor, Read, Write};
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use storage_core::{InMemoryStorageBackend, StorageBackend};
use tls_platform::{
    TlsConfig, TlsConnectionSummary, TlsConnector, TlsError, TlsStream, TlsVersion,
};

struct FixedClock(u64);

impl CameraMediaClock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.0
    }
}

struct TestNonce(u8);

impl CameraMediaNonceSource for TestNonce {
    fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), CameraMediaNonceError> {
        output.fill(self.0);
        self.0 = self.0.wrapping_add(1);
        Ok(())
    }
}

struct FixedPrincipal(Option<AgentId>);

impl CameraMediaPrincipalSource for FixedPrincipal {
    fn current_principal(&self) -> Option<AgentId> {
        self.0.clone()
    }
}

#[derive(Clone)]
struct TestCredentials {
    payload: Rc<RefCell<Option<Vec<u8>>>>,
    resolve_count: Rc<Cell<usize>>,
}

impl TestCredentials {
    fn new(payload: LeasePayload) -> Self {
        Self {
            payload: Rc::new(RefCell::new(Some(payload.as_bytes().to_vec()))),
            resolve_count: Rc::new(Cell::new(0)),
        }
    }

    fn resolve_count(&self) -> usize {
        self.resolve_count.get()
    }
}

impl FrigateCredentialSource for TestCredentials {
    fn resolve(&self, _vault_ref: &VaultRef) -> Result<LeasePayload, FrigateCredentialSourceError> {
        self.resolve_count.set(self.resolve_count.get() + 1);
        self.payload
            .borrow()
            .as_ref()
            .cloned()
            .map(LeasePayload::new)
            .ok_or(FrigateCredentialSourceError)
    }
}

#[derive(Default)]
struct ExecutorState {
    delivery_count: usize,
    registered_count: usize,
    removed_count: usize,
    fail_delivery: bool,
    endpoint: Option<String>,
}

struct TestExecutor(Rc<RefCell<ExecutorState>>);

impl CameraMediaExecutor for TestExecutor {
    type Stream = ();

    fn deliver(
        &mut self,
        execution: CameraMediaExecution<'_>,
    ) -> Result<CameraMediaExecutionResult<Self::Stream>, CameraMediaExecutionError> {
        let mut state = self.0.borrow_mut();
        state.delivery_count += 1;
        state.endpoint = Some(execution.endpoint_uri().to_string());
        if state.fail_delivery {
            return Err(CameraMediaExecutionError::Unavailable);
        }
        Ok(CameraMediaExecutionResult::snapshot(vec![
            0xff, 0xd8, 0xff, 0xd9,
        ]))
    }

    fn close_stream(
        &mut self,
        _stream: &mut Self::Stream,
    ) -> Result<(), CameraMediaExecutionError> {
        Err(CameraMediaExecutionError::Rejected)
    }
}

impl CameraMediaCredentialRegistry for TestExecutor {
    type Credentials = FrigateExecutorCredentials;
    type Error = FrigateExecutorCredentialError;

    fn register_credentials(
        &mut self,
        _entity_id: smart_home_core::EntityId,
        _credentials: Self::Credentials,
    ) -> Result<(), Self::Error> {
        self.0.borrow_mut().registered_count += 1;
        Ok(())
    }

    fn unregister_credentials(&mut self, _entity_id: &smart_home_core::EntityId) -> bool {
        self.0.borrow_mut().removed_count += 1;
        true
    }
}

type TestHost =
    FrigateSnapshotHost<FixedClock, TestNonce, FixedPrincipal, TestExecutor, TestCredentials>;

struct Fixture {
    runtime: SmartHomeRuntime,
    entity_id: smart_home_core::EntityId,
    credentials: TestCredentials,
    executor: Rc<RefCell<ExecutorState>>,
    host: TestHost,
}

fn fixture(granted: bool, fail_delivery: bool) -> Fixture {
    let bridge_id = BridgeId::trusted("bridge:frigate:test");
    let (runtime, entity_id, principal_id) = fixture_runtime(
        granted,
        bridge_id.clone(),
        VaultRef::trusted("vault://fixture/frigate"),
        "https://frigate.home:8971",
    );
    let credentials =
        TestCredentials::new(encode_frigate_credentials("operator", "secret").unwrap());
    let executor = Rc::new(RefCell::new(ExecutorState {
        fail_delivery,
        ..ExecutorState::default()
    }));
    let host = FrigateSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(executor.clone()),
        credentials.clone(),
        FrigateSnapshotEndpoint::new(
            bridge_id,
            CameraMediaConnectionTarget::new("frigate.home", "192.0.2.10:8971".parse().unwrap()),
        ),
    );
    Fixture {
        runtime,
        entity_id,
        credentials,
        executor,
        host,
    }
}

fn fixture_runtime(
    granted: bool,
    bridge_id: BridgeId,
    vault_ref: VaultRef,
    base_url: &str,
) -> (SmartHomeRuntime, smart_home_core::EntityId, AgentId) {
    let config = FrigateConfig::new(bridge_id, base_url, vault_ref).unwrap();
    let snapshot = FrigateSnapshot {
        version: "0.17.2".to_string(),
        cameras: vec![FrigateCameraStats {
            name: "Front Door".to_string(),
            camera_fps: 5.0,
            process_fps: 5.0,
            skipped_fps: 0.0,
            detection_fps: 1.0,
            detection_enabled: true,
            connection_quality: Some("excellent".to_string()),
            expected_fps: Some(5.0),
            reconnects_last_hour: Some(0),
            stalls_last_hour: Some(0),
        }],
    };
    let mut runtime = SmartHomeRuntime::new();
    let installed = install_snapshot(&mut runtime, &config, &snapshot, 10).unwrap();
    let entity_id = installed.cameras[0].camera_entity_id.clone();
    let principal_id = AgentId::trusted("operator");
    if granted {
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("grant:frigate:snapshot"),
                principal_id.clone(),
                entity_id.clone(),
                CameraMediaKind::Snapshot.capability_id(),
                PrivilegeTier::HumanApproval,
                "user",
                1,
            ));
    }
    (runtime, entity_id, principal_id)
}

#[test]
fn authorized_delivery_registers_and_removes_one_endpoint_and_credentials() {
    let mut fixture = fixture(true, false);
    let delivery = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            FrigateSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap();

    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(fixture.credentials.resolve_count(), 1);
    let executor = fixture.executor.borrow();
    assert_eq!(executor.delivery_count, 1);
    assert_eq!(executor.registered_count, 1);
    assert_eq!(executor.removed_count, 1);
    assert_eq!(
        executor.endpoint.as_deref(),
        Some("https://frigate.home:8971/api/Front%20Door/latest.jpg")
    );
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn denial_happens_before_vault_registration_or_delivery() {
    let mut fixture = fixture(false, false);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            FrigateSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, FrigateSnapshotHostError::Media(_)));
    assert_eq!(fixture.credentials.resolve_count(), 0);
    assert_eq!(fixture.executor.borrow().registered_count, 0);
    assert_eq!(fixture.executor.borrow().delivery_count, 0);
}

#[test]
fn failed_delivery_still_removes_endpoint_and_credentials() {
    let mut fixture = fixture(true, true);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            FrigateSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, FrigateSnapshotHostError::Media(_)));
    assert_eq!(fixture.executor.borrow().removed_count, 1);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn malformed_credentials_are_redacted_and_never_registered() {
    let mut fixture = fixture(true, false);
    let secret = "raw-frigate-password";
    fixture.credentials = TestCredentials::new(LeasePayload::new(
        format!(
            r#"{{"schema_version":1,"username":"operator","password":"{secret}","extra":true}}"#
        )
        .into_bytes(),
    ));
    fixture.host = FrigateSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(AgentId::trusted("operator"))),
        TestExecutor(fixture.executor.clone()),
        fixture.credentials.clone(),
        FrigateSnapshotEndpoint::new(
            BridgeId::trusted("bridge:frigate:test"),
            CameraMediaConnectionTarget::new("frigate.home", "192.0.2.10:8971".parse().unwrap()),
        ),
    );

    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            FrigateSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();
    let diagnostics = format!("{error:?} {error}");
    assert_eq!(error, FrigateSnapshotHostError::InvalidCredentialPayload);
    assert!(!diagnostics.contains(secret));
    assert_eq!(fixture.executor.borrow().registered_count, 0);
}

#[test]
fn mismatched_reviewed_address_is_rejected_before_vault() {
    let mut fixture = fixture(true, false);
    fixture.host = FrigateSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(AgentId::trusted("operator"))),
        TestExecutor(fixture.executor.clone()),
        fixture.credentials.clone(),
        FrigateSnapshotEndpoint::new(
            BridgeId::trusted("bridge:frigate:test"),
            CameraMediaConnectionTarget::new("other.home", "192.0.2.10:8971".parse().unwrap()),
        ),
    );
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            FrigateSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();
    assert_eq!(error, FrigateSnapshotHostError::InvalidTarget);
    assert_eq!(fixture.credentials.resolve_count(), 0);
}

#[test]
fn sealed_vault_record_supports_repeated_approved_deliveries() {
    let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::default());
    backend.initialize().unwrap();
    let vault = Arc::new(SealedStore::new(backend));
    vault
        .init(
            b"fixture-password",
            &InitOptions {
                argon2id_time_cost: 1,
                argon2id_memory_kib: 32,
                argon2id_parallelism: 1,
                salt_override: Some(vec![9; 16]),
            },
        )
        .unwrap();
    let payload = encode_frigate_credentials("operator", "secret").unwrap();
    vault
        .put(
            FRIGATE_VAULT_NAMESPACE,
            "bridge/main",
            payload.as_bytes(),
            None,
        )
        .unwrap();
    let bridge_id = BridgeId::trusted("bridge:frigate:test");
    let (runtime, entity_id, principal_id) = fixture_runtime(
        true,
        bridge_id.clone(),
        VaultRef::trusted(format!("{FRIGATE_VAULT_REF_PREFIX}bridge/main")),
        "https://frigate.home:8971",
    );
    let executor = Rc::new(RefCell::new(ExecutorState::default()));
    let mut host = FrigateSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(executor.clone()),
        FrigateSealedStoreCredentialSource::new(vault),
        FrigateSnapshotEndpoint::new(
            bridge_id,
            CameraMediaConnectionTarget::new("frigate.home", "192.0.2.10:8971".parse().unwrap()),
        ),
    );
    for purpose in ["first preview", "second preview"] {
        host.deliver_snapshot(
            &runtime,
            FrigateSnapshotRequest::new(entity_id.clone(), purpose, 5_000),
        )
        .unwrap();
    }
    assert_eq!(executor.borrow().delivery_count, 2);
    assert_eq!(executor.borrow().registered_count, 2);
    assert_eq!(executor.borrow().removed_count, 2);
    assert_eq!(host.media_snapshot().endpoint_count, 0);
}

#[test]
fn strict_transport_uses_pinned_tls_cookie_auth_and_logout() {
    let capture = Arc::new(Mutex::new(TlsCapture::default()));
    let responses = VecDeque::from(vec![
        wire_response(
            "200 OK",
            &[("Set-Cookie", "frigate_token=secret.jwt; HttpOnly")],
            "application/json",
            br#"{"success":true}"#,
        ),
        wire_response("200 OK", &[], "image/jpeg", &[0xff, 0xd8, 0xff, 0xd9]),
        wire_response("303 See Other", &[], "application/json", b""),
    ]);
    let bridge_id = BridgeId::trusted("bridge:frigate:test");
    let (runtime, entity_id, principal_id) = fixture_runtime(
        true,
        bridge_id.clone(),
        VaultRef::trusted("vault://fixture/frigate"),
        "https://frigate.home:8971",
    );
    let executor = FrigateSnapshotExecutor::with_connector(
        Box::new(RecordingConnector::new(responses, capture.clone())),
        TlsConfig::https_default(),
    );
    let mut host = FrigateSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        executor,
        TestCredentials::new(encode_frigate_credentials("operator", "secret").unwrap()),
        FrigateSnapshotEndpoint::new(
            bridge_id,
            CameraMediaConnectionTarget::new("frigate.home", "192.0.2.10:8971".parse().unwrap()),
        ),
    );

    let delivery = host
        .deliver_snapshot(
            &runtime,
            FrigateSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap();
    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(host.media_snapshot().endpoint_count, 0);

    let capture = capture.lock().unwrap();
    assert_eq!(capture.requests.len(), 3);
    assert_eq!(
        capture.pinned_connections,
        vec![
            (
                "frigate.home".to_string(),
                "192.0.2.10:8971".parse().unwrap()
            ),
            (
                "frigate.home".to_string(),
                "192.0.2.10:8971".parse().unwrap()
            ),
            (
                "frigate.home".to_string(),
                "192.0.2.10:8971".parse().unwrap()
            ),
        ]
    );
    let login = String::from_utf8(capture.requests[0].clone()).unwrap();
    assert!(login.starts_with("POST /api/login HTTP/1.1"));
    assert!(login.contains(r#"{"password":"secret","user":"operator"}"#));
    assert!(!login.contains("Cookie:"));
    let snapshot = String::from_utf8(capture.requests[1].clone()).unwrap();
    assert!(snapshot.starts_with("GET /api/Front%20Door/latest.jpg HTTP/1.1"));
    assert!(snapshot.contains("Cookie: frigate_token=secret.jwt"));
    assert!(!snapshot.contains("operator"));
    assert!(!snapshot.contains("secret\""));
    let logout = String::from_utf8(capture.requests[2].clone()).unwrap();
    assert!(logout.starts_with("GET /api/logout HTTP/1.1"));
    assert!(logout.contains("Cookie: frigate_token=secret.jwt"));
}

#[derive(Default)]
struct TlsCapture {
    pinned_connections: Vec<(String, SocketAddr)>,
    requests: Vec<Vec<u8>>,
}

struct RecordingConnector {
    responses: Arc<Mutex<VecDeque<Vec<u8>>>>,
    capture: Arc<Mutex<TlsCapture>>,
}

impl RecordingConnector {
    fn new(responses: VecDeque<Vec<u8>>, capture: Arc<Mutex<TlsCapture>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            capture,
        }
    }

    fn stream(&self) -> Result<Box<dyn TlsStream>, TlsError> {
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| TlsError::Io {
                phase: "fixture response",
                source: std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "missing fixture response",
                ),
            })?;
        Ok(Box::new(RecordingTlsStream {
            response: Cursor::new(response),
            request: Vec::new(),
            capture: self.capture.clone(),
        }))
    }
}

impl TlsConnector for RecordingConnector {
    fn connect(
        &self,
        _host: &str,
        _port: u16,
        _config: &TlsConfig,
    ) -> Result<Box<dyn TlsStream>, TlsError> {
        panic!("snapshot delivery must use reviewed-address pinning")
    }

    fn connect_addr(
        &self,
        server_name: &str,
        address: SocketAddr,
        _config: &TlsConfig,
    ) -> Result<Box<dyn TlsStream>, TlsError> {
        self.capture
            .lock()
            .unwrap()
            .pinned_connections
            .push((server_name.to_string(), address));
        self.stream()
    }
}

struct RecordingTlsStream {
    response: Cursor<Vec<u8>>,
    request: Vec<u8>,
    capture: Arc<Mutex<TlsCapture>>,
}

impl Drop for RecordingTlsStream {
    fn drop(&mut self) {
        self.capture
            .lock()
            .unwrap()
            .requests
            .push(std::mem::take(&mut self.request));
    }
}

impl Read for RecordingTlsStream {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        self.response.read(output)
    }
}

impl Write for RecordingTlsStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.request.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl TlsStream for RecordingTlsStream {
    fn peer_certificates(&self) -> Result<Vec<Vec<u8>>, TlsError> {
        Ok(Vec::new())
    }

    fn negotiated_alpn(&self) -> Option<String> {
        Some("http/1.1".to_string())
    }

    fn negotiated_version(&self) -> TlsVersion {
        TlsVersion::Tls13
    }

    fn close_notify(&mut self) -> Result<(), TlsError> {
        Ok(())
    }

    fn summary(&self) -> TlsConnectionSummary {
        panic!("summary is not used by the snapshot host")
    }
}

fn wire_response(
    status: &str,
    headers: &[(&str, &str)],
    content_type: &str,
    body: &[u8],
) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {status}\r\n").into_bytes();
    for (name, value) in headers {
        response.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    response.extend_from_slice(
        format!(
            "Content-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .as_bytes(),
    );
    response.extend_from_slice(body);
    response
}
