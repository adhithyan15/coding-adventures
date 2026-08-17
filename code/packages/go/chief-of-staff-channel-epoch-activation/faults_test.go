package epochactivation

import (
	"errors"
	"testing"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
	channelstore "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-store"
)

// faultyBackend wraps a real backend and injects failures on demand.
//
// D18T's security argument rests on fail-closed behavior: when the public store
// misbehaves, every operation must stop with a stable code rather than proceed
// on a guess. That is impossible to demonstrate against a backend that always
// works, so these tests supply one that does not.
type faultyBackend struct {
	inner *channelstore.MemoryChannelStorage

	failGet bool
	failPut bool

	// conflictPut counts how many more times a Put should report a storage
	// conflict, and conflictKey optionally narrows that to one record.
	//
	// The narrowing matters. Activation writes the immutable plan and grants
	// before it reaches the state CAS, and putImmutable treats a conflict as
	// "already present, compare the bytes" rather than as a retry. An
	// unscoped conflict budget is therefore silently spent on those writes and
	// never reaches the loop under test -- which is exactly the false pass this
	// field exists to prevent.
	conflictPut int
	conflictKey string
}

var errInjected = errors.New("injected backend failure")

func (b *faultyBackend) Initialize() error { return b.inner.Initialize() }

func (b *faultyBackend) Get(namespace, key string) (*channelstore.StorageRecord, error) {
	if b.failGet {
		return nil, errInjected
	}
	return b.inner.Get(namespace, key)
}

func (b *faultyBackend) Put(value channelstore.StoragePut) (channelstore.StorageRecord, error) {
	if b.conflictPut > 0 && (b.conflictKey == "" || b.conflictKey == value.Key) {
		b.conflictPut--
		return channelstore.StorageRecord{}, &channelstore.StorageConflictError{}
	}
	if b.failPut {
		return channelstore.StorageRecord{}, errInjected
	}
	return b.inner.Put(value)
}

func (b *faultyBackend) List(namespace string, options channelstore.StorageListOptions) (channelstore.StoragePage, error) {
	return b.inner.List(namespace, options)
}

func newFaultyHarness(t *testing.T) (*harness, *faultyBackend) {
	t.Helper()
	h := newHarness(t, false)
	h.create()
	faulty := &faultyBackend{inner: h.backend}
	store, err := NewStoreForTesting(faulty, h.custody, testChannelID)
	if err != nil {
		t.Fatal(err)
	}
	h.store = store
	return h, faulty
}

func TestStorageFailuresFailClosedWithStableCodes(t *testing.T) {
	t.Run("get-failure-during-state-read", func(t *testing.T) {
		h, faulty := newFaultyHarness(t)
		faulty.failGet = true
		_, err := h.store.State()
		requireCode(t, err, ErrStorage)
	})

	t.Run("get-failure-during-plan-read", func(t *testing.T) {
		h, faulty := newFaultyHarness(t)
		faulty.failGet = true
		_, err := h.store.ActivationPlanRecord(1)
		requireCode(t, err, ErrStorage)
	})

	t.Run("put-failure-during-reservation", func(t *testing.T) {
		h, faulty := newFaultyHarness(t)
		faulty.failPut = true
		_, err := h.store.ReservePublishUsingActiveEpoch(h.definition, ActiveEpochAppendRequest{
			MessageID: testMessageID, TimestampNS: 1, OriginatorID: []byte("originator"),
			ContentType: "application/octet-stream",
		}, []byte("hello"))
		requireCode(t, err, ErrStorage)
	})

	t.Run("put-failure-during-plan-write", func(t *testing.T) {
		h, faulty := newFaultyHarness(t)
		faulty.failPut = true
		_, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation())
		requireCode(t, err, ErrStorage)
	})
}

// TestBoundedCASGivesUpRatherThanForcingAWrite is invariant 8. Sixteen
// consecutive conflicts must produce concurrent_update -- never an
// unconditional write that clobbers whoever kept winning.
func TestBoundedCASGivesUpRatherThanForcingAWrite(t *testing.T) {
	t.Run("reservation", func(t *testing.T) {
		h, faulty := newFaultyHarness(t)
		faulty.conflictPut = MaxEpochCASAttempts
		_, err := h.store.ReservePublishUsingActiveEpoch(h.definition, ActiveEpochAppendRequest{
			MessageID: testMessageID, TimestampNS: 1, OriginatorID: []byte("originator"),
			ContentType: "application/octet-stream",
		}, []byte("hello"))
		requireCode(t, err, ErrConcurrentUpdate)
	})

	t.Run("abandon", func(t *testing.T) {
		h, faulty := newFaultyHarness(t)
		if _, err := h.store.ReservePublishUsingActiveEpoch(h.definition, ActiveEpochAppendRequest{
			MessageID: testMessageID, TimestampNS: 1, OriginatorID: []byte("originator"),
			ContentType: "application/octet-stream",
		}, []byte("hello")); err != nil {
			t.Fatal(err)
		}
		faulty.conflictPut = MaxEpochCASAttempts
		_, err := h.store.AbandonPending()
		requireCode(t, err, ErrConcurrentUpdate)
	})

	t.Run("activation", func(t *testing.T) {
		h, faulty := newFaultyHarness(t)
		if _, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation()); err != nil {
			t.Fatal(err)
		}
		stateKey, err := channelstore.SequenceStateRecordKey(testChannelID)
		if err != nil {
			t.Fatal(err)
		}
		// Scoped to the state record: activation replays the plan and grants
		// first, and those writes must not absorb the conflict budget.
		faulty.conflictKey = stateKey
		faulty.conflictPut = MaxEpochCASAttempts
		_, err = h.store.ActivatePreparedEpoch(h.definition, 1)
		requireCode(t, err, ErrConcurrentUpdate)
	})

	// One fewer conflict than the limit must still succeed, which is what makes
	// the bound a real boundary rather than a number the loop never reaches.
	t.Run("succeeds-just-below-the-limit", func(t *testing.T) {
		h, faulty := newFaultyHarness(t)
		faulty.conflictPut = MaxEpochCASAttempts - 1
		if _, err := h.store.ReservePublishUsingActiveEpoch(h.definition, ActiveEpochAppendRequest{
			MessageID: testMessageID, TimestampNS: 1, OriginatorID: []byte("originator"),
			ContentType: "application/octet-stream",
		}, []byte("hello")); err != nil {
			t.Fatalf("15 conflicts should still converge: %v", err)
		}
	})
}

// TestConflictingPlanBytesAreNeverReplaced covers the immutability guarantee:
// different bytes at an already-occupied plan key are a stable conflict, and
// the stored record is left alone.
func TestConflictingPlanBytesAreNeverReplaced(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	if _, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation()); err != nil {
		t.Fatal(err)
	}
	planKey, err := ActivationPlanRecordKey(testChannelID, 1)
	if err != nil {
		t.Fatal(err)
	}
	original, err := h.backend.Get(channelstore.ChannelStorageNamespace, planKey)
	if err != nil || original == nil {
		t.Fatal("plan must exist")
	}

	// Attempt to write different bytes at the same immutable key.
	err = h.store.putImmutable(planKey, ActivationPlanContentType, []byte("different"), ErrConflictingPlan)
	requireCode(t, err, ErrConflictingPlan)

	after, err := h.backend.Get(channelstore.ChannelStorageNamespace, planKey)
	if err != nil || after == nil {
		t.Fatal("plan must still exist")
	}
	if !bytesEqual(after.Body, original.Body) {
		t.Fatal("a conflicting write must not modify the stored record")
	}

	// A byte-identical rewrite is an idempotent success, not a conflict.
	if err := h.store.putImmutable(planKey, ActivationPlanContentType, original.Body, ErrConflictingPlan); err != nil {
		t.Fatalf("identical rewrite must be idempotent: %v", err)
	}
}

func TestWrongContentTypeOnPlanKeyIsCorruptRecord(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	planKey, err := ActivationPlanRecordKey(testChannelID, 1)
	if err != nil {
		t.Fatal(err)
	}
	h.backend.Corrupt(channelstore.StorageRecord{
		Namespace: channelstore.ChannelStorageNamespace, Key: planKey,
		ContentType: "application/vnd.wrong", Revision: "1", Body: []byte("x"),
	})
	_, err = h.store.ActivationPlanRecord(1)
	requireCode(t, err, ErrCorruptRecord)
}

// TestTwoReceiverRotationSortsBothOrderings exercises the two independent
// orderings D18T maintains: D18Q grants in raw receiver-ID order, and public
// plan entries in receiver-ID-HASH order. With one receiver both are trivially
// satisfied, so only a multi-receiver rotation actually tests them.
func TestTwoReceiverRotationSortsBothOrderings(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(nextCMK)
	if err != nil {
		t.Fatal(err)
	}
	receiverARotation, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverA.AgentID(), h.receiverA.PublicKey(), repeatByte(0x54, 32), repeatByte(0x64, 24),
	)
	if err != nil {
		t.Fatal(err)
	}
	receiverBRotation, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverB.AgentID(), h.receiverB.PublicKey(), repeatByte(0x55, 32), repeatByte(0x65, 24),
	)
	if err != nil {
		t.Fatal(err)
	}
	rotation, err := channelcrypto.PlanRotation(
		[]byte("originator"), testChannelID, 0, cmk,
		[]*channelcrypto.RotationReceiver{receiverBRotation, receiverARotation}, h.signer,
	)
	if err != nil {
		t.Fatal(err)
	}

	// Deliberately pass the roster out of order; PrepareRotationCandidate must
	// sort it to match D18Q's own receiver ordering.
	prepared, err := PrepareRotationCandidate(h.definition, 0,
		[]channelstore.ReceiverIdentity{h.receiverB, h.receiverA}, rotation)
	if err != nil {
		t.Fatal(err)
	}
	defer prepared.Destroy()

	public := prepared.PublicPreparation()
	plan, err := ActivationPlanDeserialize(public.PlanBytes())
	if err != nil {
		t.Fatal(err)
	}
	entries := plan.Receivers()
	if len(entries) != 2 {
		t.Fatalf("expected 2 plan entries, got %d", len(entries))
	}
	// Plan entries must be strictly ascending by receiver hash.
	if string(entries[0].ReceiverIDHash()) >= string(entries[1].ReceiverIDHash()) {
		t.Fatal("plan entries must be strictly sorted by receiver ID hash")
	}
	if len(entries[0].GrantHash()) != 32 || len(entries[1].GrantHash()) != 32 {
		t.Fatal("grant commitments must be 32 octets")
	}
	// Grants stay in D18Q raw-receiver-ID order, which is independent of the
	// hash order above.
	grants := public.Grants()
	first, err := channelcrypto.GrantDeserialize(grants[0])
	if err != nil {
		t.Fatal(err)
	}
	second, err := channelcrypto.GrantDeserialize(grants[1])
	if err != nil {
		t.Fatal(err)
	}
	if string(first.ReceiverID()) >= string(second.ReceiverID()) {
		t.Fatal("grants must be in ascending raw receiver-ID order")
	}

	// And the whole thing replays through the real store.
	if _, err := h.store.PrepareRotation(h.definition,
		[]channelstore.ReceiverIdentity{h.receiverA, h.receiverB}, twoReceiverRotation(t, h)); err != nil {
		t.Fatal(err)
	}
}

func twoReceiverRotation(t *testing.T, h *harness) *channelcrypto.RotationPlan {
	t.Helper()
	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(nextCMK)
	if err != nil {
		t.Fatal(err)
	}
	receiverARotation, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverA.AgentID(), h.receiverA.PublicKey(), repeatByte(0x54, 32), repeatByte(0x64, 24),
	)
	if err != nil {
		t.Fatal(err)
	}
	receiverBRotation, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverB.AgentID(), h.receiverB.PublicKey(), repeatByte(0x55, 32), repeatByte(0x65, 24),
	)
	if err != nil {
		t.Fatal(err)
	}
	plan, err := channelcrypto.PlanRotation(
		[]byte("originator"), testChannelID, 0, cmk,
		[]*channelcrypto.RotationReceiver{receiverARotation, receiverBRotation}, h.signer,
	)
	if err != nil {
		t.Fatal(err)
	}
	return plan
}

func TestDuplicateReceiverInRosterIsRejected(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	_, err := h.store.PrepareRotation(h.definition,
		[]channelstore.ReceiverIdentity{h.receiverB, h.receiverB}, twoReceiverRotation(t, h))
	requireCode(t, err, ErrInvalidPlan)
}

func TestRosterMustMatchGrantReceivers(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	// One-receiver rotation for B, but a roster naming A. The grant's receiver
	// does not match, so this is invalid regardless of signature validity.
	_, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverA}, h.rotation())
	requireCode(t, err, ErrInvalidPlan)
}

func TestWithKeyLendsAndDestroysATransientCopy(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	handle, err := h.custody.ResolveHandle(testChannelID, 0)
	if err != nil || handle == nil {
		t.Fatal("epoch 0 handle must resolve")
	}
	var observed []byte
	if err := h.custody.WithKey(*handle, func(cmk *channelcrypto.ChannelMasterKey) error {
		raw, bytesErr := cmk.Bytes()
		if bytesErr != nil {
			return bytesErr
		}
		observed = append([]byte(nil), raw...)
		return nil
	}); err != nil {
		t.Fatal(err)
	}
	if !bytesEqual(observed, currentCMK) {
		t.Fatal("WithKey must lend the real CMK for the requested epoch")
	}

	// An error from the operation propagates unchanged.
	sentinel := errors.New("operation failed")
	if err := h.custody.WithKey(*handle, func(*channelcrypto.ChannelMasterKey) error {
		return sentinel
	}); !errors.Is(err, sentinel) {
		t.Fatalf("WithKey must propagate the operation error, got %v", err)
	}
}

func TestRedactedTypesNeverPrintSecrets(t *testing.T) {
	handle := NewEpochKeyHandle(testChannelID, 7)
	if handle.String() != "EpochKeyHandle([REDACTED])" || handle.GoString() != "EpochKeyHandle([REDACTED])" {
		t.Fatal("EpochKeyHandle must redact under plain and Go-syntax formatting")
	}
	if !bytesEqual(handle.ChannelID(), testChannelID) || handle.Epoch() != 7 {
		t.Fatal("handle accessors must still work")
	}

	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
	if err != nil {
		t.Fatal(err)
	}
	defer cmk.Destroy()
	public := NewPublicPreparation(testChannelID, 0, 1, []byte("plan"), [][]byte{[]byte("grant")})
	prepared, err := NewPreparedEpoch(public, cmk)
	if err != nil {
		t.Fatal(err)
	}
	defer prepared.Destroy()
	if prepared.String() != "PreparedEpoch([REDACTED])" || prepared.GoString() != "PreparedEpoch([REDACTED])" {
		t.Fatal("PreparedEpoch must redact under plain and Go-syntax formatting")
	}
}

func TestPublicPreparationIsDefensivelyCopied(t *testing.T) {
	planBytes := []byte("plan")
	grants := [][]byte{[]byte("grant-one")}
	public := NewPublicPreparation(testChannelID, 0, 1, planBytes, grants)

	// Mutating the caller's inputs must not reach the stored bundle.
	planBytes[0] = 'X'
	grants[0][0] = 'X'
	if string(public.PlanBytes()) != "plan" || string(public.Grants()[0]) != "grant-one" {
		t.Fatal("PublicPreparation must own copies of its inputs")
	}

	// And mutating what the accessors hand back must not reach it either.
	returned := public.PlanBytes()
	returned[0] = 'Y'
	if string(public.PlanBytes()) != "plan" {
		t.Fatal("accessors must return defensive copies")
	}

	if !public.Equal(public.Clone()) {
		t.Fatal("a clone must compare equal")
	}
	other := NewPublicPreparation(testChannelID, 0, 1, []byte("plan"), [][]byte{[]byte("grant-two")})
	if public.Equal(other) {
		t.Fatal("different grants must not compare equal")
	}
}

func TestWireErrorAndCodeExtraction(t *testing.T) {
	err := wireFail()
	if err.Error() != "corrupt_record" {
		t.Fatalf("wire error message %q", err.Error())
	}
	if code, ok := CodeOf(err); !ok || code != ErrCorruptRecord {
		t.Fatalf("CodeOf gave (%q, %v)", code, ok)
	}
	if code, ok := CodeOf(errInjected); ok {
		t.Fatalf("a foreign error must not yield a D18T code, got %q", code)
	}
	if IsCode(errInjected, ErrStorage) {
		t.Fatal("IsCode must be false for a foreign error")
	}
	custodyErr := &CustodyError{}
	if custodyErr.Error() != "custody_error" || custodyErr.Code() != ErrCustody {
		t.Fatal("custody error must carry custody_error")
	}
}

func TestActivationPlanRecordKeyRejectsBadChannelID(t *testing.T) {
	if _, err := ActivationPlanRecordKey([]byte{0x01}, 1); !IsCode(err, ErrCorruptRecord) {
		t.Fatalf("expected corrupt_record, got %v", err)
	}
	key, err := ActivationPlanRecordKey(testChannelID, 0)
	if err != nil {
		t.Fatal(err)
	}
	// Zero padding to 20 digits is what keeps lexicographic and numeric epoch
	// order in agreement.
	if key != "018f47a09b6c7def923456789abcdef0/epochs/00000000000000000000/activation" {
		t.Fatalf("unexpected key %q", key)
	}
}

func TestDecreasingEpochIsRejected(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	if _, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation()); err != nil {
		t.Fatal(err)
	}
	if _, err := h.store.ActivatePreparedEpoch(h.definition, 1); err != nil {
		t.Fatal(err)
	}
	// Epoch 1 is now active; recovering or activating epoch 0 goes backwards.
	_, err := h.store.RecoverPreparation(h.definition, 0)
	requireCode(t, err, ErrDecreasingEpoch)
}

func TestForeignDefinitionIsRejected(t *testing.T) {
	h := newHarness(t, false)
	h.create()
	otherChannel := mustHex("018f47a09b6c7def923456789abcdef9")
	originator, err := channelstore.NewOriginatorIdentity([]byte("originator"), h.definition.Originator().PublicKey())
	if err != nil {
		t.Fatal(err)
	}
	foreign, err := channelstore.NewChannelDefinition(
		otherChannel, originator, []channelstore.ReceiverIdentity{h.receiverA},
		1_725_000_000_000_000_000, 0, channelstore.LifecycleActive,
	)
	if err != nil {
		t.Fatal(err)
	}
	_, err = h.store.State()
	if err != nil {
		t.Fatal(err)
	}
	_, err = h.store.RecoverPreparation(foreign, 1)
	requireCode(t, err, ErrInvalidPlan)
}
