package epochactivation

import (
	"testing"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
	channelstore "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-store"
)

// durableMemoryCustody is InMemoryKeyCustody that claims durability, so the
// production constructor will accept it. Only tests may do this; the point of
// the Durable() split is that a real deployment cannot.
type durableMemoryCustody struct{ *InMemoryKeyCustody }

func (durableMemoryCustody) Durable() bool { return true }

var (
	testChannelID = mustHex("018f47a09b6c7def923456789abcdef0")
	testMessageID = mustHex("018f47a09b6c7def923456789abcdef1")
	currentCMK    = repeatByte(0x22, 32)
	nextCMK       = repeatByte(0x33, 32)
)

func mustHex(value string) []byte {
	out := make([]byte, len(value)/2)
	for index := 0; index < len(out); index++ {
		high := hexNibble(value[index*2])
		low := hexNibble(value[index*2+1])
		out[index] = high<<4 | low
	}
	return out
}

func hexNibble(character byte) byte {
	switch {
	case character >= '0' && character <= '9':
		return character - '0'
	case character >= 'a' && character <= 'f':
		return character - 'a' + 10
	default:
		panic("bad hex digit")
	}
}

type harness struct {
	t          *testing.T
	signer     *channelcrypto.OriginatorSigningKey
	receiverA  channelstore.ReceiverIdentity
	receiverB  channelstore.ReceiverIdentity
	definition channelstore.ChannelDefinition
	backend    *channelstore.MemoryChannelStorage
	custody    *InMemoryKeyCustody
	store      *Store
}

func newHarness(t *testing.T, production bool) *harness {
	t.Helper()
	signer, err := channelcrypto.OriginatorSigningKeyFromSeed(repeatByte(0x11, 32))
	if err != nil {
		t.Fatal(err)
	}
	signerPublic, err := signer.PublicKey()
	if err != nil {
		t.Fatal(err)
	}
	receiverAKey, err := channelcrypto.ReceiverKeyPairFromPrivateKey(repeatByte(0x41, 32))
	if err != nil {
		t.Fatal(err)
	}
	defer receiverAKey.Destroy()
	receiverBKey, err := channelcrypto.ReceiverKeyPairFromPrivateKey(repeatByte(0x42, 32))
	if err != nil {
		t.Fatal(err)
	}
	defer receiverBKey.Destroy()
	receiverAPublic, err := receiverAKey.PublicKey()
	if err != nil {
		t.Fatal(err)
	}
	receiverBPublic, err := receiverBKey.PublicKey()
	if err != nil {
		t.Fatal(err)
	}
	receiverA, err := channelstore.NewReceiverIdentity([]byte("receiver-a"), receiverAPublic)
	if err != nil {
		t.Fatal(err)
	}
	receiverB, err := channelstore.NewReceiverIdentity([]byte("receiver-b"), receiverBPublic)
	if err != nil {
		t.Fatal(err)
	}
	originator, err := channelstore.NewOriginatorIdentity([]byte("originator"), signerPublic)
	if err != nil {
		t.Fatal(err)
	}
	definition, err := channelstore.NewChannelDefinition(
		testChannelID, originator,
		[]channelstore.ReceiverIdentity{receiverA, receiverB},
		1_725_000_000_000_000_000, 0, channelstore.LifecycleActive,
	)
	if err != nil {
		t.Fatal(err)
	}
	backend := channelstore.NewMemoryChannelStorage()
	inner := NewInMemoryKeyCustody()

	var store *Store
	if production {
		store, err = NewStore(backend, durableMemoryCustody{inner}, testChannelID)
	} else {
		store, err = NewStoreForTesting(backend, inner, testChannelID)
	}
	if err != nil {
		t.Fatal(err)
	}
	h := &harness{t: t, signer: signer, receiverA: receiverA, receiverB: receiverB, definition: definition, backend: backend, custody: inner, store: store}
	t.Cleanup(func() { signer.Destroy() })
	return h
}

func (h *harness) create() {
	h.t.Helper()
	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
	if err != nil {
		h.t.Fatal(err)
	}
	state, err := h.store.CreateEpochChannel(h.definition, cmk)
	if err != nil {
		h.t.Fatal(err)
	}
	if state.ActiveEpoch() != 0 || state.NextSequence() != 0 || state.PendingHeader() != nil {
		h.t.Fatalf("unexpected initial state (%d, %d, %v)", state.ActiveEpoch(), state.NextSequence(), state.PendingHeader())
	}
}

func (h *harness) rotation() *channelcrypto.RotationPlan {
	h.t.Helper()
	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(nextCMK)
	if err != nil {
		h.t.Fatal(err)
	}
	receiverPublic := h.receiverB.PublicKey()
	rotationReceiver, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverB.AgentID(), receiverPublic, repeatByte(0x51, 32), repeatByte(0x61, 24),
	)
	if err != nil {
		h.t.Fatal(err)
	}
	plan, err := channelcrypto.PlanRotation(
		[]byte("originator"), testChannelID, 0, cmk,
		[]*channelcrypto.RotationReceiver{rotationReceiver}, h.signer,
	)
	if err != nil {
		h.t.Fatal(err)
	}
	return plan
}

func requireCode(t *testing.T, err error, code ErrorCode) {
	t.Helper()
	if err == nil {
		t.Fatalf("expected %s, got success", code)
	}
	if !IsCode(err, code) {
		t.Fatalf("expected %s, got %v", code, err)
	}
	if err.Error() != string(code) {
		t.Fatalf("error message %q must be exactly the stable code %q", err.Error(), code)
	}
}

func TestProductionRejectsNonDurableCustodyAndAcceptsDurable(t *testing.T) {
	backend := channelstore.NewMemoryChannelStorage()
	_, err := NewStore(backend, NewInMemoryKeyCustody(), testChannelID)
	requireCode(t, err, ErrCustody)

	// The same custody, wrapped in a type that honestly claims durability, is
	// accepted -- so the gate is on the declaration, not on the type.
	h := newHarness(t, true)
	h.create()
}

func TestCustodyFirstCreationIsIdempotentAndConflictsFailClosed(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	// Re-creating with the identical CMK is an idempotent success.
	same, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
	if err != nil {
		t.Fatal(err)
	}
	state, err := h.store.CreateEpochChannel(h.definition, same)
	if err != nil {
		t.Fatal(err)
	}
	if state.ActiveEpoch() != 0 {
		t.Fatalf("active epoch %d", state.ActiveEpoch())
	}

	// A different CMK for the same epoch is a fail-closed conflict, and the
	// error does not disclose how the stored secret differed.
	different, err := channelcrypto.ChannelMasterKeyFromBytes(repeatByte(0x99, 32))
	if err != nil {
		t.Fatal(err)
	}
	_, err = h.store.CreateEpochChannel(h.definition, different)
	requireCode(t, err, ErrConflictingActiveKey)
}

func TestPrepareRecoverActivateAndProspectiveRevocation(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	outcome, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation())
	if err != nil {
		t.Fatal(err)
	}
	if outcome != Prepared {
		t.Fatalf("outcome %q, want prepared", outcome)
	}
	plan, err := h.store.ActivationPlanRecord(1)
	if err != nil || plan == nil {
		t.Fatalf("plan should be publicly durable after prepare: %v", err)
	}

	// Recovery is idempotent and does not activate.
	recovered, err := h.store.RecoverPreparation(h.definition, 1)
	if err != nil {
		t.Fatal(err)
	}
	if recovered != PreparationRepeated {
		t.Fatalf("recovery outcome %q", recovered)
	}
	state, err := h.store.State()
	if err != nil {
		t.Fatal(err)
	}
	if state.ActiveEpoch() != 0 {
		t.Fatal("preparation must not change the active epoch")
	}

	activated, err := h.store.ActivatePreparedEpoch(h.definition, 1)
	if err != nil {
		t.Fatal(err)
	}
	if activated != Activated {
		t.Fatalf("activation outcome %q", activated)
	}
	repeat, err := h.store.ActivatePreparedEpoch(h.definition, 1)
	if err != nil {
		t.Fatal(err)
	}
	if repeat != ActivationRepeated {
		t.Fatalf("repeat activation outcome %q", repeat)
	}
	state, err = h.store.State()
	if err != nil {
		t.Fatal(err)
	}
	if state.ActiveEpoch() != 1 {
		t.Fatalf("active epoch %d after activation", state.ActiveEpoch())
	}

	// Prospective revocation: the originator retains BOTH epoch keys, because
	// old messages were encrypted under epoch 0 and are never re-encrypted.
	for _, epoch := range []uint64{0, 1} {
		handle, err := h.custody.ResolveHandle(testChannelID, epoch)
		if err != nil || handle == nil {
			t.Fatalf("epoch %d key must remain resolvable: %v", epoch, err)
		}
	}
}

// TestCrashAfterCustodySelectionReplaysPublicRecords is the crash-safety core.
// It selects a candidate in custody and then does nothing else -- simulating a
// process that died between phase 2 and phase 4 -- and requires recovery to
// reconstruct every public record from the durable bundle alone.
func TestCrashAfterCustodySelectionReplaysPublicRecords(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	prepared, err := PrepareRotationCandidate(h.definition, 0, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation())
	if err != nil {
		t.Fatal(err)
	}
	selection, err := h.custody.PrepareIfAbsent(prepared)
	if err != nil {
		t.Fatal(err)
	}
	if selection != CustodySelected {
		t.Fatalf("first claim %q, want selected", selection)
	}
	// A byte-identical retry is idempotent, not a conflict.
	again, err := h.custody.PrepareIfAbsent(prepared)
	if err != nil {
		t.Fatal(err)
	}
	if again != CustodyIdempotent {
		t.Fatalf("identical retry %q, want idempotent", again)
	}
	prepared.Destroy()

	// Crash point: custody holds the bundle, nothing is public yet.
	plan, err := h.store.ActivationPlanRecord(1)
	if err != nil {
		t.Fatal(err)
	}
	if plan != nil {
		t.Fatal("no public plan should exist before replay")
	}

	if _, err := h.store.RecoverPreparation(h.definition, 1); err != nil {
		t.Fatal(err)
	}
	plan, err = h.store.ActivationPlanRecord(1)
	if err != nil || plan == nil {
		t.Fatalf("recovery must reconstruct the public plan: %v", err)
	}
	if plan.NewEpoch() != 1 || plan.BaseEpoch() != 0 {
		t.Fatalf("recovered plan epochs (%d, %d)", plan.BaseEpoch(), plan.NewEpoch())
	}
}

// TestDifferentCandidateLosesTheCustodySlot covers the race trace of the same
// name: two candidates compete for E+1, exactly one is selected, and the loser
// must not write anything public.
func TestDifferentCandidateLosesTheCustodySlot(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	winner, err := PrepareRotationCandidate(h.definition, 0, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation())
	if err != nil {
		t.Fatal(err)
	}
	defer winner.Destroy()
	if selection, err := h.custody.PrepareIfAbsent(winner); err != nil || selection != CustodySelected {
		t.Fatalf("winner selection %q %v", selection, err)
	}

	// A second candidate for the same epoch, differing only in its CMK.
	otherCMK, err := channelcrypto.ChannelMasterKeyFromBytes(repeatByte(0x77, 32))
	if err != nil {
		t.Fatal(err)
	}
	otherReceiver, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverB.AgentID(), h.receiverB.PublicKey(), repeatByte(0x52, 32), repeatByte(0x62, 24),
	)
	if err != nil {
		t.Fatal(err)
	}
	otherRotation, err := channelcrypto.PlanRotation(
		[]byte("originator"), testChannelID, 0, otherCMK,
		[]*channelcrypto.RotationReceiver{otherReceiver}, h.signer,
	)
	if err != nil {
		t.Fatal(err)
	}
	loser, err := PrepareRotationCandidate(h.definition, 0, []channelstore.ReceiverIdentity{h.receiverB}, otherRotation)
	if err != nil {
		t.Fatal(err)
	}
	defer loser.Destroy()

	selection, err := h.custody.PrepareIfAbsent(loser)
	if err != nil {
		t.Fatal(err)
	}
	if selection != CustodyConflict {
		t.Fatalf("second candidate got %q, want conflict", selection)
	}

	// And the store-level path reports the stable code.
	_, err = h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, otherRotationFor(t, h))
	requireCode(t, err, ErrConflictingPreparation)
}

func otherRotationFor(t *testing.T, h *harness) *channelcrypto.RotationPlan {
	t.Helper()
	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(repeatByte(0x88, 32))
	if err != nil {
		t.Fatal(err)
	}
	receiver, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverB.AgentID(), h.receiverB.PublicKey(), repeatByte(0x53, 32), repeatByte(0x63, 24),
	)
	if err != nil {
		t.Fatal(err)
	}
	plan, err := channelcrypto.PlanRotation(
		[]byte("originator"), testChannelID, 0, cmk,
		[]*channelcrypto.RotationReceiver{receiver}, h.signer,
	)
	if err != nil {
		t.Fatal(err)
	}
	return plan
}

// TestPendingPublishSerializesRotation covers both directions of the shared-CAS
// race: a reservation in flight blocks activation, and clearing it unblocks.
func TestPendingPublishSerializesRotation(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	epoch := uint64(0)
	reservation, err := h.store.ReservePublishUsingActiveEpoch(h.definition, ActiveEpochAppendRequest{
		MessageID: testMessageID, TimestampNS: 1_725_000_000_000_000_001,
		OriginatorID: []byte("originator"), ContentType: "application/octet-stream", KeyEpoch: &epoch,
	}, []byte("hello"))
	if err != nil {
		t.Fatal(err)
	}
	if reservation.Header.Sequence() != 0 {
		t.Fatalf("reserved sequence %d", reservation.Header.Sequence())
	}
	if reservation.KeyHandle.Epoch() != 0 {
		t.Fatalf("reservation bound to epoch %d", reservation.KeyHandle.Epoch())
	}
	if reservation.KeyHandle.String() != "EpochKeyHandle([REDACTED])" {
		t.Fatalf("handle must redact, got %q", reservation.KeyHandle.String())
	}

	// Publication won the CAS, so activation must yield rather than race it.
	_, err = h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation())
	requireCode(t, err, ErrPendingAppend)

	abandoned, err := h.store.AbandonPending()
	if err != nil {
		t.Fatal(err)
	}
	if abandoned == nil || !abandoned.Equal(reservation.Header) {
		t.Fatal("abandon must return the exact reservation it cleared")
	}
	if second, err := h.store.AbandonPending(); err != nil || second != nil {
		t.Fatalf("abandoning twice must be a no-op: %v %v", second, err)
	}

	// With the reservation cleared, rotation proceeds.
	if _, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation()); err != nil {
		t.Fatal(err)
	}
}

func TestUnactivatedEpochIsRejectedBeforeAnyStateChange(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	future := uint64(1)
	_, err := h.store.ReservePublishUsingActiveEpoch(h.definition, ActiveEpochAppendRequest{
		MessageID: testMessageID, TimestampNS: 1, OriginatorID: []byte("originator"),
		ContentType: "application/octet-stream", KeyEpoch: &future,
	}, []byte("hello"))
	requireCode(t, err, ErrUnactivatedEpoch)

	// The rejection must not have mutated state.
	state, err := h.store.State()
	if err != nil {
		t.Fatal(err)
	}
	if state.PendingHeader() != nil || state.NextSequence() != 0 {
		t.Fatal("a rejected reservation must leave state untouched")
	}
}

func TestFailClosedPreconditionsAndStableCodes(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	t.Run("preparation-missing", func(t *testing.T) {
		_, err := h.store.ActivatePreparedEpoch(h.definition, 1)
		requireCode(t, err, ErrPreparationMissing)
	})

	t.Run("unexpected-epoch-on-recovery", func(t *testing.T) {
		_, err := h.store.RecoverPreparation(h.definition, 2)
		requireCode(t, err, ErrUnexpectedEpoch)
	})

	t.Run("wrong-originator", func(t *testing.T) {
		_, err := h.store.ReservePublishUsingActiveEpoch(h.definition, ActiveEpochAppendRequest{
			MessageID: testMessageID, TimestampNS: 1, OriginatorID: []byte("not-originator"),
			ContentType: "application/octet-stream",
		}, []byte("hello"))
		requireCode(t, err, ErrInvalidPlan)
	})

	t.Run("empty-target-roster", func(t *testing.T) {
		_, err := h.store.PrepareRotation(h.definition, nil, h.rotation())
		requireCode(t, err, ErrInvalidPlan)
	})

	t.Run("active-key-missing-with-foreign-custody", func(t *testing.T) {
		isolated, err := NewStoreForTesting(h.backend, NewInMemoryKeyCustody(), testChannelID)
		if err != nil {
			t.Fatal(err)
		}
		_, err = isolated.MigrateEpochState(h.definition, nil)
		requireCode(t, err, ErrActiveKeyMissing)
	})

	t.Run("custody-error-on-unknown-handle", func(t *testing.T) {
		missing := NewEpochKeyHandle(testChannelID, 99)
		err := h.custody.WithKey(missing, func(*channelcrypto.ChannelMasterKey) error { return nil })
		requireCode(t, err, ErrCustody)
	})
}

func TestNotInitializedBeforeCreation(t *testing.T) {
	h := newHarness(t, false)
	_, err := h.store.State()
	requireCode(t, err, ErrNotInitialized)

	_, err = h.store.RecoverPreparation(h.definition, 1)
	requireCode(t, err, ErrNotInitialized)
}

func TestCorruptPublicStateFailsClosed(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	key, err := channelstore.SequenceStateRecordKey(testChannelID)
	if err != nil {
		t.Fatal(err)
	}
	record, err := h.backend.Get(channelstore.ChannelStorageNamespace, key)
	if err != nil || record == nil {
		t.Fatal("state record should exist")
	}
	h.backend.Corrupt(channelstore.StorageRecord{
		Namespace: record.Namespace, Key: record.Key, ContentType: record.ContentType,
		Revision: record.Revision, Body: append(append([]byte(nil), record.Body...), 0x00),
	})
	_, err = h.store.State()
	requireCode(t, err, ErrCorruptRecord)
}

// TestDestroyWipesCustodyButRetainsPublicHistory is invariant 6, made
// observable: destruction erases secrets but leaves the append-only public
// record exactly where it was.
func TestDestroyWipesCustodyButRetainsPublicHistory(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	if _, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation()); err != nil {
		t.Fatal(err)
	}
	if _, err := h.store.ActivatePreparedEpoch(h.definition, 1); err != nil {
		t.Fatal(err)
	}
	before, err := h.store.ActivationPlanRecord(1)
	if err != nil || before == nil {
		t.Fatal("plan must exist before destruction")
	}

	destroyed, err := channelstore.NewChannelDefinitionStore(h.backend).Destroy(testChannelID)
	if err != nil {
		t.Fatal(err)
	}
	if err := h.store.ApplyDestruction(destroyed); err != nil {
		t.Fatal(err)
	}
	if count := h.custody.RetainedKeyCount(); count != 0 {
		t.Fatalf("custody retained %d keys after destruction", count)
	}

	after, err := h.store.ActivationPlanRecord(1)
	if err != nil || after == nil {
		t.Fatalf("public plan must survive destruction: %v", err)
	}
	if !after.Equal(*before) {
		t.Fatal("destruction altered an append-only public record")
	}

	// And no further publishing is possible.
	_, err = h.store.ReservePublishUsingActiveEpoch(destroyed, ActiveEpochAppendRequest{
		MessageID: testMessageID, TimestampNS: 1, OriginatorID: []byte("originator"),
		ContentType: "application/octet-stream",
	}, []byte("hello"))
	requireCode(t, err, ErrChannelDestroyed)
}
