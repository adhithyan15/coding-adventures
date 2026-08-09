use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::{InitOptions, SealedStore};
use smart_home_camera_media::{
    CameraMediaClock, CameraMediaConnectionTarget, CameraMediaExecution, CameraMediaExecutionError,
    CameraMediaExecutionResult, CameraMediaExecutor, CameraMediaKind, CameraMediaNonceError,
    CameraMediaNonceSource, CameraMediaPolicy, CameraMediaPrincipalSource,
};
use smart_home_camera_media_http_executor::{CameraMediaHttpExecutor, CameraMediaHttpPolicy};
use smart_home_core::{
    AgentId, BridgeId, CapabilityGrant, CapabilityGrantId, PrivilegeTier, VaultRef,
};
use smart_home_runtime::SmartHomeRuntime;
use smart_home_zoneminder_integration::{
    install_snapshot, ZoneMinderAccessToken, ZoneMinderConfig, ZoneMinderCredentials,
    ZoneMinderLanTransport, ZoneMinderMonitor, ZoneMinderSnapshot,
};
use smart_home_zoneminder_snapshot_host::{
    encode_zoneminder_credentials, ZoneMinderAccessTokenSource, ZoneMinderAccessTokenSourceError,
    ZoneMinderCredentialSource, ZoneMinderCredentialSourceError, ZoneMinderLanAccessTokenSource,
    ZoneMinderSealedStoreCredentialSource, ZoneMinderSnapshotEndpoint, ZoneMinderSnapshotHost,
    ZoneMinderSnapshotHostError, ZoneMinderSnapshotRequest, ZoneMinderSnapshotResources,
    ZONEMINDER_VAULT_NAMESPACE, ZONEMINDER_VAULT_REF_PREFIX,
};
use std::cell::{Cell, RefCell};
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

impl ZoneMinderCredentialSource for TestCredentials {
    fn resolve(
        &self,
        _vault_ref: &VaultRef,
    ) -> Result<LeasePayload, ZoneMinderCredentialSourceError> {
        self.state
            .resolve_count
            .set(self.state.resolve_count.get() + 1);
        Ok(self.state.payload.clone())
    }
}

#[derive(Clone)]
struct TestTokens {
    calls: Rc<Cell<usize>>,
}

impl TestTokens {
    fn new() -> Self {
        Self {
            calls: Rc::new(Cell::new(0)),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.get()
    }
}

impl ZoneMinderAccessTokenSource for TestTokens {
    fn acquire(
        &mut self,
        _config: &ZoneMinderConfig,
        credentials: &ZoneMinderCredentials,
    ) -> Result<ZoneMinderAccessToken, ZoneMinderAccessTokenSourceError> {
        assert_eq!(
            format!("{credentials:?}"),
            "ZoneMinderCredentials([REDACTED])"
        );
        self.calls.set(self.calls.get() + 1);
        ZoneMinderAccessToken::new("secret.jwt_value.signature")
            .map_err(|_| ZoneMinderAccessTokenSourceError)
    }
}

#[derive(Default)]
struct ExecutorState {
    delivery_count: usize,
    fail_delivery: bool,
    endpoints: Vec<String>,
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
        state.endpoints.push(execution.endpoint_uri().to_string());
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

type TestHost = ZoneMinderSnapshotHost<
    FixedClock,
    TestNonce,
    FixedPrincipal,
    TestExecutor,
    TestCredentials,
    TestTokens,
>;

struct Fixture {
    runtime: SmartHomeRuntime,
    entity_id: smart_home_core::EntityId,
    credentials: TestCredentials,
    tokens: TestTokens,
    executor: Rc<RefCell<ExecutorState>>,
    host: TestHost,
}

fn fixture(granted: bool, fail_delivery: bool) -> Fixture {
    let bridge_id = BridgeId::trusted("bridge:zoneminder:test");
    let vault_ref = VaultRef::trusted("vault://fixture/zoneminder");
    let base_url = "https://zoneminder.home/zm";
    let (runtime, entity_id, principal_id) =
        fixture_runtime(granted, bridge_id.clone(), vault_ref.clone(), base_url);
    let credentials =
        TestCredentials::new(encode_zoneminder_credentials("operator", "secret").unwrap());
    let tokens = TestTokens::new();
    let executor = Rc::new(RefCell::new(ExecutorState {
        fail_delivery,
        ..ExecutorState::default()
    }));
    let endpoint = ZoneMinderSnapshotEndpoint::new(
        bridge_id,
        "https://zoneminder.home/cgi-bin/nph-zms",
        CameraMediaConnectionTarget::new("zoneminder.home", "192.0.2.10:443".parse().unwrap()),
    )
    .unwrap();
    let host = ZoneMinderSnapshotHost::new(
        CameraMediaPolicy {
            allow_plaintext_loopback: true,
            ..CameraMediaPolicy::default()
        },
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(executor.clone()),
        ZoneMinderSnapshotResources::new(credentials.clone(), tokens.clone(), endpoint),
    );
    Fixture {
        runtime,
        entity_id,
        credentials,
        tokens,
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
    let config = ZoneMinderConfig::new(bridge_id, base_url, vault_ref).unwrap();
    let snapshot = ZoneMinderSnapshot {
        version: "1.36.33".to_string(),
        api_version: "2.0".to_string(),
        monitors: vec![ZoneMinderMonitor {
            id: 7,
            name: "Front".to_string(),
            enabled: true,
            capturing: "Always".to_string(),
            analysing: "Always".to_string(),
            recording: "OnMotion".to_string(),
            status: "Connected".to_string(),
            capture_fps: Some(5.0),
            analysis_fps: Some(1.0),
            capture_bandwidth: Some(42_000),
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
                CapabilityGrantId::trusted("grant:zoneminder:snapshot"),
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
fn authorized_delivery_uses_one_token_and_removes_the_endpoint() {
    let mut fixture = fixture(true, false);
    let delivery = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ZoneMinderSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap();

    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(fixture.credentials.resolve_count(), 1);
    assert_eq!(fixture.tokens.call_count(), 1);
    let executor = fixture.executor.borrow();
    assert_eq!(executor.delivery_count, 1);
    assert_eq!(
        executor.endpoints,
        vec![
            "https://zoneminder.home/cgi-bin/nph-zms?mode=single&monitor=7&scale=100&token=secret.jwt_value.signature"
        ]
    );
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn denial_happens_before_credentials_login_or_delivery() {
    let mut fixture = fixture(false, false);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ZoneMinderSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, ZoneMinderSnapshotHostError::Media(_)));
    assert_eq!(fixture.credentials.resolve_count(), 0);
    assert_eq!(fixture.tokens.call_count(), 0);
    assert_eq!(fixture.executor.borrow().delivery_count, 0);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn failed_delivery_still_removes_the_token_bearing_endpoint() {
    let mut fixture = fixture(true, true);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ZoneMinderSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, ZoneMinderSnapshotHostError::Media(_)));
    assert_eq!(fixture.credentials.resolve_count(), 1);
    assert_eq!(fixture.tokens.call_count(), 1);
    assert_eq!(fixture.executor.borrow().delivery_count, 1);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn invalid_requests_and_targets_fail_before_secret_resolution() {
    let mut fixture = fixture(true, false);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ZoneMinderSnapshotRequest::new(fixture.entity_id.clone(), " ", 5_000),
        )
        .unwrap_err();
    assert_eq!(error, ZoneMinderSnapshotHostError::InvalidRequest);

    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ZoneMinderSnapshotRequest::new(
                smart_home_core::EntityId::trusted("zoneminder:monitor:8:camera"),
                "operator preview",
                5_000,
            ),
        )
        .unwrap_err();
    assert!(matches!(error, ZoneMinderSnapshotHostError::Media(_)));
    assert_eq!(fixture.credentials.resolve_count(), 0);
    assert_eq!(fixture.tokens.call_count(), 0);
}

#[test]
fn endpoint_configuration_rejects_credentials_queries_and_unpinned_hosts() {
    let bridge_id = BridgeId::trusted("bridge:zoneminder:test");
    let pinned =
        CameraMediaConnectionTarget::new("zoneminder.home", "192.0.2.10:443".parse().unwrap());
    assert!(ZoneMinderSnapshotEndpoint::new(
        bridge_id.clone(),
        "https://operator:secret@zoneminder.home/cgi-bin/nph-zms",
        pinned.clone(),
    )
    .is_err());
    assert!(ZoneMinderSnapshotEndpoint::new(
        bridge_id.clone(),
        "https://zoneminder.home/cgi-bin/nph-zms?token=old",
        pinned.clone(),
    )
    .is_err());
    assert!(ZoneMinderSnapshotEndpoint::new(
        bridge_id,
        "https://other.home/cgi-bin/nph-zms",
        pinned,
    )
    .is_err());
}

#[test]
fn invalid_credential_payload_is_redacted_and_never_logged_in() {
    let mut fixture = fixture(true, false);
    let secret = "raw-zone-password";
    fixture.credentials = TestCredentials::new(LeasePayload::new(
        format!(
            r#"{{"schema_version":1,"username":"operator","password":"{secret}","extra":true}}"#
        )
        .into_bytes(),
    ));
    let endpoint = ZoneMinderSnapshotEndpoint::new(
        BridgeId::trusted("bridge:zoneminder:test"),
        "https://zoneminder.home/cgi-bin/nph-zms",
        CameraMediaConnectionTarget::new("zoneminder.home", "192.0.2.10:443".parse().unwrap()),
    )
    .unwrap();
    fixture.host = ZoneMinderSnapshotHost::new(
        CameraMediaPolicy {
            allow_plaintext_loopback: true,
            ..CameraMediaPolicy::default()
        },
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(AgentId::trusted("operator"))),
        TestExecutor(fixture.executor.clone()),
        ZoneMinderSnapshotResources::new(
            fixture.credentials.clone(),
            fixture.tokens.clone(),
            endpoint,
        ),
    );

    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            ZoneMinderSnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();
    let diagnostics = format!("{error:?} {error}");
    assert_eq!(error, ZoneMinderSnapshotHostError::InvalidCredentialPayload);
    assert!(!diagnostics.contains(secret));
    assert_eq!(fixture.tokens.call_count(), 0);
}

#[test]
fn sealed_vault_record_supports_repeated_independently_authorized_deliveries() {
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
                salt_override: Some(vec![7; 16]),
            },
        )
        .unwrap();
    let payload = encode_zoneminder_credentials("operator", "secret").unwrap();
    vault
        .put(
            ZONEMINDER_VAULT_NAMESPACE,
            "bridge/main",
            payload.as_bytes(),
            None,
        )
        .unwrap();

    let bridge_id = BridgeId::trusted("bridge:zoneminder:test");
    let vault_ref = VaultRef::trusted(format!("{ZONEMINDER_VAULT_REF_PREFIX}bridge/main"));
    let (runtime, entity_id, principal_id) = fixture_runtime(
        true,
        bridge_id.clone(),
        vault_ref,
        "https://zoneminder.home/zm",
    );
    let tokens = TestTokens::new();
    let executor = Rc::new(RefCell::new(ExecutorState::default()));
    let endpoint = ZoneMinderSnapshotEndpoint::new(
        bridge_id,
        "https://zoneminder.home/cgi-bin/nph-zms",
        CameraMediaConnectionTarget::new("zoneminder.home", "192.0.2.10:443".parse().unwrap()),
    )
    .unwrap();
    let mut host = ZoneMinderSnapshotHost::new(
        CameraMediaPolicy {
            allow_plaintext_loopback: true,
            ..CameraMediaPolicy::default()
        },
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(executor.clone()),
        ZoneMinderSnapshotResources::new(
            ZoneMinderSealedStoreCredentialSource::new(vault),
            tokens.clone(),
            endpoint,
        ),
    );

    for purpose in ["first preview", "second preview"] {
        host.deliver_snapshot(
            &runtime,
            ZoneMinderSnapshotRequest::new(entity_id.clone(), purpose, 5_000),
        )
        .unwrap();
    }
    assert_eq!(tokens.call_count(), 2);
    assert_eq!(executor.borrow().delivery_count, 2);
    assert_eq!(host.media_snapshot().endpoint_count, 0);
}

#[test]
fn strict_native_transports_login_then_fetch_exactly_one_jpeg() {
    let login_capture = Arc::new(Mutex::new(TlsCapture::default()));
    let media_capture = Arc::new(Mutex::new(TlsCapture::default()));
    let login_body = br#"{"access_token":"single.jwt.value","access_token_expires":60,"refresh_token":"unused.refresh","refresh_token_expires":3600,"apiversion":"2.0"}"#;
    let bridge_id = BridgeId::trusted("bridge:zoneminder:test");
    let (runtime, entity_id, principal_id) = fixture_runtime(
        true,
        bridge_id.clone(),
        VaultRef::trusted("vault://fixture/zoneminder"),
        "https://zoneminder.home/zm",
    );
    let endpoint = ZoneMinderSnapshotEndpoint::new(
        bridge_id,
        "https://zoneminder.home/cgi-bin/nph-zms",
        CameraMediaConnectionTarget::new("zoneminder.home", "192.0.2.10:443".parse().unwrap()),
    )
    .unwrap();
    let executor = CameraMediaHttpExecutor::new(
        Box::new(RecordingConnector {
            response: wire_response("image/jpeg", &[0xff, 0xd8, 0xff, 0xd9]),
            capture: Arc::clone(&media_capture),
        }),
        TlsConfig::https_default(),
        CameraMediaHttpPolicy::default(),
    );
    let token_transport = ZoneMinderLanTransport::new(
        Box::new(RecordingConnector {
            response: wire_response("application/json", login_body),
            capture: Arc::clone(&login_capture),
        }),
        TlsConfig::https_default(),
    );
    let mut host = ZoneMinderSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        executor,
        ZoneMinderSnapshotResources::new(
            TestCredentials::new(encode_zoneminder_credentials("operator", "secret").unwrap()),
            ZoneMinderLanAccessTokenSource::new(token_transport),
            endpoint,
        ),
    );

    let delivery = host
        .deliver_snapshot(
            &runtime,
            ZoneMinderSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap();
    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(host.media_snapshot().endpoint_count, 0);
    let login = login_capture.lock().unwrap();
    let login_request = String::from_utf8(login.request.clone()).unwrap();
    assert!(login_request.starts_with("POST /zm/api/host/login.json HTTP/1.1"));
    assert!(login_request.ends_with("user=operator&pass=secret"));
    assert_eq!(
        login.host_connections,
        vec![("zoneminder.home".to_string(), 443)]
    );
    drop(login);
    let media = media_capture.lock().unwrap();
    let media_request = String::from_utf8(media.request.clone()).unwrap();
    assert!(media_request.starts_with(
        "GET /cgi-bin/nph-zms?mode=single&monitor=7&scale=100&token=single.jwt.value HTTP/1.1"
    ));
    assert_eq!(
        media.pinned_connections,
        vec![(
            "zoneminder.home".to_string(),
            "192.0.2.10:443".parse().unwrap()
        )]
    );
}

#[derive(Default)]
struct TlsCapture {
    host_connections: Vec<(String, u16)>,
    pinned_connections: Vec<(String, SocketAddr)>,
    request: Vec<u8>,
}

struct RecordingConnector {
    response: Vec<u8>,
    capture: Arc<Mutex<TlsCapture>>,
}

impl TlsConnector for RecordingConnector {
    fn connect(
        &self,
        host: &str,
        port: u16,
        _config: &TlsConfig,
    ) -> Result<Box<dyn TlsStream>, TlsError> {
        self.capture
            .lock()
            .unwrap()
            .host_connections
            .push((host.to_string(), port));
        Ok(Box::new(RecordingTlsStream {
            response: Cursor::new(self.response.clone()),
            capture: Arc::clone(&self.capture),
        }))
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
        Ok(Box::new(RecordingTlsStream {
            response: Cursor::new(self.response.clone()),
            capture: Arc::clone(&self.capture),
        }))
    }
}

struct RecordingTlsStream {
    response: Cursor<Vec<u8>>,
    capture: Arc<Mutex<TlsCapture>>,
}

impl Read for RecordingTlsStream {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        self.response.read(output)
    }
}

impl Write for RecordingTlsStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.capture
            .lock()
            .unwrap()
            .request
            .extend_from_slice(bytes);
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
