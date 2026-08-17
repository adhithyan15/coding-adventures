package epochactivation

import (
	"testing"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
	channelstore "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-store"
)

// TestMigrateFromD18PVersionOnePreservesSequenceAndPending covers the real
// upgrade path: a channel that already exists under D18P version 1 state, with
// messages already published, is brought to version 2 without losing its
// sequence or its in-flight reservation.
func TestMigrateFromD18PVersionOnePreservesSequenceAndPending(t *testing.T) {
	h := newHarness(t, false)

	// Build a version 1 channel using the plain D18P store, so the state record
	// on disk is genuinely version 1 rather than something this package wrote.
	definitions := channelstore.NewChannelDefinitionStore(h.backend)
	if _, err := definitions.Create(h.definition); err != nil {
		t.Fatal(err)
	}
	legacy, err := channelstore.NewChannelStore(h.backend, testChannelID)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := legacy.Initialize(); err != nil {
		t.Fatal(err)
	}
	header, err := legacy.ReserveAppend(channelstore.AppendRequest{
		MessageID: testMessageID, TimestampNS: 1_725_000_000_000_000_001,
		OriginatorID: []byte("originator"), ContentType: "application/octet-stream",
	}, []byte("hello"))
	if err != nil {
		t.Fatal(err)
	}

	// Migration without a CMK and without prior custody must fail closed rather
	// than publish a version 2 record whose active epoch has no key.
	_, err = h.store.MigrateEpochState(h.definition, nil)
	requireCode(t, err, ErrActiveKeyMissing)

	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
	if err != nil {
		t.Fatal(err)
	}
	state, err := h.store.MigrateEpochState(h.definition, cmk)
	if err != nil {
		t.Fatal(err)
	}
	if state.ActiveEpoch() != h.definition.KeyEpoch() {
		t.Fatalf("migrated active epoch %d, want %d", state.ActiveEpoch(), h.definition.KeyEpoch())
	}
	if state.NextSequence() != 1 {
		t.Fatalf("migration must preserve next_sequence, got %d", state.NextSequence())
	}
	pending := state.PendingHeader()
	if pending == nil || !pending.Equal(header) {
		t.Fatal("migration must preserve the in-flight reservation exactly")
	}

	// Re-migrating an already-version-2 channel is idempotent and does not
	// reset the epoch from the immutable definition.
	again, err := h.store.MigrateEpochState(h.definition, nil)
	if err != nil {
		t.Fatal(err)
	}
	if !again.Equal(state) {
		t.Fatal("re-migration must be idempotent")
	}
}

// TestMigrationRejectsPendingHeaderFromAnotherEpoch covers the version 1
// consistency check: a legacy pending header whose key epoch disagrees with the
// definition means the stored state was never coherent.
func TestMigrationRejectsPendingHeaderFromAnotherEpoch(t *testing.T) {
	h := newHarness(t, false)
	definitions := channelstore.NewChannelDefinitionStore(h.backend)
	if _, err := definitions.Create(h.definition); err != nil {
		t.Fatal(err)
	}
	// Hand-build a version 1 state whose pending header names epoch 9 while the
	// definition says epoch 0.
	hash := make([]byte, 32)
	header, err := channelstore.NewMessageHeader(
		testMessageID, 1, []byte("originator"), testChannelID, 0, 9, "application/octet-stream", hash,
	)
	if err != nil {
		t.Fatal(err)
	}
	body, err := channelstore.ChannelStateSerialize(channelstore.ChannelState{NextSequence: 1, PendingHeader: &header})
	if err != nil {
		t.Fatal(err)
	}
	key, err := channelstore.SequenceStateRecordKey(testChannelID)
	if err != nil {
		t.Fatal(err)
	}
	// Overwrite rather than create: ChannelDefinitionStore.Create already seeds
	// a version 1 state record, so an if-absent write would just conflict.
	h.backend.Corrupt(channelstore.StorageRecord{
		Namespace: channelstore.ChannelStorageNamespace, Key: key,
		ContentType: channelstore.ChannelStateContentType, Revision: "1", Body: body,
	})

	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
	if err != nil {
		t.Fatal(err)
	}
	_, err = h.store.MigrateEpochState(h.definition, cmk)
	requireCode(t, err, ErrCorruptRecord)
}

func TestCreateRejectsForeignChannelAndDestroyedDefinition(t *testing.T) {
	h := newHarness(t, false)

	t.Run("foreign-channel", func(t *testing.T) {
		other := mustHex("018f47a09b6c7def923456789abcdef9")
		originator, err := channelstore.NewOriginatorIdentity([]byte("originator"), h.definition.Originator().PublicKey())
		if err != nil {
			t.Fatal(err)
		}
		foreign, err := channelstore.NewChannelDefinition(
			other, originator, []channelstore.ReceiverIdentity{h.receiverA},
			1_725_000_000_000_000_000, 0, channelstore.LifecycleActive,
		)
		if err != nil {
			t.Fatal(err)
		}
		cmk, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
		if err != nil {
			t.Fatal(err)
		}
		_, err = h.store.CreateEpochChannel(foreign, cmk)
		requireCode(t, err, ErrInvalidPlan)
	})

	t.Run("recreate-after-destroy-is-channel-destroyed", func(t *testing.T) {
		fresh := newHarness(t, false)
		fresh.create()
		if _, err := channelstore.NewChannelDefinitionStore(fresh.backend).Destroy(testChannelID); err != nil {
			t.Fatal(err)
		}
		cmk, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
		if err != nil {
			t.Fatal(err)
		}
		_, err = fresh.store.CreateEpochChannel(fresh.definition, cmk)
		requireCode(t, err, ErrChannelDestroyed)
	})

	t.Run("conflicting-definition-at-same-channel", func(t *testing.T) {
		fresh := newHarness(t, false)
		fresh.create()
		// Same channel ID, different membership -- not the definition on disk.
		originator, err := channelstore.NewOriginatorIdentity([]byte("originator"), fresh.definition.Originator().PublicKey())
		if err != nil {
			t.Fatal(err)
		}
		different, err := channelstore.NewChannelDefinition(
			testChannelID, originator, []channelstore.ReceiverIdentity{fresh.receiverA},
			1_725_000_000_000_000_000, 0, channelstore.LifecycleActive,
		)
		if err != nil {
			t.Fatal(err)
		}
		cmk, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
		if err != nil {
			t.Fatal(err)
		}
		_, err = fresh.store.CreateEpochChannel(different, cmk)
		requireCode(t, err, ErrInvalidPlan)
	})
}

func TestApplyDestructionRequiresADestroyedDefinition(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	// The channel is still active, so destruction bookkeeping must refuse.
	err := h.store.ApplyDestruction(h.definition)
	requireCode(t, err, ErrInvalidPlan)
	if count := h.custody.RetainedKeyCount(); count == 0 {
		t.Fatal("a refused destruction must not have wiped custody")
	}
}

func TestPrepareRotationRejectsWrongEpochRotation(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	// Build a rotation targeting epoch 2 while the channel is still at 0.
	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(nextCMK)
	if err != nil {
		t.Fatal(err)
	}
	receiver, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverB.AgentID(), h.receiverB.PublicKey(), repeatByte(0x56, 32), repeatByte(0x66, 24),
	)
	if err != nil {
		t.Fatal(err)
	}
	rotation, err := channelcrypto.PlanRotation(
		[]byte("originator"), testChannelID, 1, cmk,
		[]*channelcrypto.RotationReceiver{receiver}, h.signer,
	)
	if err != nil {
		t.Fatal(err)
	}
	_, err = h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, rotation)
	requireCode(t, err, ErrUnexpectedEpoch)
}

func TestActivateRejectsUnexpectedEpochGap(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	if _, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation()); err != nil {
		t.Fatal(err)
	}
	// A preparation exists for epoch 1, but activating epoch 2 would skip a
	// global epoch, which D18T forbids outright.
	_, err := h.store.ActivatePreparedEpoch(h.definition, 2)
	requireCode(t, err, ErrPreparationMissing)
}

func TestStoreInitializeFailurePropagates(t *testing.T) {
	failing := &initFailingBackend{}
	_, err := NewStoreForTesting(failing, NewInMemoryKeyCustody(), testChannelID)
	requireCode(t, err, ErrStorage)

	_, err = NewStore(failing, durableMemoryCustody{NewInMemoryKeyCustody()}, testChannelID)
	requireCode(t, err, ErrStorage)
}

type initFailingBackend struct {
	channelstore.ChannelStorageBackend
}

func (*initFailingBackend) Initialize() error { return errInjected }
