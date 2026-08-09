use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::{InitOptions, SealedStore};
use coding_adventures_zeroize::Zeroizing;
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
use smart_home_synology_snapshot_host::{
    encode_synology_credentials, SynologyCredentialSource, SynologyCredentialSourceError,
    SynologyLanSnapshotSessionSource, SynologySealedStoreCredentialSource,
    SynologySnapshotEndpoint, SynologySnapshotHost, SynologySnapshotHostError,
    SynologySnapshotRequest, SynologySnapshotResources, SynologySnapshotSessionSource,
    SynologySnapshotSessionSourceError, SYNOLOGY_VAULT_NAMESPACE, SYNOLOGY_VAULT_REF_PREFIX,
};
use smart_home_synology_surveillance_integration::{
    install_snapshot, SynologyCamera, SynologyConfig, SynologyCredentials, SynologyLanTransport,
    SynologySnapshot, SynologySurveillanceInfo,
};
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

impl SynologyCredentialSource for TestCredentials {
    fn resolve(
        &self,
        _vault_ref: &VaultRef,
    ) -> Result<LeasePayload, SynologyCredentialSourceError> {
        self.state
            .resolve_count
            .set(self.state.resolve_count.get() + 1);
        Ok(self.state.payload.clone())
    }
}

struct TestSession(Zeroizing<String>);

#[derive(Clone)]
struct TestSessions {
    state: Rc<SessionState>,
}

struct SessionState {
    open_count: Cell<usize>,
    close_count: Cell<usize>,
    fail_close: Cell<bool>,
    endpoint: RefCell<String>,
}

impl TestSessions {
    fn new(endpoint: impl Into<String>) -> Self {
        Self {
            state: Rc::new(SessionState {
                open_count: Cell::new(0),
                close_count: Cell::new(0),
                fail_close: Cell::new(false),
                endpoint: RefCell::new(endpoint.into()),
            }),
        }
    }
}

impl SynologySnapshotSessionSource for TestSessions {
    type Session = TestSession;

    fn open(
        &mut self,
        config: &SynologyConfig,
        credentials: &SynologyCredentials,
        camera_id: u64,
    ) -> Result<Self::Session, SynologySnapshotSessionSourceError> {
        assert_eq!(config.base_url, "https://diskstation.home:5001");
        assert_eq!(camera_id, 20);
        assert_eq!(
            format!("{credentials:?}"),
            "SynologyCredentials([REDACTED])"
        );
        self.state.open_count.set(self.state.open_count.get() + 1);
        Ok(TestSession(Zeroizing::new(
            self.state.endpoint.borrow().clone(),
        )))
    }

    fn endpoint_uri<'a>(&self, session: &'a Self::Session) -> &'a str {
        session.0.as_str()
    }

    fn close(
        &mut self,
        _config: &SynologyConfig,
        _session: Self::Session,
    ) -> Result<(), SynologySnapshotSessionSourceError> {
        self.state.close_count.set(self.state.close_count.get() + 1);
        if self.state.fail_close.get() {
            Err(SynologySnapshotSessionSourceError)
        } else {
            Ok(())
        }
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

type TestHost = SynologySnapshotHost<
    FixedClock,
    TestNonce,
    FixedPrincipal,
    TestExecutor,
    TestCredentials,
    TestSessions,
>;

struct Fixture {
    runtime: SmartHomeRuntime,
    entity_id: smart_home_core::EntityId,
    credentials: TestCredentials,
    sessions: TestSessions,
    executor: Rc<RefCell<ExecutorState>>,
    host: TestHost,
}

fn fixture(granted: bool, fail_delivery: bool) -> Fixture {
    let bridge_id = BridgeId::trusted("bridge:synology:test");
    let (runtime, entity_id, principal_id) = fixture_runtime(
        granted,
        bridge_id.clone(),
        VaultRef::trusted("vault://fixture/synology"),
        "https://diskstation.home:5001",
    );
    let credentials =
        TestCredentials::new(encode_synology_credentials("operator", "secret").unwrap());
    let sessions = TestSessions::new(
        "https://diskstation.home:5001/webapi/entry.cgi?api=SYNO.SurveillanceStation.Camera&method=GetSnapshot&version=9&id=20&profileType=0&_sid=secret.sid&SynoToken=secret-token",
    );
    let executor = Rc::new(RefCell::new(ExecutorState {
        fail_delivery,
        ..ExecutorState::default()
    }));
    let endpoint = SynologySnapshotEndpoint::new(
        bridge_id,
        CameraMediaConnectionTarget::new("diskstation.home", "192.0.2.10:5001".parse().unwrap()),
    );
    let host = SynologySnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(executor.clone()),
        SynologySnapshotResources::new(credentials.clone(), sessions.clone(), endpoint),
    );
    Fixture {
        runtime,
        entity_id,
        credentials,
        sessions,
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
    let config = SynologyConfig::new(bridge_id, base_url, vault_ref).unwrap();
    let snapshot = SynologySnapshot {
        info: SynologySurveillanceInfo {
            version: "9.2-11289".to_string(),
            camera_count: 1,
            maximum_camera_count: Some(40),
            user_privilege: Some(4),
            allow_snapshot: Some(true),
            allow_manual_recording: Some(false),
        },
        cameras: vec![SynologyCamera {
            id: 20,
            name: "Front".to_string(),
            vendor: "Synology".to_string(),
            model: "BC500".to_string(),
            channel: Some("1".to_string()),
            status: 1,
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
                CapabilityGrantId::trusted("grant:synology:snapshot"),
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
fn authorized_delivery_uses_one_session_and_removes_the_endpoint_before_logout() {
    let mut fixture = fixture(true, false);
    let delivery = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            SynologySnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap();

    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(fixture.credentials.resolve_count(), 1);
    assert_eq!(fixture.sessions.state.open_count.get(), 1);
    assert_eq!(fixture.sessions.state.close_count.get(), 1);
    assert_eq!(fixture.executor.borrow().delivery_count, 1);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn denial_happens_before_vault_session_or_delivery() {
    let mut fixture = fixture(false, false);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            SynologySnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, SynologySnapshotHostError::Media(_)));
    assert_eq!(fixture.credentials.resolve_count(), 0);
    assert_eq!(fixture.sessions.state.open_count.get(), 0);
    assert_eq!(fixture.executor.borrow().delivery_count, 0);
}

#[test]
fn failed_delivery_still_removes_endpoint_and_logs_out() {
    let mut fixture = fixture(true, true);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            SynologySnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, SynologySnapshotHostError::Media(_)));
    assert_eq!(fixture.sessions.state.close_count.get(), 1);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn invalid_endpoint_registration_still_logs_out() {
    let mut fixture = fixture(true, false);
    *fixture.sessions.state.endpoint.borrow_mut() =
        "https://other.home/snapshot?_sid=secret".to_string();
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            SynologySnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();

    assert_eq!(
        error,
        SynologySnapshotHostError::EndpointRegistrationRejected
    );
    assert_eq!(fixture.sessions.state.close_count.get(), 1);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn logout_failure_is_reported_after_endpoint_removal() {
    let mut fixture = fixture(true, false);
    fixture.sessions.state.fail_close.set(true);
    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            SynologySnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();

    assert_eq!(error, SynologySnapshotHostError::SessionLogoutFailed);
    assert_eq!(fixture.host.media_snapshot().endpoint_count, 0);
}

#[test]
fn malformed_credentials_are_redacted_and_never_open_a_session() {
    let mut fixture = fixture(true, false);
    let secret = "raw-synology-password";
    fixture.credentials = TestCredentials::new(LeasePayload::new(
        format!(
            r#"{{"schema_version":1,"username":"operator","password":"{secret}","extra":true}}"#
        )
        .into_bytes(),
    ));
    fixture.host = SynologySnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(AgentId::trusted("operator"))),
        TestExecutor(fixture.executor.clone()),
        SynologySnapshotResources::new(
            fixture.credentials.clone(),
            fixture.sessions.clone(),
            SynologySnapshotEndpoint::new(
                BridgeId::trusted("bridge:synology:test"),
                CameraMediaConnectionTarget::new(
                    "diskstation.home",
                    "192.0.2.10:5001".parse().unwrap(),
                ),
            ),
        ),
    );

    let error = fixture
        .host
        .deliver_snapshot(
            &fixture.runtime,
            SynologySnapshotRequest::new(fixture.entity_id.clone(), "operator preview", 5_000),
        )
        .unwrap_err();
    let diagnostics = format!("{error:?} {error}");
    assert_eq!(error, SynologySnapshotHostError::InvalidCredentialPayload);
    assert!(!diagnostics.contains(secret));
    assert_eq!(fixture.sessions.state.open_count.get(), 0);
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
    let payload = encode_synology_credentials("operator", "secret").unwrap();
    vault
        .put(
            SYNOLOGY_VAULT_NAMESPACE,
            "bridge/main",
            payload.as_bytes(),
            None,
        )
        .unwrap();

    let bridge_id = BridgeId::trusted("bridge:synology:test");
    let (runtime, entity_id, principal_id) = fixture_runtime(
        true,
        bridge_id.clone(),
        VaultRef::trusted(format!("{SYNOLOGY_VAULT_REF_PREFIX}bridge/main")),
        "https://diskstation.home:5001",
    );
    let sessions = TestSessions::new(
        "https://diskstation.home:5001/webapi/entry.cgi?api=SYNO.SurveillanceStation.Camera&method=GetSnapshot&version=9&id=20&profileType=0&_sid=secret.sid",
    );
    let executor = Rc::new(RefCell::new(ExecutorState::default()));
    let mut host = SynologySnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(executor.clone()),
        SynologySnapshotResources::new(
            SynologySealedStoreCredentialSource::new(vault),
            sessions.clone(),
            SynologySnapshotEndpoint::new(
                bridge_id,
                CameraMediaConnectionTarget::new(
                    "diskstation.home",
                    "192.0.2.10:5001".parse().unwrap(),
                ),
            ),
        ),
    );

    for purpose in ["first preview", "second preview"] {
        host.deliver_snapshot(
            &runtime,
            SynologySnapshotRequest::new(entity_id.clone(), purpose, 5_000),
        )
        .unwrap();
    }
    assert_eq!(sessions.state.open_count.get(), 2);
    assert_eq!(sessions.state.close_count.get(), 2);
    assert_eq!(executor.borrow().delivery_count, 2);
    assert_eq!(host.media_snapshot().endpoint_count, 0);
}

#[test]
fn strict_native_transports_revalidate_camera_fetch_one_jpeg_and_logout() {
    let session_capture = Arc::new(Mutex::new(TlsCapture::default()));
    let media_capture = Arc::new(Mutex::new(TlsCapture::default()));
    let session_responses = VecDeque::from(vec![
        json_response(br#"{"success":true,"data":{"SYNO.API.Auth":{"path":"auth.cgi","minVersion":1,"maxVersion":6},"SYNO.SurveillanceStation.Info":{"path":"entry.cgi","minVersion":1,"maxVersion":5},"SYNO.SurveillanceStation.Camera":{"path":"entry.cgi","minVersion":1,"maxVersion":9}}}"#),
        json_response(br#"{"success":true,"data":{"sid":"secret.sid.value","synotoken":"secret-token"}}"#),
        json_response(br#"{"success":true,"data":{"version":{"major":9,"minor":2,"build":11289},"cameraNumber":1,"maxCameraSupport":40,"userPriv":4,"allowSnapshot":true,"allowManualRec":false}}"#),
        json_response(br#"{"success":true,"data":{"total":1,"cameras":[{"id":20,"name":"Front","vendor":"Synology","model":"BC500","channel":"1","status":1}]}}"#),
        json_response(br#"{"success":true}"#),
    ]);
    let bridge_id = BridgeId::trusted("bridge:synology:test");
    let (runtime, entity_id, principal_id) = fixture_runtime(
        true,
        bridge_id.clone(),
        VaultRef::trusted("vault://fixture/synology"),
        "https://diskstation.home:5001",
    );
    let session_transport = SynologyLanTransport::new(
        Box::new(RecordingConnector::new(
            session_responses,
            Arc::clone(&session_capture),
        )),
        TlsConfig::https_default(),
    );
    let executor = CameraMediaHttpExecutor::new(
        Box::new(RecordingConnector::new(
            VecDeque::from(vec![wire_response("image/jpeg", &[0xff, 0xd8, 0xff, 0xd9])]),
            Arc::clone(&media_capture),
        )),
        TlsConfig::https_default(),
        CameraMediaHttpPolicy::default(),
    );
    let mut host = SynologySnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        executor,
        SynologySnapshotResources::new(
            TestCredentials::new(encode_synology_credentials("operator", "secret").unwrap()),
            SynologyLanSnapshotSessionSource::new(session_transport),
            SynologySnapshotEndpoint::new(
                bridge_id,
                CameraMediaConnectionTarget::new(
                    "diskstation.home",
                    "192.0.2.10:5001".parse().unwrap(),
                ),
            ),
        ),
    );

    let delivery = host
        .deliver_snapshot(
            &runtime,
            SynologySnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap();
    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(host.media_snapshot().endpoint_count, 0);

    let session = session_capture.lock().unwrap();
    assert_eq!(session.requests.len(), 5);
    let login = String::from_utf8(session.requests[1].clone()).unwrap();
    assert!(login.contains("account=operator&passwd=secret"));
    let list = String::from_utf8(session.requests[3].clone()).unwrap();
    assert!(list.contains("method=List"));
    assert!(list.contains("blPrivilege=true"));
    let logout = String::from_utf8(session.requests[4].clone()).unwrap();
    assert!(logout.contains("method=logout"));
    assert!(logout.contains("_sid=secret.sid.value"));
    drop(session);

    let media = media_capture.lock().unwrap();
    assert_eq!(media.requests.len(), 1);
    let request = String::from_utf8(media.requests[0].clone()).unwrap();
    assert!(request.contains("method=GetSnapshot"));
    assert!(request.contains("id=20"));
    assert!(request.contains("profileType=0"));
    assert!(request.contains("_sid=secret.sid.value"));
    assert!(request.contains("SynoToken=secret-token"));
    assert_eq!(
        media.pinned_connections,
        vec![(
            "diskstation.home".to_string(),
            "192.0.2.10:5001".parse().unwrap()
        )]
    );
}

#[derive(Default)]
struct TlsCapture {
    host_connections: Vec<(String, u16)>,
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

impl RecordingConnector {
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

fn json_response(body: &[u8]) -> Vec<u8> {
    wire_response("application/json", body)
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
