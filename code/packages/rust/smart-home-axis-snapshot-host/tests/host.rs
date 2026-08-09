use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::{InitOptions, SealedStore};
use smart_home_axis_snapshot_host::{
    encode_axis_credentials, AxisCredentialSource, AxisCredentialSourceError,
    AxisSealedStoreCredentialSource, AxisSnapshotEndpoint, AxisSnapshotHost, AxisSnapshotHostError,
    AxisSnapshotRequest, AXIS_VAULT_NAMESPACE, AXIS_VAULT_REF_PREFIX,
};
use smart_home_camera_media::{
    CameraMediaClock, CameraMediaConnectionTarget, CameraMediaCredentialRegistry,
    CameraMediaExecution, CameraMediaExecutionError, CameraMediaExecutionResult,
    CameraMediaExecutor, CameraMediaNonceError, CameraMediaNonceSource, CameraMediaPolicy,
    CameraMediaPrincipalSource,
};
use smart_home_camera_media_http_executor::{
    CameraMediaHttpCredentialError, CameraMediaHttpCredentials, CameraMediaHttpExecutor,
    CameraMediaHttpPolicy,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId,
    CapabilityMode, Device, DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId,
    Metadata, PrivilegeTier, ValueKind, VaultRef,
};
use smart_home_runtime::SmartHomeRuntime;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, VecDeque};
use std::io::{Cursor, Read, Write};
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use storage_core::{InMemoryStorageBackend, StorageBackend};
use tls_platform::{
    TlsConfig, TlsConnectionSummary, TlsConnector, TlsError, TlsStream, TlsVersion,
};

#[derive(Clone)]
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

#[derive(Clone)]
struct FixedPrincipal(Option<AgentId>);

impl CameraMediaPrincipalSource for FixedPrincipal {
    fn current_principal(&self) -> Option<AgentId> {
        self.0.clone()
    }
}

#[derive(Clone)]
struct TestCredentials {
    state: Rc<CredentialState>,
}

struct CredentialState {
    resolve_count: Cell<usize>,
    payload: LeasePayload,
}

impl TestCredentials {
    fn new(payload: LeasePayload) -> Self {
        Self {
            state: Rc::new(CredentialState {
                resolve_count: Cell::new(0),
                payload,
            }),
        }
    }

    fn resolve_count(&self) -> usize {
        self.state.resolve_count.get()
    }
}

impl AxisCredentialSource for TestCredentials {
    fn resolve(&self, _vault_ref: &VaultRef) -> Result<LeasePayload, AxisCredentialSourceError> {
        self.state
            .resolve_count
            .set(self.state.resolve_count.get() + 1);
        Ok(self.state.payload.clone())
    }
}

#[derive(Default)]
struct ExecutorState {
    registered: BTreeSet<EntityId>,
    register_count: usize,
    unregister_count: usize,
    delivery_count: usize,
    last_endpoint: Option<String>,
    fail_registration: bool,
    fail_delivery: bool,
}

struct TestExecutor(Rc<RefCell<ExecutorState>>);

impl CameraMediaCredentialRegistry for TestExecutor {
    type Credentials = CameraMediaHttpCredentials;
    type Error = CameraMediaHttpCredentialError;

    fn register_credentials(
        &mut self,
        entity_id: EntityId,
        _credentials: Self::Credentials,
    ) -> Result<(), Self::Error> {
        let mut state = self.0.borrow_mut();
        state.register_count += 1;
        if state.fail_registration {
            return Err(CameraMediaHttpCredentialError::CredentialAlreadyRegistered);
        }
        state.registered.insert(entity_id);
        Ok(())
    }

    fn unregister_credentials(&mut self, entity_id: &EntityId) -> bool {
        let mut state = self.0.borrow_mut();
        state.unregister_count += 1;
        state.registered.remove(entity_id)
    }
}

impl CameraMediaExecutor for TestExecutor {
    type Stream = ();

    fn deliver(
        &mut self,
        execution: CameraMediaExecution<'_>,
    ) -> Result<CameraMediaExecutionResult<Self::Stream>, CameraMediaExecutionError> {
        let mut state = self.0.borrow_mut();
        assert!(state.registered.contains(execution.entity_id()));
        assert!(execution.connection_target().is_some());
        state.delivery_count += 1;
        state.last_endpoint = Some(execution.endpoint_uri().to_string());
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

type TestHost =
    AxisSnapshotHost<FixedClock, TestNonce, FixedPrincipal, TestExecutor, TestCredentials>;

struct Fixture {
    runtime: SmartHomeRuntime,
    entity_id: EntityId,
    credentials: TestCredentials,
    state: Rc<RefCell<ExecutorState>>,
    host: TestHost,
}

fn fixture_runtime(granted: bool, base_url: &str) -> (SmartHomeRuntime, EntityId, AgentId) {
    let mut runtime = SmartHomeRuntime::new();
    let bridge_id = BridgeId::trusted("bridge:axis:test");
    let device_id = DeviceId::trusted("axis:accc8eaf8c30");
    let entity_id = EntityId::trusted("axis:accc8eaf8c30:camera");
    let principal_id = AgentId::trusted("operator");
    let mut bridge = Bridge::new(
        bridge_id.clone(),
        IntegrationId::trusted("axis_vapix"),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(base_url.to_string());
    bridge.auth_ref = Some(VaultRef::trusted("vault://fixture/axis"));
    runtime.upsert_bridge(bridge).unwrap();
    runtime
        .upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id,
            manufacturer: "AXIS".to_string(),
            model: "Q1785-LE".to_string(),
            name: "AXIS Camera".to_string(),
            serial: Some("ACCC8EAF8C30".to_string()),
            firmware_version: Some("12.1.0".to_string()),
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: Vec::new(),
            health: Health::Online,
            metadata: Vec::new(),
        })
        .unwrap();
    runtime
        .upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id,
            kind: EntityKind::Camera,
            name: "AXIS Camera".to_string(),
            capabilities: vec![Capability::new(
                smart_home_camera_media::CameraMediaKind::Snapshot.capability_id(),
                CapabilityMode::Command,
                ValueKind::Text,
            )],
            state: None,
            metadata: vec![Metadata::new("axis.video_channel", "1")],
        })
        .unwrap();
    if granted {
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("grant:axis-snapshot"),
                principal_id.clone(),
                entity_id.clone(),
                smart_home_camera_media::CameraMediaKind::Snapshot.capability_id(),
                PrivilegeTier::HumanApproval,
                "user",
                1,
            ));
    }
    (runtime, entity_id, principal_id)
}

fn test_host(granted: bool, fail_registration: bool, fail_delivery: bool) -> Fixture {
    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let (runtime, entity_id, principal_id) = fixture_runtime(granted, "https://axis.home");
    let credentials =
        TestCredentials::new(encode_axis_credentials("camera-user", "camera-secret").unwrap());
    let state = Rc::new(RefCell::new(ExecutorState {
        fail_registration,
        fail_delivery,
        ..ExecutorState::default()
    }));
    let policy = CameraMediaPolicy {
        allow_plaintext_loopback: true,
        ..CameraMediaPolicy::default()
    };
    let host = AxisSnapshotHost::new(
        policy,
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        credentials.clone(),
        AxisSnapshotEndpoint::new(
            BridgeId::trusted("bridge:axis:test"),
            CameraMediaConnectionTarget::new("axis.home", address),
        ),
    );
    Fixture {
        runtime,
        entity_id,
        credentials,
        state,
        host,
    }
}

#[test]
fn authorized_delivery_uses_exact_camera_one_endpoint_and_cleans_resources() {
    let mut fixture = test_host(true, false, false);
    let delivery = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            AxisSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
        )
        .unwrap();

    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(fixture.credentials.resolve_count(), 1);
    let state = fixture.state.borrow();
    assert_eq!(state.register_count, 1);
    assert_eq!(state.unregister_count, 1);
    assert_eq!(state.delivery_count, 1);
    assert_eq!(
        state.last_endpoint.as_deref(),
        Some("https://axis.home/axis-cgi/jpg/image.cgi?camera=1")
    );
    assert!(state.registered.is_empty());
    drop(state);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn denial_happens_before_vault_endpoint_or_delivery() {
    let mut fixture = test_host(false, false, false);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            AxisSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, AxisSnapshotHostError::Media(_)));
    assert_eq!(fixture.credentials.resolve_count(), 0);
    assert_eq!(fixture.state.borrow().register_count, 0);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn failed_delivery_still_removes_credentials_and_endpoint() {
    let mut fixture = test_host(true, false, true);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            AxisSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, AxisSnapshotHostError::Media(_)));
    let state = fixture.state.borrow();
    assert_eq!(state.unregister_count, 1);
    assert!(state.registered.is_empty());
    drop(state);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn credential_registration_failure_removes_temporary_endpoint() {
    let mut fixture = test_host(true, true, false);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            AxisSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
        )
        .unwrap_err();

    assert_eq!(error, AxisSnapshotHostError::CredentialRegistrationRejected);
    assert_eq!(fixture.state.borrow().delivery_count, 0);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn invalid_camera_correspondence_is_rejected_before_vault() {
    let mut fixture = test_host(true, false, false);
    let mut entity = fixture
        .runtime
        .registry()
        .entity(&fixture.entity_id)
        .unwrap()
        .clone();
    entity.metadata = vec![Metadata::new("axis.video_channel", "2")];
    fixture.runtime.upsert_entity(entity).unwrap();

    assert_eq!(
        fixture
            .host
            .deliver_snapshot(
                &fixture.runtime,
                AxisSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
            )
            .unwrap_err(),
        AxisSnapshotHostError::InvalidTarget
    );
    assert_eq!(fixture.credentials.resolve_count(), 0);
}

#[test]
fn malformed_credentials_are_redacted_and_never_registered() {
    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let (runtime, entity_id, principal_id) = fixture_runtime(true, "https://axis.home");
    let secret = "raw-axis-password";
    let credentials = TestCredentials::new(LeasePayload::new(
        format!(r#"{{"username":"camera","password":"{secret}","extra":true}}"#).into_bytes(),
    ));
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let mut host = AxisSnapshotHost::new(
        CameraMediaPolicy {
            allow_plaintext_loopback: true,
            ..CameraMediaPolicy::default()
        },
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        credentials,
        AxisSnapshotEndpoint::new(
            BridgeId::trusted("bridge:axis:test"),
            CameraMediaConnectionTarget::new("axis.home", address),
        ),
    );

    let error = host
        .deliver_snapshot(
            &runtime,
            AxisSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap_err();
    let diagnostics = format!("{error:?} {error}");
    assert!(!diagnostics.contains(secret));
    assert_eq!(state.borrow().register_count, 0);
    assert_eq!(host.media_snapshot().endpoint_count, 0);
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
    let payload = encode_axis_credentials("camera-user", "camera-secret").unwrap();
    vault
        .put(
            AXIS_VAULT_NAMESPACE,
            "bridge/main",
            payload.as_bytes(),
            None,
        )
        .unwrap();

    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let (mut runtime, entity_id, principal_id) = fixture_runtime(true, "https://axis.home");
    let bridge_id = BridgeId::trusted("bridge:axis:test");
    let mut bridge = runtime.registry().bridge(&bridge_id).unwrap().clone();
    bridge.auth_ref = Some(VaultRef::trusted(format!(
        "{AXIS_VAULT_REF_PREFIX}bridge/main"
    )));
    runtime.upsert_bridge(bridge).unwrap();
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let mut host = AxisSnapshotHost::new(
        CameraMediaPolicy {
            allow_plaintext_loopback: true,
            ..CameraMediaPolicy::default()
        },
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        AxisSealedStoreCredentialSource::new(vault),
        AxisSnapshotEndpoint::new(
            bridge_id,
            CameraMediaConnectionTarget::new("axis.home", address),
        ),
    );

    for purpose in ["first preview", "second preview"] {
        host.deliver_snapshot(
            &runtime,
            AxisSnapshotRequest::new(entity_id.clone(), purpose, 5_000),
        )
        .unwrap();
    }
    let state = state.borrow();
    assert_eq!(state.register_count, 2);
    assert_eq!(state.unregister_count, 2);
    assert_eq!(state.delivery_count, 2);
    assert!(state.registered.is_empty());
    drop(state);
    assert_eq!(host.media_snapshot().endpoint_count, 0);
}

#[test]
fn native_http_executor_delivers_one_basic_authenticated_axis_jpeg() {
    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let capture = Arc::new(Mutex::new(TlsCapture::default()));
    let connector = RecordingConnector::new(
        VecDeque::from(vec![
            b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"AXIS\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
            wire_response("image/jpeg", &[0xff, 0xd8, 0xff, 0xd9]),
        ]),
        Arc::clone(&capture),
    );
    let (runtime, entity_id, principal_id) = fixture_runtime(true, "https://axis.home");
    let credentials = TestCredentials::new(encode_axis_credentials("user", "secret").unwrap());
    let executor = CameraMediaHttpExecutor::new(
        Box::new(connector),
        TlsConfig::https_default(),
        CameraMediaHttpPolicy::default(),
    );
    let mut host = AxisSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        executor,
        credentials,
        AxisSnapshotEndpoint::new(
            BridgeId::trusted("bridge:axis:test"),
            CameraMediaConnectionTarget::new("axis.home", address),
        ),
    );

    let delivery = host
        .deliver_snapshot(
            &runtime,
            AxisSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap();
    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(host.media_snapshot().endpoint_count, 0);
    let capture = capture.lock().unwrap();
    assert_eq!(capture.requests.len(), 2);
    let first = String::from_utf8(capture.requests[0].clone()).unwrap();
    assert!(first.starts_with("GET /axis-cgi/jpg/image.cgi?camera=1 HTTP/1.1\r\n"));
    assert!(!first.contains("Authorization:"));
    let second = String::from_utf8(capture.requests[1].clone()).unwrap();
    assert!(second.starts_with("GET /axis-cgi/jpg/image.cgi?camera=1 HTTP/1.1\r\n"));
    assert!(second.contains("Authorization: Basic dXNlcjpzZWNyZXQ="));
    assert_eq!(
        capture.pinned_connections,
        vec![
            ("axis.home".to_string(), address),
            ("axis.home".to_string(), address)
        ]
    );
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
            capture: Arc::clone(&self.capture),
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
        self.stream()
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

fn wire_response(content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}
