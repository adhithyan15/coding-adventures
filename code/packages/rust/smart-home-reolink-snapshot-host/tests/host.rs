use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::{InitOptions, SealedStore};
use smart_home_camera_media::{
    CameraMediaClock, CameraMediaConnectionTarget, CameraMediaExecution, CameraMediaExecutionError,
    CameraMediaExecutionResult, CameraMediaExecutor, CameraMediaNonceError, CameraMediaNonceSource,
    CameraMediaPolicy, CameraMediaPrincipalSource,
};
use smart_home_camera_media_http_executor::{CameraMediaHttpExecutor, CameraMediaHttpPolicy};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId,
    CapabilityMode, Device, DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId,
    Metadata, PrivilegeTier, StateConfidence, StateSnapshot, StateSource, Value, ValueKind,
    VaultRef,
};
use smart_home_reolink_snapshot_host::{
    encode_reolink_credentials, ReolinkCredentialSource, ReolinkCredentialSourceError,
    ReolinkSealedStoreCredentialSource, ReolinkSnapshotEndpoint, ReolinkSnapshotHost,
    ReolinkSnapshotHostError, ReolinkSnapshotRequest, REOLINK_VAULT_NAMESPACE,
    REOLINK_VAULT_REF_PREFIX,
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
    resolve_count: Rc<Cell<usize>>,
    payload: LeasePayload,
}

impl TestCredentials {
    fn new(payload: LeasePayload) -> Self {
        Self {
            resolve_count: Rc::new(Cell::new(0)),
            payload,
        }
    }

    fn resolve_count(&self) -> usize {
        self.resolve_count.get()
    }
}

impl ReolinkCredentialSource for TestCredentials {
    fn resolve(&self, _vault_ref: &VaultRef) -> Result<LeasePayload, ReolinkCredentialSourceError> {
        self.resolve_count.set(self.resolve_count.get() + 1);
        Ok(self.payload.clone())
    }
}

#[derive(Default)]
struct ExecutorState {
    delivery_count: usize,
    last_endpoint: Option<String>,
    fail_delivery: bool,
}

struct TestExecutor(Rc<RefCell<ExecutorState>>);

impl CameraMediaExecutor for TestExecutor {
    type Stream = ();

    fn deliver(
        &mut self,
        execution: CameraMediaExecution<'_>,
    ) -> Result<CameraMediaExecutionResult<Self::Stream>, CameraMediaExecutionError> {
        let mut state = self.0.borrow_mut();
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
    ReolinkSnapshotHost<FixedClock, TestNonce, FixedPrincipal, TestExecutor, TestCredentials>;

struct Fixture {
    runtime: SmartHomeRuntime,
    entity_id: EntityId,
    credentials: TestCredentials,
    state: Rc<RefCell<ExecutorState>>,
    host: TestHost,
}

fn fixture_runtime(granted: bool, base_url: &str) -> (SmartHomeRuntime, EntityId, AgentId) {
    let mut runtime = SmartHomeRuntime::new();
    let bridge_id = BridgeId::trusted("bridge:reolink:test");
    let device_id = DeviceId::trusted("reolink:abc123:ch0");
    let entity_id = EntityId::trusted("reolink:abc123:ch0:camera");
    let principal_id = AgentId::trusted("operator");
    let mut bridge = Bridge::new(
        bridge_id.clone(),
        IntegrationId::trusted("reolink"),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(base_url.to_string());
    bridge.auth_ref = Some(VaultRef::trusted("vault://fixture/reolink"));
    runtime.upsert_bridge(bridge).unwrap();
    runtime
        .upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id,
            manufacturer: "Reolink".to_string(),
            model: "RLC-520A".to_string(),
            name: "Driveway".to_string(),
            serial: None,
            firmware_version: Some("v3.1".to_string()),
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
            name: "Driveway".to_string(),
            capabilities: vec![
                Capability::new(
                    smart_home_camera_media::CameraMediaKind::Snapshot.capability_id(),
                    CapabilityMode::Command,
                    ValueKind::Text,
                ),
                Capability::new(
                    smart_home_core::CapabilityId::trusted("camera.health"),
                    CapabilityMode::Observe,
                    ValueKind::Object,
                ),
            ],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: Value::Object(vec![
                    ("online".to_string(), Value::Bool(true)),
                    ("sleeping".to_string(), Value::Bool(false)),
                ]),
                source: StateSource::Poll,
                observed_at_ms: 1,
                received_at_ms: 1,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("reolink.channel", "0")],
        })
        .unwrap();
    if granted {
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("grant:reolink-snapshot"),
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

fn test_host(granted: bool, fail_delivery: bool) -> Fixture {
    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let (runtime, entity_id, principal_id) = fixture_runtime(granted, "https://reolink.home");
    let credentials =
        TestCredentials::new(encode_reolink_credentials("camera-user", "camera-secret").unwrap());
    let state = Rc::new(RefCell::new(ExecutorState {
        fail_delivery,
        ..ExecutorState::default()
    }));
    let host = ReolinkSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        credentials.clone(),
        ReolinkSnapshotEndpoint::new(
            BridgeId::trusted("bridge:reolink:test"),
            CameraMediaConnectionTarget::new("reolink.home", address),
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
fn authorized_delivery_uses_exact_channel_endpoint_and_cleans_it() {
    let mut fixture = test_host(true, false);
    let delivery = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ReolinkSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
        )
        .unwrap();

    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(fixture.credentials.resolve_count(), 1);
    let state = fixture.state.borrow();
    assert_eq!(state.delivery_count, 1);
    assert_eq!(
        state.last_endpoint.as_deref(),
        Some("https://reolink.home/cgi-bin/api.cgi?cmd=Snap&channel=0&rs=smart-home-d23&user=camera-user&password=camera-secret")
    );
    drop(state);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn denial_happens_before_vault_endpoint_or_delivery() {
    let mut fixture = test_host(false, false);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ReolinkSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, ReolinkSnapshotHostError::Media(_)));
    assert_eq!(fixture.credentials.resolve_count(), 0);
    assert_eq!(fixture.state.borrow().delivery_count, 0);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn failed_delivery_still_removes_the_token_bearing_endpoint() {
    let mut fixture = test_host(true, true);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ReolinkSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, ReolinkSnapshotHostError::Media(_)));
    assert_eq!(fixture.state.borrow().delivery_count, 1);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn unsupported_or_stale_channel_is_rejected_before_vault() {
    let mut fixture = test_host(true, false);
    let mut device = fixture
        .runtime
        .registry()
        .device(&DeviceId::trusted("reolink:abc123:ch0"))
        .unwrap()
        .clone();
    device.model = "RLN8-410".to_string();
    fixture.runtime.upsert_device(device).unwrap();

    assert_eq!(
        fixture
            .host
            .deliver_snapshot(
                &fixture.runtime,
                ReolinkSnapshotRequest::new(fixture.entity_id, "operator preview", 5_000),
            )
            .unwrap_err(),
        ReolinkSnapshotHostError::InvalidTarget
    );
    assert_eq!(fixture.credentials.resolve_count(), 0);
}

#[test]
fn malformed_credentials_are_redacted_and_never_delivered() {
    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let (runtime, entity_id, principal_id) = fixture_runtime(true, "https://reolink.home");
    let secret = "raw-reolink-password";
    let credentials = TestCredentials::new(LeasePayload::new(
        format!(r#"{{"username":"camera","password":"{secret}","extra":true}}"#).into_bytes(),
    ));
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let mut host = ReolinkSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        credentials,
        ReolinkSnapshotEndpoint::new(
            BridgeId::trusted("bridge:reolink:test"),
            CameraMediaConnectionTarget::new("reolink.home", address),
        ),
    );

    let error = host
        .deliver_snapshot(
            &runtime,
            ReolinkSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap_err();
    let diagnostics = format!("{error:?} {error}");
    assert!(!diagnostics.contains(secret));
    assert_eq!(state.borrow().delivery_count, 0);
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
    let payload = encode_reolink_credentials("camera-user", "camera-secret").unwrap();
    vault
        .put(
            REOLINK_VAULT_NAMESPACE,
            "bridge/main",
            payload.as_bytes(),
            None,
        )
        .unwrap();

    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let (mut runtime, entity_id, principal_id) = fixture_runtime(true, "https://reolink.home");
    let bridge_id = BridgeId::trusted("bridge:reolink:test");
    let mut bridge = runtime.registry().bridge(&bridge_id).unwrap().clone();
    bridge.auth_ref = Some(VaultRef::trusted(format!(
        "{REOLINK_VAULT_REF_PREFIX}bridge/main"
    )));
    runtime.upsert_bridge(bridge).unwrap();
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let mut host = ReolinkSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        ReolinkSealedStoreCredentialSource::new(vault),
        ReolinkSnapshotEndpoint::new(
            bridge_id,
            CameraMediaConnectionTarget::new("reolink.home", address),
        ),
    );

    for purpose in ["first preview", "second preview"] {
        host.deliver_snapshot(
            &runtime,
            ReolinkSnapshotRequest::new(entity_id.clone(), purpose, 5_000),
        )
        .unwrap();
    }
    assert_eq!(state.borrow().delivery_count, 2);
    assert_eq!(host.media_snapshot().endpoint_count, 0);
}

#[test]
fn native_http_executor_uses_pinned_tls_and_percent_encoded_query_credentials() {
    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let capture = Arc::new(Mutex::new(TlsCapture::default()));
    let connector = RecordingConnector::new(
        VecDeque::from(vec![wire_response("image/jpeg", &[0xff, 0xd8, 0xff, 0xd9])]),
        Arc::clone(&capture),
    );
    let (runtime, entity_id, principal_id) = fixture_runtime(true, "https://reolink.home");
    let credentials =
        TestCredentials::new(encode_reolink_credentials("operator+home", "s&ecret/1").unwrap());
    let executor = CameraMediaHttpExecutor::new(
        Box::new(connector),
        TlsConfig::https_default(),
        CameraMediaHttpPolicy::default(),
    );
    let mut host = ReolinkSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        executor,
        credentials,
        ReolinkSnapshotEndpoint::new(
            BridgeId::trusted("bridge:reolink:test"),
            CameraMediaConnectionTarget::new("reolink.home", address),
        ),
    );

    let delivery = host
        .deliver_snapshot(
            &runtime,
            ReolinkSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap();
    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(host.media_snapshot().endpoint_count, 0);
    let capture = capture.lock().unwrap();
    assert_eq!(capture.requests.len(), 1);
    let request = String::from_utf8(capture.requests[0].clone()).unwrap();
    assert!(request.starts_with(
        "GET /cgi-bin/api.cgi?cmd=Snap&channel=0&rs=smart-home-d23&user=operator%2Bhome&password=s%26ecret%2F1 HTTP/1.1\r\n"
    ));
    assert!(!request.contains("s&ecret/1"));
    assert_eq!(
        capture.pinned_connections,
        vec![("reolink.home".to_string(), address)]
    );
}

#[test]
fn root_slash_is_normalized_and_preconfigured_query_data_is_rejected() {
    let address: SocketAddr = "192.0.2.25:443".parse().unwrap();
    let (runtime, entity_id, principal_id) = fixture_runtime(true, "https://reolink.home/");
    let credentials =
        TestCredentials::new(encode_reolink_credentials("camera-user", "camera-secret").unwrap());
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let mut host = ReolinkSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        credentials.clone(),
        ReolinkSnapshotEndpoint::new(
            BridgeId::trusted("bridge:reolink:test"),
            CameraMediaConnectionTarget::new("reolink.home", address),
        ),
    );
    host.deliver_snapshot(
        &runtime,
        ReolinkSnapshotRequest::new(entity_id, "operator preview", 5_000),
    )
    .unwrap();
    assert_eq!(
        state.borrow().last_endpoint.as_deref(),
        Some("https://reolink.home/cgi-bin/api.cgi?cmd=Snap&channel=0&rs=smart-home-d23&user=camera-user&password=camera-secret")
    );

    let (runtime, entity_id, principal_id) =
        fixture_runtime(true, "https://reolink.home/?unexpected=true");
    let mut host = ReolinkSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(Rc::new(RefCell::new(ExecutorState::default()))),
        credentials.clone(),
        ReolinkSnapshotEndpoint::new(
            BridgeId::trusted("bridge:reolink:test"),
            CameraMediaConnectionTarget::new("reolink.home", address),
        ),
    );
    assert_eq!(
        host.deliver_snapshot(
            &runtime,
            ReolinkSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap_err(),
        ReolinkSnapshotHostError::InvalidTarget
    );
    assert_eq!(credentials.resolve_count(), 1);
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
