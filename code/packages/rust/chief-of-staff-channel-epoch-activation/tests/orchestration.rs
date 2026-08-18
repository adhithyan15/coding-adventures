use chief_of_staff_channel_crypto::grant_profile::{
    grant_deserialize, plan_rotation, RotationReceiver,
};
use chief_of_staff_channel_crypto::wire::{key_grant_record_key, CHANNEL_STORAGE_NAMESPACE};
use chief_of_staff_channel_crypto::{
    decrypt_message, ChannelId, ChannelMasterKey, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair,
    Sequence,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_channel_epoch_activation::{
    activation_plan_record_key, prepare_rotation_candidate, ActivationOutcome,
    ActiveEpochAppendRequest, CustodySelection, EpochActivationStore, InMemoryKeyCustody,
    OriginatorKeyCustody, PreparationOutcome, ACTIVATION_PLAN_CONTENT_TYPE,
};
use chief_of_staff_channel_store::{AppendRequest, ChannelStore};
use coding_adventures_json_value::JsonValue;
use storage_core::{
    InMemoryStorageBackend, Revision, StorageBackend, StorageError, StorageLease,
    StorageListOptions, StoragePage, StoragePutInput, StorageRecord, StorageStat,
};

const CURRENT_CMK: [u8; 32] = [0x21; 32];
const NEXT_CMK: [u8; 32] = [0x31; 32];
const GRANT_CONTENT_TYPE: &str = "application/vnd.coding-adventures.chief-channel-key-grant-v1";

#[derive(Default)]
struct StateCasConflictBackend {
    inner: InMemoryStorageBackend,
    reject_state_cas: AtomicBool,
}

impl StateCasConflictBackend {
    fn reject_state_cas(&self) {
        self.reject_state_cas.store(true, Ordering::SeqCst);
    }
}

impl StorageBackend for StateCasConflictBackend {
    fn initialize(&self) -> Result<(), StorageError> {
        self.inner.initialize()
    }

    fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
        self.inner.get(namespace, key)
    }

    fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
        if self.reject_state_cas.load(Ordering::SeqCst)
            && input.key
                == chief_of_staff_channel_crypto::wire::sequence_state_record_key(channel_id())
            && input.if_revision.is_some()
        {
            return Err(StorageError::Conflict {
                namespace: input.namespace,
                key: input.key,
                expected_revision: input.if_revision.map(|revision| revision.to_string()),
                actual_revision: Some("forced-cas-race".to_owned()),
            });
        }
        self.inner.put(input)
    }

    fn delete(
        &self,
        namespace: &str,
        key: &str,
        if_revision: Option<&Revision>,
    ) -> Result<(), StorageError> {
        self.inner.delete(namespace, key, if_revision)
    }

    fn list(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<StoragePage, StorageError> {
        self.inner.list(namespace, options)
    }

    fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError> {
        self.inner.stat(namespace, key)
    }

    fn acquire_lease(&self, name: &str, ttl_ms: u64) -> Result<Option<StorageLease>, StorageError> {
        self.inner.acquire_lease(name, ttl_ms)
    }
}

struct Fixture {
    backend: InMemoryStorageBackend,
    custody: InMemoryKeyCustody,
    definition: ChannelDefinition,
    signer: OriginatorSigningKey,
    receiver_a: ReceiverIdentity,
    receiver_b: ReceiverIdentity,
}

impl Fixture {
    fn new() -> Self {
        let backend = InMemoryStorageBackend::new();
        let signer = OriginatorSigningKey::from_seed([0x11; 32]);
        let receiver_a_key = ReceiverKeyPair::from_private_key([0x41; 32]).unwrap();
        let receiver_b_key = ReceiverKeyPair::from_private_key([0x42; 32]).unwrap();
        let receiver_a = ReceiverIdentity {
            agent_id: AgentId::new(b"receiver-a".to_vec()).unwrap(),
            public_key: receiver_a_key.public_key(),
        };
        let receiver_b = ReceiverIdentity {
            agent_id: AgentId::new(b"receiver-b".to_vec()).unwrap(),
            public_key: receiver_b_key.public_key(),
        };
        let definition = ChannelDefinition::new(
            channel_id(),
            OriginatorIdentity {
                agent_id: AgentId::new(b"originator".to_vec()).unwrap(),
                public_key: signer.public_key(),
            },
            vec![receiver_a.clone(), receiver_b.clone()],
            1_725_000_000_000_000_000,
            KeyEpoch(0),
        )
        .unwrap();
        ChannelDefinitionStore::new(&backend)
            .create(&definition)
            .unwrap();
        Self {
            backend,
            custody: InMemoryKeyCustody::new(),
            definition,
            signer,
            receiver_a,
            receiver_b,
        }
    }

    fn store(&self) -> EpochActivationStore<'_, InMemoryKeyCustody> {
        EpochActivationStore::new_for_testing(&self.backend, &self.custody, channel_id()).unwrap()
    }

    fn rotation(
        &self,
        cmk: [u8; 32],
        material: u8,
    ) -> chief_of_staff_channel_crypto::grant_profile::RotationPlan {
        plan_rotation(
            self.definition.originator().agent_id.as_bytes(),
            channel_id(),
            KeyEpoch(0),
            ChannelMasterKey::from_bytes(cmk),
            vec![RotationReceiver::with_material(
                self.receiver_b.agent_id.as_bytes().to_vec(),
                self.receiver_b.public_key,
                [material; 32],
                [material.wrapping_add(1); 24],
            )
            .unwrap()],
            &self.signer,
        )
        .unwrap()
    }

    fn two_receiver_rotation(&self) -> chief_of_staff_channel_crypto::grant_profile::RotationPlan {
        plan_rotation(
            self.definition.originator().agent_id.as_bytes(),
            channel_id(),
            KeyEpoch(0),
            ChannelMasterKey::from_bytes(NEXT_CMK),
            vec![
                RotationReceiver::with_material(
                    self.receiver_b.agent_id.as_bytes().to_vec(),
                    self.receiver_b.public_key,
                    [0x52; 32],
                    [0x62; 24],
                )
                .unwrap(),
                RotationReceiver::with_material(
                    self.receiver_a.agent_id.as_bytes().to_vec(),
                    self.receiver_a.public_key,
                    [0x51; 32],
                    [0x61; 24],
                )
                .unwrap(),
            ],
            &self.signer,
        )
        .unwrap()
    }
}

fn channel_id() -> ChannelId {
    ChannelId([
        0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0,
    ])
}

fn message_id(byte: u8) -> [u8; 16] {
    let mut bytes = [byte; 16];
    bytes[6] = 0x70 | (byte & 0x0f);
    bytes[8] = 0x80 | (byte & 0x3f);
    bytes
}

#[test]
fn migrates_legacy_pending_state_without_changing_its_header() {
    let fixture = Fixture::new();
    let legacy = ChannelStore::new(&fixture.backend, channel_id());
    let pending = legacy
        .reserve_append(
            AppendRequest {
                message_id: message_id(1),
                timestamp_ns: 11,
                originator_id: b"originator".to_vec(),
                key_epoch: KeyEpoch(0),
                content_type: "text/plain".to_owned(),
            },
            b"already reserved",
        )
        .unwrap();

    let state = fixture
        .store()
        .migrate_epoch_state(
            &fixture.definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    assert_eq!(state.active_epoch(), KeyEpoch(0));
    assert_eq!(state.next_sequence(), Sequence(1));
    assert_eq!(state.pending_header(), Some(&pending));
    assert_eq!(
        fixture
            .store()
            .migrate_epoch_state(&fixture.definition, None)
            .unwrap(),
        state
    );
}

#[test]
fn selection_replay_activation_and_active_epoch_publish_are_idempotent() {
    let fixture = Fixture::new();
    let store = fixture.store();
    store
        .migrate_epoch_state(
            &fixture.definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    assert_eq!(
        store
            .prepare_rotation(
                &fixture.definition,
                std::slice::from_ref(&fixture.receiver_b),
                fixture.rotation(NEXT_CMK, 0x51),
            )
            .unwrap(),
        PreparationOutcome::Prepared
    );
    assert_eq!(
        store
            .prepare_rotation(
                &fixture.definition,
                std::slice::from_ref(&fixture.receiver_b),
                fixture.rotation(NEXT_CMK, 0x51),
            )
            .unwrap(),
        PreparationOutcome::Idempotent
    );
    assert_eq!(
        store
            .activate_prepared_epoch(&fixture.definition, KeyEpoch(1))
            .unwrap(),
        ActivationOutcome::Activated
    );
    assert_eq!(
        store
            .activate_prepared_epoch(&fixture.definition, KeyEpoch(1))
            .unwrap(),
        ActivationOutcome::Idempotent
    );

    let reservation = store
        .reserve_publish_using_active_epoch(
            &fixture.definition,
            ActiveEpochAppendRequest {
                message_id: message_id(2),
                timestamp_ns: 12,
                originator_id: b"originator".to_vec(),
                content_type: "text/plain".to_owned(),
            },
            b"epoch one",
        )
        .unwrap();
    assert_eq!(reservation.header().fields().key_epoch(), KeyEpoch(1));
    let message = store
        .commit_reserved(
            &fixture.definition,
            &reservation,
            b"epoch one",
            &fixture.signer,
        )
        .unwrap();
    assert_eq!(
        decrypt_message(
            &message,
            &ChannelMasterKey::from_bytes(NEXT_CMK),
            &fixture.signer.public_key(),
        )
        .unwrap(),
        b"epoch one"
    );
    assert_eq!(store.state().unwrap().active_epoch(), KeyEpoch(1));
    assert_eq!(store.state().unwrap().pending_header(), None);
}

#[test]
fn one_candidate_wins_and_a_pending_publish_blocks_activation() {
    let fixture = Fixture::new();
    let store = fixture.store();
    store
        .migrate_epoch_state(
            &fixture.definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    store
        .prepare_rotation(
            &fixture.definition,
            std::slice::from_ref(&fixture.receiver_b),
            fixture.rotation(NEXT_CMK, 0x51),
        )
        .unwrap();
    let conflict = store
        .prepare_rotation(
            &fixture.definition,
            std::slice::from_ref(&fixture.receiver_b),
            fixture.rotation([0x32; 32], 0x52),
        )
        .unwrap_err();
    assert_eq!(conflict.code(), "conflicting_preparation");

    let reservation = store
        .reserve_publish_using_active_epoch(
            &fixture.definition,
            ActiveEpochAppendRequest {
                message_id: message_id(3),
                timestamp_ns: 13,
                originator_id: b"originator".to_vec(),
                content_type: "text/plain".to_owned(),
            },
            b"epoch zero first",
        )
        .unwrap();
    assert_eq!(reservation.header().fields().key_epoch(), KeyEpoch(0));
    let error = store
        .activate_prepared_epoch(&fixture.definition, KeyEpoch(1))
        .unwrap_err();
    assert_eq!(error.code(), "pending_append");
    store
        .commit_reserved(
            &fixture.definition,
            &reservation,
            b"epoch zero first",
            &fixture.signer,
        )
        .unwrap();
    assert_eq!(
        store
            .activate_prepared_epoch(&fixture.definition, KeyEpoch(1))
            .unwrap(),
        ActivationOutcome::Activated
    );
}

#[test]
fn production_rejects_memory_custody_and_destruction_erases_all_keys() {
    let fixture = Fixture::new();
    let error = match EpochActivationStore::new(&fixture.backend, &fixture.custody, channel_id()) {
        Ok(_) => panic!("production accepted non-durable custody"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "custody_error");
    let store = fixture.store();
    store
        .migrate_epoch_state(
            &fixture.definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    store
        .prepare_rotation(
            &fixture.definition,
            std::slice::from_ref(&fixture.receiver_b),
            fixture.rotation(NEXT_CMK, 0x51),
        )
        .unwrap();
    assert_eq!(fixture.custody.retained_key_count().unwrap(), 2);
    let destroyed = ChannelDefinitionStore::new(&fixture.backend)
        .destroy(channel_id())
        .unwrap();
    store.apply_destruction(&destroyed).unwrap();
    assert_eq!(fixture.custody.retained_key_count().unwrap(), 0);
    assert_eq!(
        store.state().unwrap().active_epoch(),
        KeyEpoch(0),
        "public history remains after logical secret destruction"
    );
}

#[test]
fn every_public_replay_crash_boundary_converges_to_the_selected_bundle() {
    for records_written_before_restart in 0_usize..=3 {
        let fixture = Fixture::new();
        let store = fixture.store();
        store
            .migrate_epoch_state(
                &fixture.definition,
                Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
            )
            .unwrap();
        let prepared = prepare_rotation_candidate(
            &fixture.definition,
            KeyEpoch(0),
            &[fixture.receiver_a.clone(), fixture.receiver_b.clone()],
            fixture.two_receiver_rotation(),
        )
        .unwrap();
        let public = prepared.public().clone();
        assert_eq!(
            fixture.custody.prepare_if_absent(prepared).unwrap(),
            CustodySelection::Selected
        );

        if records_written_before_restart >= 1 {
            fixture
                .backend
                .put(
                    StoragePutInput::new(
                        CHANNEL_STORAGE_NAMESPACE,
                        activation_plan_record_key(channel_id(), KeyEpoch(1)),
                        ACTIVATION_PLAN_CONTENT_TYPE,
                        JsonValue::Object(vec![]),
                        public.plan_bytes().to_vec(),
                    )
                    .unwrap()
                    .with_if_absent(),
                )
                .unwrap();
        }
        for bytes in public
            .grants()
            .iter()
            .take(records_written_before_restart.saturating_sub(1))
        {
            let grant = grant_deserialize(bytes).unwrap();
            fixture
                .backend
                .put(
                    StoragePutInput::new(
                        CHANNEL_STORAGE_NAMESPACE,
                        key_grant_record_key(channel_id(), grant.key_epoch(), grant.receiver_id()),
                        GRANT_CONTENT_TYPE,
                        JsonValue::Object(vec![]),
                        bytes.clone(),
                    )
                    .unwrap()
                    .with_if_absent(),
                )
                .unwrap();
        }

        assert_eq!(
            fixture
                .store()
                .recover_preparation(&fixture.definition, KeyEpoch(1))
                .unwrap(),
            PreparationOutcome::Idempotent,
            "restart after {records_written_before_restart} public records"
        );
        assert_eq!(
            fixture
                .store()
                .activate_prepared_epoch(&fixture.definition, KeyEpoch(1))
                .unwrap(),
            ActivationOutcome::Activated
        );
    }
}

#[test]
fn sixteen_state_cas_conflicts_return_the_portable_concurrent_update_error() {
    let backend = StateCasConflictBackend::default();
    let signer = OriginatorSigningKey::from_seed([0x11; 32]);
    let receiver_key = ReceiverKeyPair::from_private_key([0x42; 32]).unwrap();
    let receiver = ReceiverIdentity {
        agent_id: AgentId::new(b"receiver-b".to_vec()).unwrap(),
        public_key: receiver_key.public_key(),
    };
    let definition = ChannelDefinition::new(
        channel_id(),
        OriginatorIdentity {
            agent_id: AgentId::new(b"originator".to_vec()).unwrap(),
            public_key: signer.public_key(),
        },
        vec![receiver.clone()],
        1,
        KeyEpoch(0),
    )
    .unwrap();
    ChannelDefinitionStore::new(&backend)
        .create(&definition)
        .unwrap();
    let custody = InMemoryKeyCustody::new();
    let store = EpochActivationStore::new_for_testing(&backend, &custody, channel_id()).unwrap();
    store
        .migrate_epoch_state(
            &definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    let rotation = plan_rotation(
        b"originator",
        channel_id(),
        KeyEpoch(0),
        ChannelMasterKey::from_bytes(NEXT_CMK),
        vec![RotationReceiver::with_material(
            b"receiver-b".to_vec(),
            receiver.public_key,
            [0x51; 32],
            [0x61; 24],
        )
        .unwrap()],
        &signer,
    )
    .unwrap();
    store
        .prepare_rotation(&definition, &[receiver], rotation)
        .unwrap();
    backend.reject_state_cas();
    let error = store
        .activate_prepared_epoch(&definition, KeyEpoch(1))
        .unwrap_err();
    assert_eq!(error.code(), "concurrent_update");
    assert_eq!(store.state().unwrap().active_epoch(), KeyEpoch(0));
}

#[test]
fn missing_custody_corrupt_plan_and_destroyed_channel_fail_closed() {
    let missing = Fixture::new();
    missing
        .store()
        .migrate_epoch_state(
            &missing.definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    assert_eq!(
        missing
            .store()
            .activate_prepared_epoch(&missing.definition, KeyEpoch(1))
            .unwrap_err()
            .code(),
        "preparation_missing"
    );

    let corrupt = Fixture::new();
    corrupt
        .store()
        .migrate_epoch_state(
            &corrupt.definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    let prepared = prepare_rotation_candidate(
        &corrupt.definition,
        KeyEpoch(0),
        std::slice::from_ref(&corrupt.receiver_b),
        corrupt.rotation(NEXT_CMK, 0x51),
    )
    .unwrap();
    let plan_bytes = prepared.public().plan_bytes().to_vec();
    corrupt.custody.prepare_if_absent(prepared).unwrap();
    corrupt
        .backend
        .put(
            StoragePutInput::new(
                CHANNEL_STORAGE_NAMESPACE,
                activation_plan_record_key(channel_id(), KeyEpoch(1)),
                "application/octet-stream",
                JsonValue::Object(vec![]),
                plan_bytes,
            )
            .unwrap()
            .with_if_absent(),
        )
        .unwrap();
    assert_eq!(
        corrupt
            .store()
            .recover_preparation(&corrupt.definition, KeyEpoch(1))
            .unwrap_err()
            .code(),
        "corrupt_record"
    );

    let destroyed = Fixture::new();
    destroyed
        .store()
        .migrate_epoch_state(
            &destroyed.definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    let retired = ChannelDefinitionStore::new(&destroyed.backend)
        .destroy(channel_id())
        .unwrap();
    assert_eq!(
        destroyed
            .store()
            .activate_prepared_epoch(&retired, KeyEpoch(1))
            .unwrap_err()
            .code(),
        "channel_destroyed"
    );
}

#[test]
fn recovery_rejects_decreasing_epochs_and_preparation_reports_exhaustion() {
    let fixture = Fixture::new();
    let store = fixture.store();
    store
        .migrate_epoch_state(
            &fixture.definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    store
        .prepare_rotation(
            &fixture.definition,
            std::slice::from_ref(&fixture.receiver_b),
            fixture.rotation(NEXT_CMK, 0x51),
        )
        .unwrap();
    store
        .activate_prepared_epoch(&fixture.definition, KeyEpoch(1))
        .unwrap();
    let epoch_two = plan_rotation(
        b"originator",
        channel_id(),
        KeyEpoch(1),
        ChannelMasterKey::from_bytes([0x32; 32]),
        vec![RotationReceiver::with_material(
            b"receiver-b".to_vec(),
            fixture.receiver_b.public_key,
            [0x53; 32],
            [0x63; 24],
        )
        .unwrap()],
        &fixture.signer,
    )
    .unwrap();
    store
        .prepare_rotation(
            &fixture.definition,
            std::slice::from_ref(&fixture.receiver_b),
            epoch_two,
        )
        .unwrap();
    store
        .activate_prepared_epoch(&fixture.definition, KeyEpoch(2))
        .unwrap();
    assert_eq!(
        store
            .recover_preparation(&fixture.definition, KeyEpoch(1))
            .unwrap_err()
            .code(),
        "decreasing_epoch"
    );

    let backend = InMemoryStorageBackend::new();
    let signer = OriginatorSigningKey::from_seed([0x11; 32]);
    let receiver_key = ReceiverKeyPair::from_private_key([0x42; 32]).unwrap();
    let receiver = ReceiverIdentity {
        agent_id: AgentId::new(b"receiver-b".to_vec()).unwrap(),
        public_key: receiver_key.public_key(),
    };
    let definition = ChannelDefinition::new(
        channel_id(),
        OriginatorIdentity {
            agent_id: AgentId::new(b"originator".to_vec()).unwrap(),
            public_key: signer.public_key(),
        },
        vec![receiver.clone()],
        1,
        KeyEpoch(u64::MAX),
    )
    .unwrap();
    ChannelDefinitionStore::new(&backend)
        .create(&definition)
        .unwrap();
    let custody = InMemoryKeyCustody::new();
    let exhausted =
        EpochActivationStore::new_for_testing(&backend, &custody, channel_id()).unwrap();
    exhausted
        .migrate_epoch_state(
            &definition,
            Some(&ChannelMasterKey::from_bytes(CURRENT_CMK)),
        )
        .unwrap();
    let irrelevant_rotation = plan_rotation(
        b"originator",
        channel_id(),
        KeyEpoch(0),
        ChannelMasterKey::from_bytes(NEXT_CMK),
        vec![RotationReceiver::with_material(
            b"receiver-b".to_vec(),
            receiver.public_key,
            [0x51; 32],
            [0x61; 24],
        )
        .unwrap()],
        &signer,
    )
    .unwrap();
    assert_eq!(
        exhausted
            .prepare_rotation(&definition, &[receiver], irrelevant_rotation)
            .unwrap_err()
            .code(),
        "epoch_exhausted"
    );
}
use std::sync::atomic::{AtomicBool, Ordering};

/// A candidate whose `base_epoch` is already saturated has no successor at all.
///
/// The D18T roster calls that `epoch_exhausted`, not `unexpected_epoch`:
/// `base_epoch + 1` is not a meaningful question once `base_epoch` is
/// `u64::MAX`, so exhaustion must be checked before the successor comparison.
/// This test is what keeps the two codes disjoint.
#[test]
fn candidate_construction_reports_exhaustion_before_an_unexpected_successor() {
    let fixture = Fixture::new();
    // D18Q itself refuses to plan a rotation from a saturated epoch, so the
    // rotation is built normally and the saturated base_epoch is supplied
    // separately -- which is exactly how a caller reaches this path.
    let rotation = plan_rotation(
        b"originator",
        channel_id(),
        KeyEpoch(0),
        ChannelMasterKey::from_bytes([0x34; 32]),
        vec![RotationReceiver::with_material(
            b"receiver-b".to_vec(),
            fixture.receiver_b.public_key,
            [0x55; 32],
            [0x65; 24],
        )
        .unwrap()],
        &fixture.signer,
    )
    .unwrap();
    assert_eq!(
        prepare_rotation_candidate(
            &fixture.definition,
            KeyEpoch(u64::MAX),
            std::slice::from_ref(&fixture.receiver_b),
            rotation,
        )
        .unwrap_err()
        .code(),
        "epoch_exhausted"
    );
}
