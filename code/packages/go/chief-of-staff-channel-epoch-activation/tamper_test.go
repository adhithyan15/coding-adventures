package epochactivation

import (
	"testing"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
	channelstore "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-store"
)

// TestTamperedCustodyBundlesAreRejected is the reason validatePublicPreparation
// recomputes the plan from the grants on every replay instead of trusting the
// stored plan commitment.
//
// Custody is an injected dependency. A compromised or buggy custody backend
// could hand back a bundle whose plan no longer matches its grants. Because
// replay re-derives the plan from the grant bytes and compares, that mismatch
// is caught before anything public is written.
func TestTamperedCustodyBundlesAreRejected(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	genuine, err := PrepareRotationCandidate(h.definition, 0, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation())
	if err != nil {
		t.Fatal(err)
	}
	defer genuine.Destroy()
	public := genuine.PublicPreparation()

	for _, testCase := range []struct {
		name   string
		bundle PublicPreparation
		code   ErrorCode
	}{
		{
			name:   "plan-bytes-replaced-with-garbage",
			bundle: NewPublicPreparation(testChannelID, 0, 1, []byte("not a plan"), public.Grants()),
			code:   ErrCorruptRecord,
		},
		{
			name:   "epochs-disagree-with-plan",
			bundle: NewPublicPreparation(testChannelID, 5, 6, public.PlanBytes(), public.Grants()),
			code:   ErrInvalidPlan,
		},
		{
			name:   "grant-list-emptied",
			bundle: NewPublicPreparation(testChannelID, 0, 1, public.PlanBytes(), nil),
			code:   ErrInvalidPlan,
		},
		{
			name:   "grant-body-corrupted",
			bundle: NewPublicPreparation(testChannelID, 0, 1, public.PlanBytes(), [][]byte{[]byte("not a grant")}),
			code:   ErrCrypto,
		},
		{
			name:   "foreign-channel",
			bundle: NewPublicPreparation(mustHex("018f47a09b6c7def923456789abcdef9"), 0, 1, public.PlanBytes(), public.Grants()),
			code:   ErrInvalidPlan,
		},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			_, err := validatePublicPreparation(h.definition, testCase.bundle)
			requireCode(t, err, testCase.code)
		})
	}
}

// TestActivationRefusesWhenAGrantIsNotRetrievable is invariant 3, "all grants
// before visibility".
//
// The subtlety is that writing a grant successfully is not the same as being
// able to read it back. The record a Put echoes sits on the same trust boundary
// as the write, so against a write-behind or eventually-consistent backend an
// echoed success proves nothing. Activation must therefore re-READ every grant
// and byte-compare before advancing the epoch -- otherwise it can make E+1
// current while a receiver's grant is unretrievable, locking that receiver out
// of a channel it was authorized for.
func TestActivationRefusesWhenAGrantIsNotRetrievable(t *testing.T) {
	makeReady := func(t *testing.T) *harness {
		t.Helper()
		h := newHarness(t, false)
		h.create()
		if _, err := h.store.PrepareRotation(h.definition, []channelstore.ReceiverIdentity{h.receiverB}, h.rotation()); err != nil {
			t.Fatal(err)
		}
		return h
	}
	grantKeyFor := func(t *testing.T, h *harness) string {
		t.Helper()
		key, err := channelstore.KeyGrantRecordKey(testChannelID, 1, h.receiverB.AgentID())
		if err != nil {
			t.Fatal(err)
		}
		return key
	}

	// Each case must simulate what putImmutable structurally CANNOT observe: a
	// grant that writes successfully and then does not read back the same way.
	//
	// Corrupting an already-written record is not sufficient and is in fact a
	// vacuous test. ActivatePreparedEpoch replays through putImmutable first,
	// whose if-absent Put conflicts, re-Gets, and returns before control ever
	// reaches the phase-6 read-back loop. A test built that way passes even
	// with the entire invariant-3 check deleted -- which is exactly how the
	// first version of this test was written, and exactly what the second
	// security-review round caught.
	for _, testCase := range []struct {
		name string
		mode grantReadFault
		code ErrorCode
	}{
		{"grant-not-retrievable-after-write", grantReadVanishes, ErrCorruptRecord},
		{"grant-reads-back-different-bytes", grantReadMutatesBody, ErrCorruptRecord},
		{"grant-reads-back-wrong-content-type", grantReadMutatesContentType, ErrCorruptRecord},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			h := makeReady(t)
			faulty := &grantFaultBackend{inner: h.backend, grantKey: grantKeyFor(t, h), skipReads: 1}
			store, err := NewStoreForTesting(faulty, h.custody, testChannelID)
			if err != nil {
				t.Fatal(err)
			}
			// Arm the fault only AFTER prepare has written the grant cleanly,
			// so the write genuinely succeeds and only the read-back diverges.
			faulty.fault = testCase.mode
			_, err = store.ActivatePreparedEpoch(h.definition, 1)
			requireCode(t, err, testCase.code)

			// The epoch must not have advanced.
			faulty.fault = grantReadHealthy
			state, stateErr := store.State()
			if stateErr != nil {
				t.Fatal(stateErr)
			}
			if state.ActiveEpoch() != 0 {
				t.Fatalf("epoch advanced to %d despite an unverifiable grant", state.ActiveEpoch())
			}
		})
	}
}

type grantReadFault int

const (
	grantReadHealthy grantReadFault = iota
	grantReadVanishes
	grantReadMutatesBody
	grantReadMutatesContentType
)

// grantFaultBackend accepts writes normally but diverges on reads of one grant
// key, modelling a write-behind or eventually-consistent backend whose echoed
// Put result does not prove retrievability.
type grantFaultBackend struct {
	inner    *channelstore.MemoryChannelStorage
	grantKey string
	fault    grantReadFault

	// skipReads leaves the first N reads of the grant key healthy.
	//
	// Replay writes each grant through putImmutable, whose if-absent Put
	// conflicts on an already-present record and then re-Gets it to compare
	// bytes. That re-Get would absorb the fault and return putImmutable's own
	// conflicting_grant, so the phase-6 read-back loop would never be reached
	// and the test would be measuring the wrong check. Skipping exactly that
	// one read aims the fault at phase 6.
	skipReads int
}

func (b *grantFaultBackend) Initialize() error { return b.inner.Initialize() }

func (b *grantFaultBackend) Get(namespace, key string) (*channelstore.StorageRecord, error) {
	record, err := b.inner.Get(namespace, key)
	if err != nil || record == nil || key != b.grantKey {
		return record, err
	}
	if b.skipReads > 0 {
		b.skipReads--
		return record, nil
	}
	switch b.fault {
	case grantReadVanishes:
		return nil, nil
	case grantReadMutatesBody:
		mutated := *record
		mutated.Body = append(append([]byte(nil), record.Body...), 0x00)
		return &mutated, nil
	case grantReadMutatesContentType:
		mutated := *record
		mutated.ContentType = "application/vnd.wrong"
		return &mutated, nil
	default:
		return record, nil
	}
}

func (b *grantFaultBackend) Put(value channelstore.StoragePut) (channelstore.StorageRecord, error) {
	return b.inner.Put(value)
}

func (b *grantFaultBackend) List(namespace string, options channelstore.StorageListOptions) (channelstore.StoragePage, error) {
	return b.inner.List(namespace, options)
}

// TestGrantSignedByAnotherOriginatorIsRejected proves the D18T layer actually
// checks signatures rather than trusting whatever custody produced. It is also
// the test that would fail if VerifyGrantSignature were dropped from the
// validation path.
func TestGrantSignedByAnotherOriginatorIsRejected(t *testing.T) {
	h := newHarness(t, false)
	h.create()

	// A rotation signed by a different originator key entirely.
	impostor, err := channelcrypto.OriginatorSigningKeyFromSeed(repeatByte(0x12, 32))
	if err != nil {
		t.Fatal(err)
	}
	defer impostor.Destroy()
	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(nextCMK)
	if err != nil {
		t.Fatal(err)
	}
	receiver, err := channelcrypto.NewRotationReceiverWithMaterial(
		h.receiverB.AgentID(), h.receiverB.PublicKey(), repeatByte(0x57, 32), repeatByte(0x67, 24),
	)
	if err != nil {
		t.Fatal(err)
	}
	rotation, err := channelcrypto.PlanRotation(
		[]byte("originator"), testChannelID, 0, cmk,
		[]*channelcrypto.RotationReceiver{receiver}, impostor,
	)
	if err != nil {
		t.Fatal(err)
	}
	_, err = PrepareRotationCandidate(h.definition, 0, []channelstore.ReceiverIdentity{h.receiverB}, rotation)
	requireCode(t, err, ErrCrypto)
}

// TestStoreErrorTranslationCoversTheD18PRoster checks that foreign D18P codes
// are mapped onto the D18T roster rather than leaking through. A code with no
// D18T meaning must become storage_error, never a made-up value.
func TestStoreErrorTranslationCoversTheD18PRoster(t *testing.T) {
	for _, testCase := range []struct {
		source channelstore.ErrorCode
		want   ErrorCode
	}{
		{channelstore.ErrChannelDestroyed, ErrChannelDestroyed},
		{channelstore.ErrConflictingDefinition, ErrInvalidPlan},
		{channelstore.ErrDefinitionChanged, ErrInvalidPlan},
		{channelstore.ErrCorruptDefinition, ErrCorruptRecord},
		{channelstore.ErrCorruptRecord, ErrCorruptRecord},
		{channelstore.ErrDefinitionNotFound, ErrNotInitialized},
		{channelstore.ErrNotInitialized, ErrNotInitialized},
		// No D18T meaning: must fall through to storage_error.
		{channelstore.ErrAcknowledgementAhead, ErrStorage},
		{channelstore.ErrInvalidPageSize, ErrStorage},
		{channelstore.ErrSequenceExhausted, ErrStorage},
	} {
		t.Run(string(testCase.source), func(t *testing.T) {
			translated := translateStoreError(&channelstore.ProfileError{Code: testCase.source})
			requireCode(t, translated, testCase.want)
		})
	}
	if translateStoreError(nil) != nil {
		t.Fatal("translating a nil error must yield nil")
	}
	// Every D18T code produced by translation must be inside the closed roster.
	for _, source := range channelstore.ChannelErrorCodes {
		translated := translateStoreError(&channelstore.ProfileError{Code: source})
		code, ok := CodeOf(translated)
		if !ok {
			t.Fatalf("translation of %q produced an uncoded error", source)
		}
		if !rosterContains(code) {
			t.Fatalf("translation of %q produced %q, which is outside the D18T roster", source, code)
		}
	}
}

func rosterContains(code ErrorCode) bool {
	for _, candidate := range EpochActivationErrorCodes {
		if candidate == code {
			return true
		}
	}
	return false
}

func TestCloneCMKRejectsNilAndDestroyedKeys(t *testing.T) {
	if _, err := cloneCMK(nil); !IsCode(err, ErrCustody) {
		t.Fatalf("cloning nil must be a custody error, got %v", err)
	}
	cmk, err := channelcrypto.ChannelMasterKeyFromBytes(currentCMK)
	if err != nil {
		t.Fatal(err)
	}
	cmk.Destroy()
	if _, err := cloneCMK(cmk); !IsCode(err, ErrCustody) {
		t.Fatalf("cloning a destroyed key must be a custody error, got %v", err)
	}
}

func TestActivationPlanDeserializeRejectsBadCounts(t *testing.T) {
	channelID := testChannelID
	build := func(count uint32, entries []byte) []byte {
		body := append([]byte(nil), "D18T"...)
		body = append(body, 0x01)
		body = append(body, channelID...)
		body = appendU64(body, 0)
		body = appendU64(body, 1)
		body = appendU32(body, count)
		return append(body, entries...)
	}
	for _, testCase := range []struct {
		name string
		data []byte
	}{
		{"zero-receivers", build(0, nil)},
		{"count-over-maximum", build(MaxPlanReceivers+1, nil)},
		{"count-exceeds-body", build(2, repeatByte(0x01, 64))},
		{"body-exceeds-count", build(1, repeatByte(0x01, 128))},
		{"truncated-entry", build(1, repeatByte(0x01, 63))},
		{"wrong-plan-version", func() []byte { b := build(1, repeatByte(0x01, 64)); b[4] = 2; return b }()},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			if _, err := ActivationPlanDeserialize(testCase.data); !IsCode(err, ErrCorruptRecord) {
				t.Fatalf("expected corrupt_record, got %v", err)
			}
		})
	}
}

func TestEpochStateRejectsIncoherentPendingHeaders(t *testing.T) {
	hash := make([]byte, 32)
	header, err := channelstore.NewMessageHeader(
		testMessageID, 1, []byte("originator"), testChannelID, 5, 0, "application/octet-stream", hash,
	)
	if err != nil {
		t.Fatal(err)
	}

	t.Run("sequence-not-one-below-next", func(t *testing.T) {
		// header.Sequence()+1 == 6, but next_sequence says 99.
		if _, err := NewEpochState(testChannelID, 0, 99, &header); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	t.Run("key-epoch-disagrees-with-active", func(t *testing.T) {
		if _, err := NewEpochState(testChannelID, 7, 6, &header); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	t.Run("foreign-channel-in-header", func(t *testing.T) {
		other := mustHex("018f47a09b6c7def923456789abcdef9")
		if _, err := NewEpochState(other, 0, 6, &header); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	t.Run("bad-channel-length", func(t *testing.T) {
		if _, err := NewEpochState([]byte{0x01}, 0, 0, nil); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	t.Run("coherent-header-is-accepted", func(t *testing.T) {
		state, err := NewEpochState(testChannelID, 0, 6, &header)
		if err != nil {
			t.Fatal(err)
		}
		if state.PendingHeader() == nil || !state.PendingHeader().Equal(header) {
			t.Fatal("a coherent pending header must round-trip")
		}
		// Round-trip through the wire.
		encoded, err := EpochStateSerialize(state)
		if err != nil {
			t.Fatal(err)
		}
		decoded, err := EpochStateDeserialize(encoded, testChannelID)
		if err != nil {
			t.Fatal(err)
		}
		if !decoded.Equal(state) {
			t.Fatal("state with a pending header must survive a wire round-trip")
		}
	})
}
