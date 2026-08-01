use smart_home_camera_media::{
    CameraMediaAccessRequest, CameraMediaClock, CameraMediaDelivery, CameraMediaError,
    CameraMediaExecution, CameraMediaExecutionError, CameraMediaExecutionResult,
    CameraMediaExecutor, CameraMediaKind, CameraMediaNonceError, CameraMediaNonceSource,
    CameraMediaPolicy, CameraMediaPrincipalSource, CameraMediaService,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId,
    CapabilityGrantStatus, CapabilityMode, Device, DeviceId, Entity, EntityId, EntityKind, Health,
    IntegrationId, Metadata, PrivilegeTier, ProtocolIdentifier, ValueKind,
};
use smart_home_runtime::SmartHomeRuntime;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
struct SharedClock(Rc<Cell<u64>>);

impl SharedClock {
    fn new(now_ms: u64) -> Self {
        Self(Rc::new(Cell::new(now_ms)))
    }

    fn set(&self, now_ms: u64) {
        self.0.set(now_ms);
    }
}

impl CameraMediaClock for SharedClock {
    fn now_ms(&self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone)]
struct SharedPrincipal(Rc<RefCell<Option<AgentId>>>);

impl SharedPrincipal {
    fn new(principal_id: AgentId) -> Self {
        Self(Rc::new(RefCell::new(Some(principal_id))))
    }

    fn set(&self, principal_id: Option<AgentId>) {
        *self.0.borrow_mut() = principal_id;
    }
}

impl CameraMediaPrincipalSource for SharedPrincipal {
    fn current_principal(&self) -> Option<AgentId> {
        self.0.borrow().clone()
    }
}

struct TestNonce {
    next: u8,
    calls: usize,
    fail_on_call: Option<usize>,
    repeat: bool,
}

impl TestNonce {
    fn sequence(next: u8) -> Self {
        Self {
            next,
            calls: 0,
            fail_on_call: None,
            repeat: false,
        }
    }

    fn failing() -> Self {
        Self {
            fail_on_call: Some(1),
            ..Self::sequence(0)
        }
    }

    fn repeating(next: u8) -> Self {
        Self {
            repeat: true,
            ..Self::sequence(next)
        }
    }
}

impl CameraMediaNonceSource for TestNonce {
    fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), CameraMediaNonceError> {
        self.calls += 1;
        if self.fail_on_call == Some(self.calls) {
            return Err(CameraMediaNonceError);
        }
        output.fill(self.next);
        if !self.repeat {
            self.next = self.next.wrapping_add(1);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ExecutorMode {
    Snapshot(usize),
    Stream,
    Failure(CameraMediaExecutionError),
    WrongKind,
}

#[derive(Default)]
struct ExecutorState {
    endpoints: Vec<String>,
    closed_streams: Vec<String>,
    execution_debug: Vec<String>,
    fail_close: bool,
}

struct TestExecutor {
    mode: ExecutorMode,
    state: Rc<RefCell<ExecutorState>>,
}

impl CameraMediaExecutor for TestExecutor {
    type Stream = String;

    fn deliver(
        &mut self,
        execution: CameraMediaExecution<'_>,
    ) -> Result<CameraMediaExecutionResult<Self::Stream>, CameraMediaExecutionError> {
        let endpoint = execution.endpoint_uri().to_owned();
        let debug = format!("{execution:?}");
        assert!(!debug.contains(&endpoint));
        let mut state = self.state.borrow_mut();
        state.endpoints.push(endpoint);
        state.execution_debug.push(debug);
        drop(state);
        match self.mode {
            ExecutorMode::Snapshot(byte_count) if execution.kind() == CameraMediaKind::Snapshot => {
                assert!(execution.max_snapshot_bytes() > 0);
                Ok(CameraMediaExecutionResult::snapshot(vec![0x5a; byte_count]))
            }
            ExecutorMode::Stream if execution.kind() == CameraMediaKind::Stream => Ok(
                CameraMediaExecutionResult::stream("executor-private-resource".to_string()),
            ),
            ExecutorMode::Failure(error) => Err(error),
            ExecutorMode::WrongKind if execution.kind() == CameraMediaKind::Snapshot => Ok(
                CameraMediaExecutionResult::stream("wrong-kind-resource".to_string()),
            ),
            ExecutorMode::WrongKind => Ok(CameraMediaExecutionResult::snapshot(vec![1])),
            _ => Ok(CameraMediaExecutionResult::snapshot(vec![1])),
        }
    }

    fn close_stream(&mut self, stream: &mut Self::Stream) -> Result<(), CameraMediaExecutionError> {
        let mut state = self.state.borrow_mut();
        if state.fail_close {
            return Err(CameraMediaExecutionError::Unavailable);
        }
        state.closed_streams.push(stream.clone());
        Ok(())
    }
}

type TestService = CameraMediaService<SharedClock, TestNonce, SharedPrincipal, TestExecutor>;

fn service(
    clock: &SharedClock,
    principal: &SharedPrincipal,
    nonce: TestNonce,
    mode: ExecutorMode,
    policy: CameraMediaPolicy,
) -> (TestService, Rc<RefCell<ExecutorState>>) {
    let state = Rc::new(RefCell::new(ExecutorState::default()));
    let executor = TestExecutor {
        mode,
        state: Rc::clone(&state),
    };
    (
        CameraMediaService::new(policy, clock.clone(), nonce, principal.clone(), executor),
        state,
    )
}

fn fixture_policy() -> CameraMediaPolicy {
    CameraMediaPolicy {
        allow_plaintext_loopback: true,
        ..CameraMediaPolicy::default()
    }
}

fn fixture_runtime(
    grant_expiry_ms: Option<u64>,
) -> (SmartHomeRuntime, EntityId, AgentId, CapabilityGrantId) {
    let mut runtime = SmartHomeRuntime::default();
    let bridge_id = BridgeId::trusted("camera-bridge");
    let device_id = DeviceId::trusted("camera-device");
    let entity_id = EntityId::trusted("camera-entity");
    let principal_id = AgentId::trusted("dashboard-user");
    let grant_id = CapabilityGrantId::trusted("camera-snapshot-grant");
    let mut bridge = Bridge::new(
        bridge_id.clone(),
        IntegrationId::trusted("onvif"),
        BridgeTransport::LanHttp,
    );
    bridge.health = Health::Online;
    runtime.upsert_bridge(bridge).unwrap();
    runtime
        .upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id,
            manufacturer: "Fixture".to_string(),
            model: "Camera".to_string(),
            name: "Front Door".to_string(),
            serial: Some("fixture-1".to_string()),
            firmware_version: None,
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: Vec::<ProtocolIdentifier>::new(),
            health: Health::Online,
            metadata: Vec::<Metadata>::new(),
        })
        .unwrap();
    runtime
        .upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id,
            kind: EntityKind::Camera,
            name: "Front Door".to_string(),
            capabilities: vec![
                Capability::new(
                    CameraMediaKind::Snapshot.capability_id(),
                    CapabilityMode::Command,
                    ValueKind::Text,
                ),
                Capability::new(
                    CameraMediaKind::Stream.capability_id(),
                    CapabilityMode::Command,
                    ValueKind::Text,
                ),
            ],
            state: None,
            metadata: Vec::new(),
        })
        .unwrap();
    let mut grant = CapabilityGrant::for_entity_capability(
        grant_id.clone(),
        principal_id.clone(),
        entity_id.clone(),
        CameraMediaKind::Snapshot.capability_id(),
        PrivilegeTier::HumanApproval,
        "user",
        1,
    );
    grant.expires_at_ms = grant_expiry_ms;
    runtime.registry_mut().upsert_capability_grant(grant);
    (runtime, entity_id, principal_id, grant_id)
}

fn request(entity_id: EntityId, kind: CameraMediaKind, ttl_ms: u64) -> CameraMediaAccessRequest {
    CameraMediaAccessRequest::new(entity_id, kind, "operator preview", ttl_ms)
}

#[test]
fn delivery_is_host_mediated_single_use_and_audits_never_expose_bearers() {
    let (runtime, entity_id, principal_id, _) = fixture_runtime(None);
    let clock = SharedClock::new(10);
    let principal = SharedPrincipal::new(principal_id);
    let (mut service, state) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x11),
        ExecutorMode::Snapshot(1_024),
        fixture_policy(),
    );
    let secret_uri = "https://camera.local/snapshot.jpg?token=secret";
    service
        .register_endpoint(entity_id.clone(), CameraMediaKind::Snapshot, secret_uri)
        .unwrap();
    let lease = service
        .issue_lease(&runtime, request(entity_id, CameraMediaKind::Snapshot, 500))
        .unwrap();
    let raw_lease_id = lease.lease_id.as_hex().to_owned();
    let delivery = service.deliver_lease(&runtime, &lease.lease_id).unwrap();
    assert_eq!(delivery.snapshot_bytes().unwrap().len(), 1_024);
    assert!(matches!(
        service.deliver_lease(&runtime, &lease.lease_id),
        Err(CameraMediaError::UnknownLease)
    ));
    assert_eq!(state.borrow().endpoints, vec![secret_uri]);
    let public = format!(
        "{delivery:?}{:?}",
        service.audit_records().collect::<Vec<_>>()
    );
    assert!(!public.contains(secret_uri));
    assert!(!public.contains(&raw_lease_id));
}

#[test]
fn trusted_identity_time_and_current_grant_are_rechecked() {
    let (mut runtime, entity_id, principal_id, grant_id) = fixture_runtime(Some(120));
    let clock = SharedClock::new(100);
    let principal = SharedPrincipal::new(principal_id.clone());
    let (mut service, _) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x21),
        ExecutorMode::Snapshot(2),
        fixture_policy(),
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "https://camera.local/snapshot.jpg",
        )
        .unwrap();
    let lease = service
        .issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Snapshot, 1_000),
        )
        .unwrap();
    assert_eq!(lease.expires_at_ms, 120);
    principal.set(Some(AgentId::trusted("forged-user")));
    assert!(matches!(
        service.deliver_lease(&runtime, &lease.lease_id),
        Err(CameraMediaError::LeasePrincipalMismatch)
    ));
    principal.set(Some(principal_id));
    runtime
        .registry_mut()
        .update_capability_grant_status(&grant_id, CapabilityGrantStatus::Revoked)
        .unwrap();
    assert!(matches!(
        service.deliver_lease(&runtime, &lease.lease_id),
        Err(CameraMediaError::Unauthorized { .. })
    ));
    assert_eq!(service.snapshot().active_lease_count, 0);
}

#[test]
fn expiry_prunes_before_quota_and_uses_only_the_installed_clock() {
    let (runtime, entity_id, principal_id, _) = fixture_runtime(None);
    let clock = SharedClock::new(10);
    let principal = SharedPrincipal::new(principal_id);
    let policy = CameraMediaPolicy {
        max_active_leases: 1,
        max_active_leases_per_principal: 1,
        ..fixture_policy()
    };
    let (mut service, _) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x31),
        ExecutorMode::Snapshot(1),
        policy,
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "https://camera.local/snapshot.jpg",
        )
        .unwrap();
    let first = service
        .issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Snapshot, 10),
        )
        .unwrap();
    assert!(matches!(
        service.issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Snapshot, 10)
        ),
        Err(CameraMediaError::LeaseQuotaExceeded { maximum: 1 })
    ));
    clock.set(first.expires_at_ms);
    let second = service
        .issue_lease(&runtime, request(entity_id, CameraMediaKind::Snapshot, 10))
        .unwrap();
    assert_ne!(first.lease_id, second.lease_id);
    assert_eq!(service.snapshot().active_lease_count, 1);
}

#[test]
fn endpoint_rotation_and_removal_consume_stale_leases() {
    let (runtime, entity_id, principal_id, _) = fixture_runtime(None);
    let clock = SharedClock::new(10);
    let principal = SharedPrincipal::new(principal_id);
    let (mut service, state) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x41),
        ExecutorMode::Snapshot(1),
        fixture_policy(),
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "https://camera.local/old",
        )
        .unwrap();
    let rotated = service
        .issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Snapshot, 100),
        )
        .unwrap();
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "https://camera.local/new",
        )
        .unwrap();
    assert!(matches!(
        service.deliver_lease(&runtime, &rotated.lease_id),
        Err(CameraMediaError::EndpointGenerationChanged)
    ));
    let removed = service
        .issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Snapshot, 100),
        )
        .unwrap();
    assert!(service.unregister_endpoint(&entity_id, CameraMediaKind::Snapshot));
    assert!(!service.unregister_endpoint(&entity_id, CameraMediaKind::Snapshot));
    assert!(matches!(
        service.deliver_lease(&runtime, &removed.lease_id),
        Err(CameraMediaError::MissingEndpoint { .. })
    ));
    assert!(state.borrow().endpoints.is_empty());
}

#[test]
fn endpoint_policy_is_secure_by_default_and_fixture_opt_in_is_explicit() {
    let (_, entity_id, principal_id, _) = fixture_runtime(None);
    let clock = SharedClock::new(10);
    let principal = SharedPrincipal::new(principal_id);
    let (mut secure, _) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x51),
        ExecutorMode::Snapshot(1),
        CameraMediaPolicy::default(),
    );
    assert!(matches!(
        secure.register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "http://127.0.0.1/snapshot"
        ),
        Err(CameraMediaError::InsecureEndpoint)
    ));
    let (mut fixtures, _) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x52),
        ExecutorMode::Snapshot(1),
        fixture_policy(),
    );
    fixtures
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "http://127.2.3.4/snapshot",
        )
        .unwrap();
    assert!(matches!(
        fixtures.register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "http://127.0.0.1/snapshot?token=plaintext"
        ),
        Err(CameraMediaError::InsecureEndpoint)
    ));
    for uri in [
        "http://127.evil.local.com/snapshot",
        "https://user:password@camera.local/snapshot",
        "https://camera.local/snapshot#secret",
        "ftp://camera.local/snapshot",
    ] {
        assert!(fixtures
            .register_endpoint(entity_id.clone(), CameraMediaKind::Snapshot, uri)
            .is_err());
    }
    fixtures
        .register_endpoint(
            entity_id,
            CameraMediaKind::Snapshot,
            "https://camera.local/snapshot?token=transport-protected",
        )
        .unwrap();
}

#[test]
fn nonce_failures_and_collisions_fail_closed() {
    let (runtime, entity_id, principal_id, _) = fixture_runtime(None);
    let clock = SharedClock::new(10);
    let principal = SharedPrincipal::new(principal_id);
    let (mut failing, _) = service(
        &clock,
        &principal,
        TestNonce::failing(),
        ExecutorMode::Snapshot(1),
        fixture_policy(),
    );
    failing
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "https://camera.local/snapshot",
        )
        .unwrap();
    assert!(matches!(
        failing.issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Snapshot, 10)
        ),
        Err(CameraMediaError::NonceUnavailable)
    ));

    let (mut repeating, _) = service(
        &clock,
        &principal,
        TestNonce::repeating(0x61),
        ExecutorMode::Snapshot(1),
        fixture_policy(),
    );
    repeating
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "https://camera.local/snapshot",
        )
        .unwrap();
    repeating
        .issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Snapshot, 10),
        )
        .unwrap();
    assert!(matches!(
        repeating.issue_lease(&runtime, request(entity_id, CameraMediaKind::Snapshot, 10)),
        Err(CameraMediaError::DuplicateLeaseId)
    ));
}

#[test]
fn executor_failure_oversize_and_wrong_kind_are_consumed_and_closed() {
    let (runtime, entity_id, principal_id, _) = fixture_runtime(None);
    for (mode, expected) in [
        (
            ExecutorMode::Failure(CameraMediaExecutionError::Protocol),
            "media transport protocol failure",
        ),
        (ExecutorMode::Snapshot(9), "invalid delivery"),
        (ExecutorMode::WrongKind, "invalid delivery"),
    ] {
        let clock = SharedClock::new(10);
        let principal = SharedPrincipal::new(principal_id.clone());
        let policy = CameraMediaPolicy {
            max_snapshot_bytes: 8,
            ..fixture_policy()
        };
        let (mut service, state) =
            service(&clock, &principal, TestNonce::sequence(0x71), mode, policy);
        service
            .register_endpoint(
                entity_id.clone(),
                CameraMediaKind::Snapshot,
                "https://camera.local/snapshot",
            )
            .unwrap();
        let lease = service
            .issue_lease(
                &runtime,
                request(entity_id.clone(), CameraMediaKind::Snapshot, 10),
            )
            .unwrap();
        let error = service
            .deliver_lease(&runtime, &lease.lease_id)
            .unwrap_err();
        assert!(error.to_string().contains(expected));
        assert!(matches!(
            service.deliver_lease(&runtime, &lease.lease_id),
            Err(CameraMediaError::UnknownLease)
        ));
        if matches!(mode, ExecutorMode::WrongKind) {
            assert_eq!(state.borrow().closed_streams, vec!["wrong-kind-resource"]);
        }
    }
}

#[test]
fn broker_mints_stream_ids_and_owns_close_and_expiry() {
    let (mut runtime, entity_id, principal_id, _) = fixture_runtime(None);
    runtime
        .registry_mut()
        .upsert_capability_grant(CapabilityGrant::for_entity_capability(
            CapabilityGrantId::trusted("camera-stream-grant"),
            principal_id.clone(),
            entity_id.clone(),
            CameraMediaKind::Stream.capability_id(),
            PrivilegeTier::HumanApproval,
            "user",
            1,
        ));
    let clock = SharedClock::new(20);
    let principal = SharedPrincipal::new(principal_id.clone());
    let (mut service, state) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x81),
        ExecutorMode::Stream,
        fixture_policy(),
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Stream,
            "rtsps://camera.local/live?token=secret",
        )
        .unwrap();
    let lease = service
        .issue_lease(&runtime, request(entity_id, CameraMediaKind::Stream, 50))
        .unwrap();
    let delivery = service.deliver_lease(&runtime, &lease.lease_id).unwrap();
    let CameraMediaDelivery::Stream {
        session_id,
        expires_at_ms,
    } = delivery
    else {
        panic!("expected stream delivery");
    };
    assert_eq!(expires_at_ms, 70);
    assert_eq!(session_id.as_str().len(), 32);
    assert!(!session_id.as_str().contains("executor-private-resource"));
    assert!(!format!("{session_id:?}").contains(session_id.as_str()));
    principal.set(Some(AgentId::trusted("other-user")));
    assert!(matches!(
        service.close_stream(&session_id),
        Err(CameraMediaError::StreamPrincipalMismatch)
    ));
    principal.set(Some(principal_id));
    service.close_stream(&session_id).unwrap();
    assert_eq!(
        state.borrow().closed_streams,
        vec!["executor-private-resource"]
    );
    assert!(matches!(
        service.close_stream(&session_id),
        Err(CameraMediaError::UnknownStreamSession)
    ));
}

#[test]
fn failed_stream_teardown_retains_ownership_for_retry() {
    let (mut runtime, entity_id, principal_id, _) = fixture_runtime(None);
    runtime
        .registry_mut()
        .upsert_capability_grant(CapabilityGrant::for_entity_capability(
            CapabilityGrantId::trusted("retry-stream-grant"),
            principal_id.clone(),
            entity_id.clone(),
            CameraMediaKind::Stream.capability_id(),
            PrivilegeTier::HumanApproval,
            "user",
            1,
        ));
    let clock = SharedClock::new(20);
    let principal = SharedPrincipal::new(principal_id);
    let (mut service, state) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x89),
        ExecutorMode::Stream,
        fixture_policy(),
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Stream,
            "rtsps://camera.local/live",
        )
        .unwrap();
    let lease = service
        .issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Stream, 10),
        )
        .unwrap();
    let CameraMediaDelivery::Stream { session_id, .. } =
        service.deliver_lease(&runtime, &lease.lease_id).unwrap()
    else {
        panic!("expected stream delivery");
    };
    state.borrow_mut().fail_close = true;
    assert!(matches!(
        service.close_stream(&session_id),
        Err(CameraMediaError::Execution(
            CameraMediaExecutionError::Unavailable
        ))
    ));
    assert_eq!(service.snapshot().active_stream_count, 1);
    clock.set(30);
    assert_eq!(
        service.reconcile(&runtime),
        smart_home_camera_media::CameraMediaReconcileReport {
            expired_lease_count: 0,
            closed_stream_count: 0,
            failed_stream_close_count: 1,
        }
    );
    assert_eq!(service.snapshot().active_stream_count, 1);
    state.borrow_mut().fail_close = false;
    assert_eq!(
        service.reconcile(&runtime),
        smart_home_camera_media::CameraMediaReconcileReport {
            expired_lease_count: 0,
            closed_stream_count: 1,
            failed_stream_close_count: 0,
        }
    );
    assert_eq!(service.snapshot().active_stream_count, 0);
}

#[test]
fn failed_post_open_cleanup_is_retained_without_a_public_session() {
    let (mut runtime, entity_id, principal_id, _) = fixture_runtime(None);
    runtime
        .registry_mut()
        .upsert_capability_grant(CapabilityGrant::for_entity_capability(
            CapabilityGrantId::trusted("cleanup-stream-grant"),
            principal_id.clone(),
            entity_id.clone(),
            CameraMediaKind::Stream.capability_id(),
            PrivilegeTier::HumanApproval,
            "user",
            1,
        ));
    let clock = SharedClock::new(20);
    let principal = SharedPrincipal::new(principal_id);
    let nonce = TestNonce {
        fail_on_call: Some(2),
        ..TestNonce::sequence(0x8d)
    };
    let (mut service, state) = service(
        &clock,
        &principal,
        nonce,
        ExecutorMode::Stream,
        fixture_policy(),
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Stream,
            "rtsps://camera.local/live",
        )
        .unwrap();
    let lease = service
        .issue_lease(&runtime, request(entity_id, CameraMediaKind::Stream, 10))
        .unwrap();
    state.borrow_mut().fail_close = true;
    assert!(matches!(
        service.deliver_lease(&runtime, &lease.lease_id),
        Err(CameraMediaError::Execution(
            CameraMediaExecutionError::Unavailable
        ))
    ));
    assert_eq!(service.snapshot().active_stream_count, 0);
    assert_eq!(service.snapshot().pending_stream_cleanup_count, 1);
    state.borrow_mut().fail_close = false;
    assert_eq!(
        service.reconcile(&runtime),
        smart_home_camera_media::CameraMediaReconcileReport {
            expired_lease_count: 0,
            closed_stream_count: 1,
            failed_stream_close_count: 0,
        }
    );
    assert_eq!(service.snapshot().pending_stream_cleanup_count, 0);
}

#[test]
fn stream_quota_and_expiry_cleanup_are_enforced() {
    let (mut runtime, entity_id, principal_id, _) = fixture_runtime(None);
    runtime
        .registry_mut()
        .upsert_capability_grant(CapabilityGrant::for_entity_capability(
            CapabilityGrantId::trusted("stream-grant"),
            principal_id.clone(),
            entity_id.clone(),
            CameraMediaKind::Stream.capability_id(),
            PrivilegeTier::HumanApproval,
            "user",
            1,
        ));
    let clock = SharedClock::new(10);
    let principal = SharedPrincipal::new(principal_id);
    let policy = CameraMediaPolicy {
        max_active_streams: 1,
        ..fixture_policy()
    };
    let (mut service, state) = service(
        &clock,
        &principal,
        TestNonce::sequence(0x91),
        ExecutorMode::Stream,
        policy,
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Stream,
            "rtsps://camera.local/live",
        )
        .unwrap();
    let first = service
        .issue_lease(
            &runtime,
            request(entity_id.clone(), CameraMediaKind::Stream, 20),
        )
        .unwrap();
    service.deliver_lease(&runtime, &first.lease_id).unwrap();
    let second = service
        .issue_lease(&runtime, request(entity_id, CameraMediaKind::Stream, 20))
        .unwrap();
    assert!(matches!(
        service.deliver_lease(&runtime, &second.lease_id),
        Err(CameraMediaError::StreamQuotaExceeded { maximum: 1 })
    ));
    clock.set(30);
    assert_eq!(
        service.reconcile(&runtime),
        smart_home_camera_media::CameraMediaReconcileReport {
            expired_lease_count: 0,
            closed_stream_count: 1,
            failed_stream_close_count: 0,
        }
    );
    assert_eq!(service.snapshot().active_stream_count, 0);
    assert_eq!(state.borrow().closed_streams.len(), 1);
}

#[test]
fn unauthenticated_hosts_requests_and_error_text_fail_closed() {
    let (runtime, entity_id, principal_id, _) = fixture_runtime(None);
    let clock = SharedClock::new(u64::MAX);
    let principal = SharedPrincipal::new(principal_id);
    principal.set(None);
    let (mut service, _) = service(
        &clock,
        &principal,
        TestNonce::sequence(0xa1),
        ExecutorMode::Snapshot(1),
        fixture_policy(),
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "https://camera.local/snapshot",
        )
        .unwrap();
    assert!(matches!(
        service.issue_lease(&runtime, request(entity_id, CameraMediaKind::Snapshot, 1)),
        Err(CameraMediaError::Unauthenticated)
    ));
    for error in [
        CameraMediaError::PrincipalLeaseQuotaExceeded { maximum: 1 },
        CameraMediaError::StreamQuotaExceeded { maximum: 1 },
        CameraMediaError::DuplicateStreamSessionId,
        CameraMediaError::UnknownStreamSession,
        CameraMediaError::StreamPrincipalMismatch,
        CameraMediaError::Unauthenticated,
    ] {
        let rendered = error.to_string();
        assert!(!rendered.contains("camera.local"));
        assert!(!rendered.is_empty());
    }
}

#[test]
fn bounded_audit_discards_oldest_records() {
    let (_, entity_id, principal_id, _) = fixture_runtime(None);
    let clock = SharedClock::new(10);
    let principal = SharedPrincipal::new(principal_id);
    let policy = CameraMediaPolicy {
        max_audit_records: 1,
        ..fixture_policy()
    };
    let (mut service, _) = service(
        &clock,
        &principal,
        TestNonce::sequence(0xb1),
        ExecutorMode::Snapshot(1),
        policy,
    );
    service
        .register_endpoint(
            entity_id.clone(),
            CameraMediaKind::Snapshot,
            "https://camera.local/one",
        )
        .unwrap();
    service
        .register_endpoint(
            entity_id,
            CameraMediaKind::Snapshot,
            "https://camera.local/two",
        )
        .unwrap();
    let records = service.audit_records().collect::<Vec<_>>();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].sequence, 2);
}
