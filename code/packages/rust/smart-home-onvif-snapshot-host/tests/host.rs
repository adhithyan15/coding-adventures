use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::{InitOptions, SealedStore};
use smart_home_camera_media::{
    CameraMediaClock, CameraMediaConnectionTarget, CameraMediaCredentialRegistry,
    CameraMediaEndpointRegistry, CameraMediaExecution, CameraMediaExecutionError,
    CameraMediaExecutionResult, CameraMediaExecutor, CameraMediaKind, CameraMediaNonceError,
    CameraMediaNonceSource, CameraMediaPolicy, CameraMediaPrincipalSource,
};
use smart_home_camera_media_http_executor::{
    CameraMediaHttpCredentialError, CameraMediaHttpCredentials, CameraMediaHttpExecutor,
    CameraMediaHttpPolicy,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId,
    CapabilityMode, Device, DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId,
    PrivilegeTier, ValueKind, VaultRef,
};
use smart_home_onvif_snapshot_host::{
    encode_onvif_credentials, OnvifCredentialSource, OnvifCredentialSourceError,
    OnvifSealedStoreCredentialSource, OnvifSnapshotHost, OnvifSnapshotHostError,
    OnvifSnapshotRequest, ONVIF_VAULT_NAMESPACE, ONVIF_VAULT_REF_PREFIX,
};
use smart_home_runtime::SmartHomeRuntime;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use storage_core::{InMemoryStorageBackend, StorageBackend};
use tls_platform::{default_connector, TlsConfig};

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
    state: Rc<LeaseState>,
}

struct LeaseState {
    resolve_count: Cell<usize>,
    payload: LeasePayload,
}

impl TestCredentials {
    fn new(payload: LeasePayload) -> Self {
        Self {
            state: Rc::new(LeaseState {
                resolve_count: Cell::new(0),
                payload,
            }),
        }
    }

    fn resolve_count(&self) -> usize {
        self.state.resolve_count.get()
    }
}

impl OnvifCredentialSource for TestCredentials {
    fn resolve(&self, _vault_ref: &VaultRef) -> Result<LeasePayload, OnvifCredentialSourceError> {
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
    fail_delivery: bool,
}

struct TestExecutor(Rc<RefCell<ExecutorState>>);

type TestHost =
    OnvifSnapshotHost<FixedClock, TestNonce, FixedPrincipal, TestExecutor, TestCredentials>;
type TestHostFixture = (
    SmartHomeRuntime,
    EntityId,
    TestCredentials,
    Rc<RefCell<ExecutorState>>,
    TestHost,
);

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
        state.delivery_count += 1;
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

fn fixture_runtime(granted: bool) -> (SmartHomeRuntime, EntityId, AgentId) {
    let mut runtime = SmartHomeRuntime::new();
    let bridge_id = BridgeId::trusted("bridge:onvif:test");
    let device_id = DeviceId::trusted("onvif:test-camera");
    let entity_id = EntityId::trusted("onvif:test-camera:main");
    let principal_id = AgentId::trusted("operator");
    let mut bridge = Bridge::new(
        bridge_id.clone(),
        IntegrationId::trusted("onvif"),
        BridgeTransport::LanHttp,
    );
    bridge.auth_ref = Some(VaultRef::trusted("vault-lease:fixture"));
    runtime.upsert_bridge(bridge).unwrap();
    runtime
        .upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id,
            manufacturer: "Fixture".to_string(),
            model: "Camera".to_string(),
            name: "Camera".to_string(),
            serial: None,
            firmware_version: None,
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
            name: "Main profile".to_string(),
            capabilities: vec![Capability::new(
                CameraMediaKind::Snapshot.capability_id(),
                CapabilityMode::Command,
                ValueKind::Text,
            )],
            state: None,
            metadata: Vec::new(),
        })
        .unwrap();
    if granted {
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("grant:snapshot"),
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

fn test_host(granted: bool, fail_delivery: bool) -> TestHostFixture {
    let (runtime, entity_id, principal_id) = fixture_runtime(granted);
    let credentials =
        TestCredentials::new(encode_onvif_credentials("camera-user", "camera-secret").unwrap());
    let state = Rc::new(RefCell::new(ExecutorState {
        fail_delivery,
        ..ExecutorState::default()
    }));
    let policy = CameraMediaPolicy {
        allow_plaintext_loopback: true,
        ..CameraMediaPolicy::default()
    };
    let mut host = OnvifSnapshotHost::new(
        policy,
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        credentials.clone(),
    );
    host.register_pinned_camera_endpoint(
        entity_id.clone(),
        CameraMediaKind::Snapshot,
        "http://127.0.0.1:18080/snapshot",
        CameraMediaConnectionTarget::new("127.0.0.1", "127.0.0.1:18080".parse().unwrap()),
    )
    .unwrap();
    (runtime, entity_id, credentials, state, host)
}

#[test]
fn authorized_delivery_scopes_and_removes_credentials() {
    let (runtime, entity_id, credentials, state, mut host) = test_host(true, false);
    let delivery = host
        .deliver_snapshot(
            &runtime,
            OnvifSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap();

    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    assert_eq!(credentials.resolve_count(), 1);
    let state = state.borrow();
    assert_eq!(state.register_count, 1);
    assert_eq!(state.unregister_count, 1);
    assert_eq!(state.delivery_count, 1);
    assert!(state.registered.is_empty());
}

#[test]
fn denial_happens_before_credential_resolution_or_delivery() {
    let (runtime, entity_id, credentials, state, mut host) = test_host(false, false);
    let error = host
        .deliver_snapshot(
            &runtime,
            OnvifSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, OnvifSnapshotHostError::Media(_)));
    assert_eq!(credentials.resolve_count(), 0);
    let state = state.borrow();
    assert_eq!(state.register_count, 0);
    assert_eq!(state.delivery_count, 0);
}

#[test]
fn failed_delivery_still_removes_credentials() {
    let (runtime, entity_id, credentials, state, mut host) = test_host(true, true);
    let error = host
        .deliver_snapshot(
            &runtime,
            OnvifSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap_err();

    assert!(matches!(error, OnvifSnapshotHostError::Media(_)));
    assert_eq!(credentials.resolve_count(), 1);
    let state = state.borrow();
    assert_eq!(state.register_count, 1);
    assert_eq!(state.unregister_count, 1);
    assert!(state.registered.is_empty());
}

#[test]
fn invalid_request_does_not_resolve_credentials() {
    let (runtime, entity_id, credentials, state, mut host) = test_host(true, false);
    let error = host
        .deliver_snapshot(&runtime, OnvifSnapshotRequest::new(entity_id, " ", 5_000))
        .unwrap_err();

    assert_eq!(error, OnvifSnapshotHostError::InvalidRequest);
    assert_eq!(credentials.resolve_count(), 0);
    assert_eq!(state.borrow().register_count, 0);
}

#[test]
fn missing_endpoint_does_not_resolve_credentials() {
    let (runtime, entity_id, principal_id) = fixture_runtime(true);
    let credentials =
        TestCredentials::new(encode_onvif_credentials("camera-user", "camera-secret").unwrap());
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let mut host = OnvifSnapshotHost::new(
        CameraMediaPolicy::default(),
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        credentials.clone(),
    );

    let error = host
        .deliver_snapshot(
            &runtime,
            OnvifSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap_err();

    assert_eq!(error, OnvifSnapshotHostError::MissingEndpoint);
    assert_eq!(credentials.resolve_count(), 0);
    assert_eq!(state.borrow().register_count, 0);
}

#[test]
fn invalid_payload_is_redacted_and_never_registered() {
    let (runtime, entity_id, principal_id) = fixture_runtime(true);
    let secret = "raw-camera-password";
    let credentials = TestCredentials::new(LeasePayload::new(
        format!(r#"{{"username":"camera","password":"{secret}","extra":true}}"#).into_bytes(),
    ));
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let policy = CameraMediaPolicy {
        allow_plaintext_loopback: true,
        ..CameraMediaPolicy::default()
    };
    let mut host = OnvifSnapshotHost::new(
        policy,
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        credentials,
    );
    host.register_pinned_camera_endpoint(
        entity_id.clone(),
        CameraMediaKind::Snapshot,
        "http://127.0.0.1:18080/snapshot",
        CameraMediaConnectionTarget::new("127.0.0.1", "127.0.0.1:18080".parse().unwrap()),
    )
    .unwrap();

    let error = host
        .deliver_snapshot(
            &runtime,
            OnvifSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap_err();
    let diagnostics = format!("{error:?} {error}");
    assert!(!diagnostics.contains(secret));
    assert_eq!(state.borrow().register_count, 0);
}

#[test]
fn sealed_vault_reference_supports_repeated_independently_authorized_snapshots() {
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
    let payload = encode_onvif_credentials("camera-user", "camera-secret").unwrap();
    vault
        .put(
            ONVIF_VAULT_NAMESPACE,
            "bridge/main",
            payload.as_bytes(),
            None,
        )
        .unwrap();

    let (mut runtime, entity_id, principal_id) = fixture_runtime(true);
    let bridge_id = BridgeId::trusted("bridge:onvif:test");
    let mut bridge = runtime.registry().bridge(&bridge_id).unwrap().clone();
    bridge.auth_ref = Some(VaultRef::trusted(format!(
        "{ONVIF_VAULT_REF_PREFIX}bridge/main"
    )));
    runtime.upsert_bridge(bridge).unwrap();
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let policy = CameraMediaPolicy {
        allow_plaintext_loopback: true,
        ..CameraMediaPolicy::default()
    };
    let mut host = OnvifSnapshotHost::new(
        policy,
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        TestExecutor(state.clone()),
        OnvifSealedStoreCredentialSource::new(vault),
    );
    host.register_pinned_camera_endpoint(
        entity_id.clone(),
        CameraMediaKind::Snapshot,
        "http://127.0.0.1:18080/snapshot",
        CameraMediaConnectionTarget::new("127.0.0.1", "127.0.0.1:18080".parse().unwrap()),
    )
    .unwrap();

    for purpose in ["first preview", "second preview"] {
        host.deliver_snapshot(
            &runtime,
            OnvifSnapshotRequest::new(entity_id.clone(), purpose, 5_000),
        )
        .unwrap();
    }
    let state = state.borrow();
    assert_eq!(state.register_count, 2);
    assert_eq!(state.unregister_count, 2);
    assert_eq!(state.delivery_count, 2);
    assert!(state.registered.is_empty());
}

#[test]
fn native_http_executor_delivers_one_basic_authenticated_loopback_snapshot() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut first = listener.accept().unwrap().0;
        let first_request = read_request(&mut first);
        assert!(!first_request.contains("Authorization:"));
        first
            .write_all(
                b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"camera\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        drop(first);

        let mut second = listener.accept().unwrap().0;
        let second_request = read_request(&mut second);
        assert!(second_request.contains("Authorization: Basic dXNlcjpzZWNyZXQ="));
        second
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: 4\r\nConnection: close\r\n\r\n\xff\xd8\xff\xd9",
            )
            .unwrap();
    });

    let (runtime, entity_id, principal_id) = fixture_runtime(true);
    let credentials = TestCredentials::new(encode_onvif_credentials("user", "secret").unwrap());
    let media_policy = CameraMediaPolicy {
        allow_plaintext_loopback: true,
        ..CameraMediaPolicy::default()
    };
    let executor = CameraMediaHttpExecutor::new(
        default_connector(),
        TlsConfig::https_default(),
        CameraMediaHttpPolicy {
            allow_plaintext_loopback: true,
            ..CameraMediaHttpPolicy::default()
        },
    );
    let mut host = OnvifSnapshotHost::new(
        media_policy,
        FixedClock(10),
        TestNonce(1),
        FixedPrincipal(Some(principal_id)),
        executor,
        credentials,
    );
    let endpoint = format!("http://127.0.0.1:{}/snapshot", address.port());
    host.register_pinned_camera_endpoint(
        entity_id.clone(),
        CameraMediaKind::Snapshot,
        &endpoint,
        CameraMediaConnectionTarget::new("127.0.0.1", address),
    )
    .unwrap();

    let delivery = host
        .deliver_snapshot(
            &runtime,
            OnvifSnapshotRequest::new(entity_id, "operator preview", 5_000),
        )
        .unwrap();
    assert_eq!(
        delivery.snapshot_bytes(),
        Some(&[0xff, 0xd8, 0xff, 0xd9][..])
    );
    server.join().unwrap();
}

fn read_request(stream: &mut impl Read) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 512];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).unwrap();
        assert!(read > 0);
        bytes.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(bytes).unwrap()
}
