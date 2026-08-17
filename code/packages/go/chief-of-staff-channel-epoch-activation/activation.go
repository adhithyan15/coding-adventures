package epochactivation

import (
	"errors"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
	channelstore "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-store"
	sha256 "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"
)

// MaxEpochCASAttempts bounds every public compare-and-swap loop. On exhaustion
// the operation reports concurrent_update rather than writing unconditionally:
// D18T would rather make no progress than make progress by force.
const MaxEpochCASAttempts = 16

// ErrorCode is the stable, portable D18T failure classification.
type ErrorCode string

const (
	ErrNotInitialized         ErrorCode = "not_initialized"
	ErrChannelDestroyed       ErrorCode = "channel_destroyed"
	ErrInvalidPlan            ErrorCode = "invalid_plan"
	ErrCorruptRecord          ErrorCode = "corrupt_record"
	ErrPendingAppend          ErrorCode = "pending_append"
	ErrUnactivatedEpoch       ErrorCode = "unactivated_epoch"
	ErrActiveKeyMissing       ErrorCode = "active_key_missing"
	ErrConflictingActiveKey   ErrorCode = "conflicting_active_key"
	ErrPreparationMissing     ErrorCode = "preparation_missing"
	ErrConflictingPreparation ErrorCode = "conflicting_preparation"
	ErrConflictingPlan        ErrorCode = "conflicting_plan"
	ErrConflictingGrant       ErrorCode = "conflicting_grant"
	ErrUnexpectedEpoch        ErrorCode = "unexpected_epoch"
	ErrDecreasingEpoch        ErrorCode = "decreasing_epoch"
	ErrEpochExhausted         ErrorCode = "epoch_exhausted"
	ErrConcurrentUpdate       ErrorCode = "concurrent_update"
	ErrStorage                ErrorCode = "storage_error"
	ErrCustody                ErrorCode = "custody_error"
	ErrCrypto                 ErrorCode = "crypto_error"
)

// EpochActivationErrorCodes is the closed roster, in the manifest's order. The
// conformance gate compares this list byte-for-byte against the fixture.
var EpochActivationErrorCodes = []ErrorCode{
	ErrNotInitialized,
	ErrChannelDestroyed,
	ErrInvalidPlan,
	ErrCorruptRecord,
	ErrPendingAppend,
	ErrUnactivatedEpoch,
	ErrActiveKeyMissing,
	ErrConflictingActiveKey,
	ErrPreparationMissing,
	ErrConflictingPreparation,
	ErrConflictingPlan,
	ErrConflictingGrant,
	ErrUnexpectedEpoch,
	ErrDecreasingEpoch,
	ErrEpochExhausted,
	ErrConcurrentUpdate,
	ErrStorage,
	ErrCustody,
	ErrCrypto,
}

// ActivationError is a stable D18T failure. Its message is exactly the code:
// no channel bytes, no epoch numbers, no key material, nothing an operator
// might paste into a bug report and regret.
type ActivationError struct{ code ErrorCode }

func (e *ActivationError) Error() string { return string(e.code) }

// Code reports the stable code.
func (e *ActivationError) Code() ErrorCode { return e.code }

func fail(code ErrorCode) error { return &ActivationError{code: code} }

// CodeOf extracts the stable code from any D18T error, including the wire and
// custody errors, which carry their own types but the same roster.
func CodeOf(err error) (ErrorCode, bool) {
	var coded interface{ Code() ErrorCode }
	if errors.As(err, &coded) {
		return coded.Code(), true
	}
	return "", false
}

// IsCode reports whether err carries the given stable code.
func IsCode(err error, code ErrorCode) bool {
	actual, ok := CodeOf(err)
	return ok && actual == code
}

// PreparationOutcome distinguishes a fresh selection from an idempotent retry.
type PreparationOutcome string

const (
	Prepared            PreparationOutcome = "prepared"
	PreparationRepeated PreparationOutcome = "idempotent"
)

// ActivationOutcome distinguishes a committed activation from an idempotent one.
type ActivationOutcome string

const (
	Activated          ActivationOutcome = "activated"
	ActivationRepeated ActivationOutcome = "idempotent"
)

// SecretErasureCapability reports Go's honest erasure capability, which is
// inherited from D18Q rather than claimed independently.
//
// Go cannot promise more than best_effort: owned slices are cleared on
// controlled destruction, but value copies, the garbage collector, and
// intermediates inside the repository primitives are all outside this
// package's reach. The Rust reference reports "guaranteed"; overstating Go's
// position to match it would be the dishonest kind of portability.
func SecretErasureCapability() string {
	return string(channelcrypto.GrantSecretErasureCapability())
}

// ActiveEpochAppendRequest asks to publish at whatever epoch is currently
// active. KeyEpoch is optional: leave it nil to accept the active epoch, or
// set it to assert an expectation that will be checked before any encryption.
type ActiveEpochAppendRequest struct {
	MessageID    []byte
	TimestampNS  uint64
	OriginatorID []byte
	ContentType  string
	KeyEpoch     *uint64
}

// EpochReservation pairs a durable D18H reservation with the redacted handle
// for the exact epoch key it was reserved against.
type EpochReservation struct {
	Header    channelstore.MessageHeader
	KeyHandle EpochKeyHandle
}

// Store coordinates D18T over injected public storage and secret custody.
type Store struct {
	backend   channelstore.ChannelStorageBackend
	custody   OriginatorKeyCustody
	channelID []byte
}

// NewStore opens a production D18T coordinator and refuses non-durable custody.
func NewStore(backend channelstore.ChannelStorageBackend, custody OriginatorKeyCustody, channelID []byte) (*Store, error) {
	if custody == nil || !custody.Durable() {
		return nil, fail(ErrCustody)
	}
	if err := backend.Initialize(); err != nil {
		return nil, fail(ErrStorage)
	}
	return newStore(backend, custody, channelID), nil
}

// NewStoreForTesting accepts non-durable custody for deterministic tests.
func NewStoreForTesting(backend channelstore.ChannelStorageBackend, custody OriginatorKeyCustody, channelID []byte) (*Store, error) {
	if err := backend.Initialize(); err != nil {
		return nil, fail(ErrStorage)
	}
	return newStore(backend, custody, channelID), nil
}

func newStore(backend channelstore.ChannelStorageBackend, custody OriginatorKeyCustody, channelID []byte) *Store {
	return &Store{backend: backend, custody: custody, channelID: append([]byte(nil), channelID...)}
}

// CreateEpochChannel imports the initial CMK into custody *before* creating any
// D18S state.
//
// That ordering is the invariant, not an optimization. A crash between the
// import and the state write leaves no state record at all, so a retry simply
// re-imports idempotently. The reverse order would publish a channel whose
// active epoch has no resolvable key -- unrecoverable, because D18T forbids
// inventing one.
//
// The definition is settled first, though, and that ordering matters too but
// for a different reason. Custody slots are keyed by (channel_id, epoch) and
// the first writer wins permanently. If the import ran before the definition
// were checked, a caller presenting a mismatched definition could claim an
// unclaimed slot and then fail -- leaving the legitimate import to hit
// conflicting_active_key forever. Fail-closed, but permanently wedged. The
// spec only requires custody-before-*state*, so validating the definition
// first costs nothing and removes the wedge.
func (s *Store) CreateEpochChannel(definition channelstore.ChannelDefinition, initialCMK *channelcrypto.ChannelMasterKey) (EpochState, error) {
	consumed := false
	defer func() {
		if !consumed && initialCMK != nil {
			initialCMK.Destroy()
		}
	}()
	if !bytesEqual(definition.ChannelID(), s.channelID) || definition.Lifecycle() != channelstore.LifecycleActive {
		return EpochState{}, fail(ErrInvalidPlan)
	}
	definitions := channelstore.NewChannelDefinitionStore(s.backend)
	existing, err := definitions.Load(s.channelID)
	if err != nil {
		return EpochState{}, translateStoreError(err)
	}
	switch {
	case existing == nil:
		if _, err := definitions.Create(definition); err != nil {
			return EpochState{}, translateStoreError(err)
		}
	case existing.Lifecycle() == channelstore.LifecycleDestroyed:
		return EpochState{}, fail(ErrChannelDestroyed)
	case !definitionsEqual(*existing, definition):
		return EpochState{}, fail(ErrInvalidPlan)
	}
	consumed = true
	if err := s.importInitialKey(definition.KeyEpoch(), initialCMK); err != nil {
		return EpochState{}, err
	}
	return s.MigrateEpochState(definition, nil)
}

// MigrateEpochState brings a channel to D18S version 2, whether it is new or
// was created under D18P version 1.
//
// Publishing version 2 is the rolling-upgrade boundary: a version 1 process
// rejects the record rather than misreading it, so operators must deploy
// D18T-aware readers and writers before migrating. Nothing here ever clears a
// pending publish, resets a sequence, or generates key material.
func (s *Store) MigrateEpochState(definition channelstore.ChannelDefinition, currentCMK *channelcrypto.ChannelMasterKey) (EpochState, error) {
	// Own the caller's key for the whole call. Every exit path must erase it,
	// including the common steady-state path where the channel is already at
	// version 2 and the key was never needed -- an unused secret left on the
	// heap is still a secret left on the heap. ensureInitialKey nils this out
	// once it has taken ownership, so the defer never double-destroys.
	defer func() {
		if currentCMK != nil {
			currentCMK.Destroy()
		}
	}()
	if err := s.requireDefinition(definition, false); err != nil {
		return EpochState{}, err
	}
	for attempt := 0; attempt < MaxEpochCASAttempts; attempt++ {
		record, err := s.stateRecord()
		if err != nil {
			return EpochState{}, err
		}
		if record != nil && record.ContentType == EpochStateContentType {
			state, decodeErr := s.decodeV2StateRecord(*record)
			if decodeErr != nil {
				return EpochState{}, decodeErr
			}
			// An existing version 2 state is only an idempotent success once
			// custody proves its active epoch is still resolvable. Its epoch is
			// never reset from the immutable definition.
			handle, handleErr := s.resolveHandle(state.ActiveEpoch())
			if handleErr != nil {
				return EpochState{}, handleErr
			}
			if handle == nil {
				return EpochState{}, fail(ErrActiveKeyMissing)
			}
			return state, nil
		}
		if err := s.ensureInitialKey(definition.KeyEpoch(), currentCMK); err != nil {
			return EpochState{}, err
		}
		currentCMK = nil // consumed by ensureInitialKey on the first attempt

		var next EpochState
		if record == nil {
			next, err = NewEpochState(s.channelID, definition.KeyEpoch(), 0, nil)
			if err != nil {
				return EpochState{}, fail(ErrCorruptRecord)
			}
		} else {
			stateKey, keyErr := channelstore.SequenceStateRecordKey(s.channelID)
			if keyErr != nil {
				return EpochState{}, fail(ErrCorruptRecord)
			}
			if err := requireEnvelope(*record, stateKey, channelstore.ChannelStateContentType); err != nil {
				return EpochState{}, err
			}
			prior, priorErr := channelstore.ChannelStateDeserialize(record.Body, s.channelID)
			if priorErr != nil {
				return EpochState{}, fail(ErrCorruptRecord)
			}
			if prior.PendingHeader != nil && prior.PendingHeader.KeyEpoch() != definition.KeyEpoch() {
				return EpochState{}, fail(ErrCorruptRecord)
			}
			next, err = NewEpochState(s.channelID, definition.KeyEpoch(), prior.NextSequence, prior.PendingHeader)
			if err != nil {
				return EpochState{}, fail(ErrCorruptRecord)
			}
		}
		body, err := EpochStateSerialize(next)
		if err != nil {
			return EpochState{}, fail(ErrCorruptRecord)
		}
		stateKey, keyErr := channelstore.SequenceStateRecordKey(s.channelID)
		if keyErr != nil {
			return EpochState{}, fail(ErrCorruptRecord)
		}
		put := publicPut(stateKey, EpochStateContentType, body, record == nil, revisionOf(record))
		stored, err := s.backend.Put(put)
		if err != nil {
			if channelstore.IsStorageConflict(err) {
				continue
			}
			return EpochState{}, fail(ErrStorage)
		}
		return s.decodeV2StateRecord(stored)
	}
	return EpochState{}, fail(ErrConcurrentUpdate)
}

// State loads the canonical D18S version 2 state.
func (s *Store) State() (EpochState, error) {
	record, err := s.stateRecord()
	if err != nil {
		return EpochState{}, err
	}
	if record == nil {
		return EpochState{}, fail(ErrNotInitialized)
	}
	return s.decodeV2StateRecord(*record)
}

// PrepareRotation runs the full prepare-and-replay protocol for one candidate.
//
// Custody comes first, before any public write, because it is the only
// operation that both selects a winner and makes everything needed for replay
// durable in one atomic step. A crash before it leaves no candidate; a crash
// after it is fully recoverable from custody plus the public store.
func (s *Store) PrepareRotation(definition channelstore.ChannelDefinition, targetRoster []channelstore.ReceiverIdentity, rotation *channelcrypto.RotationPlan) (PreparationOutcome, error) {
	if err := s.requireDefinition(definition, false); err != nil {
		rotation.Destroy()
		return "", err
	}
	state, err := s.State()
	if err != nil {
		rotation.Destroy()
		return "", err
	}
	if state.PendingHeader() != nil {
		rotation.Destroy()
		return "", fail(ErrPendingAppend)
	}
	if state.ActiveEpoch() == MaxU64 {
		rotation.Destroy()
		return "", fail(ErrEpochExhausted)
	}
	expected := state.ActiveEpoch() + 1
	if rotation.NewEpoch() != expected {
		rotation.Destroy()
		return "", fail(ErrUnexpectedEpoch)
	}
	prepared, err := PrepareRotationCandidate(definition, state.ActiveEpoch(), targetRoster, rotation)
	if err != nil {
		return "", err
	}
	selection, custodyErr := s.custody.PrepareIfAbsent(prepared)
	prepared.Destroy()
	if custodyErr != nil {
		return "", fail(ErrCustody)
	}
	if selection == CustodyConflict {
		return "", fail(ErrConflictingPreparation)
	}
	if err := s.replayPreparation(definition, expected); err != nil {
		return "", err
	}
	if selection == CustodySelected {
		return Prepared, nil
	}
	return PreparationRepeated, nil
}

// RecoverPreparation replays the durable bundle after a crash. It never
// generates a CMK, reseals a grant, accepts replacement bytes, or picks a
// different candidate -- recovery finishes the selected plan or fails.
func (s *Store) RecoverPreparation(definition channelstore.ChannelDefinition, newEpoch uint64) (PreparationOutcome, error) {
	if err := s.requireDefinition(definition, false); err != nil {
		return "", err
	}
	state, err := s.State()
	if err != nil {
		return "", err
	}
	active := state.ActiveEpoch()
	if newEpoch < active {
		return "", fail(ErrDecreasingEpoch)
	}
	if newEpoch != active {
		if active == MaxU64 {
			return "", fail(ErrEpochExhausted)
		}
		if newEpoch != active+1 {
			return "", fail(ErrUnexpectedEpoch)
		}
	}
	if err := s.replayPreparation(definition, newEpoch); err != nil {
		return "", err
	}
	return PreparationRepeated, nil
}

// ActivatePreparedEpoch commits the epoch transition with a bounded CAS.
//
// Every precondition is re-checked inside the retry loop, because a CAS
// conflict means somebody else changed the state and any fact read before it
// may now be stale.
func (s *Store) ActivatePreparedEpoch(definition channelstore.ChannelDefinition, newEpoch uint64) (ActivationOutcome, error) {
	if err := s.requireDefinition(definition, false); err != nil {
		return "", err
	}
	prepared, err := s.custody.LoadPreparation(s.channelID, newEpoch)
	if err != nil {
		return "", fail(ErrCustody)
	}
	if prepared == nil {
		return "", fail(ErrPreparationMissing)
	}
	for attempt := 0; attempt < MaxEpochCASAttempts; attempt++ {
		if err := s.requireDefinition(definition, false); err != nil {
			return "", err
		}
		record, err := s.stateRecord()
		if err != nil {
			return "", err
		}
		if record == nil {
			return "", fail(ErrNotInitialized)
		}
		state, err := s.decodeV2StateRecord(*record)
		if err != nil {
			return "", err
		}
		active := state.ActiveEpoch()
		if active == newEpoch {
			if err := s.validateAndReplay(definition, *prepared); err != nil {
				return "", err
			}
			if _, err := s.requireHandle(newEpoch); err != nil {
				return "", err
			}
			return ActivationRepeated, nil
		}
		if active > newEpoch {
			return "", fail(ErrDecreasingEpoch)
		}
		if active == MaxU64 {
			return "", fail(ErrEpochExhausted)
		}
		if active+1 != newEpoch || prepared.BaseEpoch() != active || prepared.NewEpoch() != newEpoch {
			return "", fail(ErrUnexpectedEpoch)
		}
		if err := s.validateAndReplay(definition, *prepared); err != nil {
			return "", err
		}
		if _, err := s.requireHandle(newEpoch); err != nil {
			return "", err
		}
		// Checked last, immediately before the CAS: a reservation that landed
		// during replay must still block this activation.
		if state.PendingHeader() != nil {
			return "", fail(ErrPendingAppend)
		}
		updated, err := state.WithActiveEpoch(s.channelID, newEpoch)
		if err != nil {
			return "", fail(ErrCorruptRecord)
		}
		body, err := EpochStateSerialize(updated)
		if err != nil {
			return "", fail(ErrCorruptRecord)
		}
		stateKey, keyErr := channelstore.SequenceStateRecordKey(s.channelID)
		if keyErr != nil {
			return "", fail(ErrCorruptRecord)
		}
		stored, err := s.backend.Put(publicPut(stateKey, EpochStateContentType, body, false, record.Revision))
		if err != nil {
			if channelstore.IsStorageConflict(err) {
				continue
			}
			return "", fail(ErrStorage)
		}
		readBack, err := s.decodeV2StateRecord(stored)
		if err != nil {
			return "", err
		}
		if !readBack.Equal(updated) {
			return "", fail(ErrCorruptRecord)
		}
		return Activated, nil
	}
	return "", fail(ErrConcurrentUpdate)
}

// ReservePublishUsingActiveEpoch builds a D18H reservation bound to the current
// active epoch and its resolved key handle.
//
// This is the publication half of the shared CAS. If activation wins the race,
// this loop's Put conflicts, reloads, and rebuilds against E+1. If this wins,
// activation observes the pending header and reports pending_append. Encryption
// never falls back to an old epoch and never invents a missing key.
func (s *Store) ReservePublishUsingActiveEpoch(definition channelstore.ChannelDefinition, request ActiveEpochAppendRequest, plaintext []byte) (EpochReservation, error) {
	if err := s.requireDefinition(definition, false); err != nil {
		return EpochReservation{}, err
	}
	if !bytesEqual(request.OriginatorID, definition.Originator().AgentID()) {
		return EpochReservation{}, fail(ErrInvalidPlan)
	}
	for attempt := 0; attempt < MaxEpochCASAttempts; attempt++ {
		record, err := s.stateRecord()
		if err != nil {
			return EpochReservation{}, err
		}
		if record == nil {
			return EpochReservation{}, fail(ErrNotInitialized)
		}
		state, err := s.decodeV2StateRecord(*record)
		if err != nil {
			return EpochReservation{}, err
		}
		if request.KeyEpoch != nil && *request.KeyEpoch != state.ActiveEpoch() {
			return EpochReservation{}, fail(ErrUnactivatedEpoch)
		}
		handle, err := s.requireHandle(state.ActiveEpoch())
		if err != nil {
			return EpochReservation{}, err
		}
		if state.PendingHeader() != nil {
			return EpochReservation{}, fail(ErrPendingAppend)
		}
		if state.NextSequence() == MaxU64 {
			return EpochReservation{}, fail(ErrCrypto)
		}
		hash := sha256.Sum256(plaintext)
		header, err := channelstore.NewMessageHeader(
			request.MessageID, request.TimestampNS, request.OriginatorID, s.channelID,
			state.NextSequence(), state.ActiveEpoch(), request.ContentType, hash[:],
		)
		if err != nil {
			return EpochReservation{}, fail(ErrCrypto)
		}
		updated, err := state.WithPending(s.channelID, state.NextSequence()+1, &header)
		if err != nil {
			return EpochReservation{}, fail(ErrCorruptRecord)
		}
		body, err := EpochStateSerialize(updated)
		if err != nil {
			return EpochReservation{}, fail(ErrCorruptRecord)
		}
		stateKey, keyErr := channelstore.SequenceStateRecordKey(s.channelID)
		if keyErr != nil {
			return EpochReservation{}, fail(ErrCorruptRecord)
		}
		if _, err := s.backend.Put(publicPut(stateKey, EpochStateContentType, body, false, record.Revision)); err != nil {
			if channelstore.IsStorageConflict(err) {
				continue
			}
			return EpochReservation{}, fail(ErrStorage)
		}
		return EpochReservation{Header: header, KeyHandle: *handle}, nil
	}
	return EpochReservation{}, fail(ErrConcurrentUpdate)
}

// AbandonPending clears an in-flight reservation without publishing, releasing
// the CAS so a blocked activation can proceed. The sequence is not rewound --
// D18P sequences are append-only, so the abandoned slot simply stays empty.
func (s *Store) AbandonPending() (*channelstore.MessageHeader, error) {
	for attempt := 0; attempt < MaxEpochCASAttempts; attempt++ {
		record, err := s.stateRecord()
		if err != nil {
			return nil, err
		}
		if record == nil {
			return nil, fail(ErrNotInitialized)
		}
		state, err := s.decodeV2StateRecord(*record)
		if err != nil {
			return nil, err
		}
		pending := state.PendingHeader()
		if pending == nil {
			return nil, nil
		}
		updated, err := state.WithPending(s.channelID, state.NextSequence(), nil)
		if err != nil {
			return nil, fail(ErrCorruptRecord)
		}
		body, err := EpochStateSerialize(updated)
		if err != nil {
			return nil, fail(ErrCorruptRecord)
		}
		stateKey, keyErr := channelstore.SequenceStateRecordKey(s.channelID)
		if keyErr != nil {
			return nil, fail(ErrCorruptRecord)
		}
		if _, err := s.backend.Put(publicPut(stateKey, EpochStateContentType, body, false, record.Revision)); err != nil {
			if channelstore.IsStorageConflict(err) {
				continue
			}
			return nil, fail(ErrStorage)
		}
		return pending, nil
	}
	return nil, fail(ErrConcurrentUpdate)
}

// ActivationPlanRecord loads the immutable public plan for an epoch, or nil.
func (s *Store) ActivationPlanRecord(newEpoch uint64) (*ActivationPlan, error) {
	key, err := ActivationPlanRecordKey(s.channelID, newEpoch)
	if err != nil {
		return nil, fail(ErrCorruptRecord)
	}
	record, err := s.backend.Get(channelstore.ChannelStorageNamespace, key)
	if err != nil {
		return nil, fail(ErrStorage)
	}
	if record == nil {
		return nil, nil
	}
	if err := requireEnvelope(*record, key, ActivationPlanContentType); err != nil {
		return nil, err
	}
	plan, err := ActivationPlanDeserialize(record.Body)
	if err != nil {
		return nil, fail(ErrCorruptRecord)
	}
	if !bytesEqual(plan.ChannelID(), s.channelID) || plan.NewEpoch() != newEpoch {
		return nil, fail(ErrCorruptRecord)
	}
	return &plan, nil
}

// ApplyDestruction wipes custody for a destroyed channel while leaving every
// public plan, grant, and message exactly where it is. D18T revocation is
// prospective: it stops future access, it does not rewrite history.
func (s *Store) ApplyDestruction(definition channelstore.ChannelDefinition) error {
	if err := s.requireDefinition(definition, true); err != nil {
		return err
	}
	if err := s.custody.DestroyChannel(s.channelID); err != nil {
		return fail(ErrCustody)
	}
	return nil
}

func (s *Store) ensureInitialKey(epoch uint64, currentCMK *channelcrypto.ChannelMasterKey) error {
	handle, err := s.resolveHandle(epoch)
	if err != nil {
		return err
	}
	if handle != nil {
		if currentCMK != nil {
			currentCMK.Destroy()
		}
		return nil
	}
	if currentCMK == nil {
		return fail(ErrActiveKeyMissing)
	}
	return s.importInitialKey(epoch, currentCMK)
}

func (s *Store) importInitialKey(epoch uint64, currentCMK *channelcrypto.ChannelMasterKey) error {
	if currentCMK == nil {
		return fail(ErrActiveKeyMissing)
	}
	selection, err := s.custody.ImportActiveIfAbsent(s.channelID, epoch, currentCMK)
	currentCMK.Destroy()
	if err != nil {
		return fail(ErrCustody)
	}
	if selection == CustodyConflict {
		return fail(ErrConflictingActiveKey)
	}
	return nil
}

func (s *Store) replayPreparation(definition channelstore.ChannelDefinition, newEpoch uint64) error {
	prepared, err := s.custody.LoadPreparation(s.channelID, newEpoch)
	if err != nil {
		return fail(ErrCustody)
	}
	if prepared == nil {
		return fail(ErrPreparationMissing)
	}
	return s.validateAndReplay(definition, *prepared)
}

// validateAndReplay is replay phases 3 through 6: re-validate the durable
// bundle, write the plan and every grant with create-if-absent, then read the
// plan back and compare. Byte-identical writes are idempotent; different bytes
// at the same key are a stable conflict and are never replaced.
func (s *Store) validateAndReplay(definition channelstore.ChannelDefinition, prepared PublicPreparation) error {
	plan, err := validatePublicPreparation(definition, prepared)
	if err != nil {
		return err
	}
	planKey, err := ActivationPlanRecordKey(s.channelID, plan.NewEpoch())
	if err != nil {
		return fail(ErrCorruptRecord)
	}
	if err := s.putImmutable(planKey, ActivationPlanContentType, prepared.PlanBytes(), ErrConflictingPlan); err != nil {
		return err
	}
	for _, data := range prepared.Grants() {
		grant, grantErr := channelcrypto.GrantDeserialize(data)
		if grantErr != nil {
			return fail(ErrCrypto)
		}
		grantKey, keyErr := channelstore.KeyGrantRecordKey(s.channelID, grant.KeyEpoch(), grant.ReceiverID())
		if keyErr != nil {
			return fail(ErrCorruptRecord)
		}
		if err := s.putImmutable(grantKey, channelstore.ChannelGrantContentType, data, ErrConflictingGrant); err != nil {
			return err
		}
	}
	// Phase 6 reloads from public storage. Re-reading is not paranoia about our
	// own writes -- it is invariant 3, "all grants before visibility". The
	// record a Put echoes back sits on the same trust boundary as the write
	// itself, so against a write-behind or eventually-consistent backend an
	// echoed success does not prove the grant is retrievable. Activation may
	// only advance the epoch once every receiver's grant can actually be READ.
	stored, err := s.ActivationPlanRecord(plan.NewEpoch())
	if err != nil {
		return err
	}
	if stored == nil || !stored.Equal(plan) {
		return fail(ErrCorruptRecord)
	}
	for _, data := range prepared.Grants() {
		grant, grantErr := channelcrypto.GrantDeserialize(data)
		if grantErr != nil {
			return fail(ErrCrypto)
		}
		grantKey, keyErr := channelstore.KeyGrantRecordKey(s.channelID, grant.KeyEpoch(), grant.ReceiverID())
		if keyErr != nil {
			return fail(ErrCorruptRecord)
		}
		record, getErr := s.backend.Get(channelstore.ChannelStorageNamespace, grantKey)
		if getErr != nil {
			return fail(ErrStorage)
		}
		if record == nil {
			return fail(ErrCorruptRecord)
		}
		if err := requireEnvelope(*record, grantKey, channelstore.ChannelGrantContentType); err != nil {
			return err
		}
		if !bytesEqual(record.Body, data) {
			return fail(ErrConflictingGrant)
		}
	}
	return nil
}

func (s *Store) requireHandle(epoch uint64) (*EpochKeyHandle, error) {
	handle, err := s.resolveHandle(epoch)
	if err != nil {
		return nil, err
	}
	if handle == nil {
		return nil, fail(ErrActiveKeyMissing)
	}
	return handle, nil
}

func (s *Store) resolveHandle(epoch uint64) (*EpochKeyHandle, error) {
	handle, err := s.custody.ResolveHandle(s.channelID, epoch)
	if err != nil {
		return nil, fail(ErrCustody)
	}
	return handle, nil
}

func (s *Store) requireDefinition(expected channelstore.ChannelDefinition, requireDestroyed bool) error {
	if !bytesEqual(expected.ChannelID(), s.channelID) {
		return fail(ErrInvalidPlan)
	}
	actual, err := channelstore.NewChannelDefinitionStore(s.backend).Load(s.channelID)
	if err != nil {
		return translateStoreError(err)
	}
	if actual == nil {
		return fail(ErrNotInitialized)
	}
	if !definitionsEqual(*actual, expected) {
		return fail(ErrInvalidPlan)
	}
	if !requireDestroyed && actual.Lifecycle() == channelstore.LifecycleDestroyed {
		return fail(ErrChannelDestroyed)
	}
	if requireDestroyed && actual.Lifecycle() != channelstore.LifecycleDestroyed {
		return fail(ErrInvalidPlan)
	}
	return nil
}

func (s *Store) stateRecord() (*channelstore.StorageRecord, error) {
	key, err := channelstore.SequenceStateRecordKey(s.channelID)
	if err != nil {
		return nil, fail(ErrCorruptRecord)
	}
	record, err := s.backend.Get(channelstore.ChannelStorageNamespace, key)
	if err != nil {
		return nil, fail(ErrStorage)
	}
	return record, nil
}

func (s *Store) decodeV2StateRecord(record channelstore.StorageRecord) (EpochState, error) {
	key, err := channelstore.SequenceStateRecordKey(s.channelID)
	if err != nil {
		return EpochState{}, fail(ErrCorruptRecord)
	}
	if err := requireEnvelope(record, key, EpochStateContentType); err != nil {
		return EpochState{}, err
	}
	state, err := EpochStateDeserialize(record.Body, s.channelID)
	if err != nil {
		return EpochState{}, fail(ErrCorruptRecord)
	}
	return state, nil
}

func (s *Store) putImmutable(key, contentType string, body []byte, conflictCode ErrorCode) error {
	record, err := s.backend.Put(publicPut(key, contentType, body, true, ""))
	if err == nil {
		if envelopeErr := requireEnvelope(record, key, contentType); envelopeErr != nil {
			return envelopeErr
		}
		if !bytesEqual(record.Body, body) {
			return fail(ErrCorruptRecord)
		}
		return nil
	}
	if !channelstore.IsStorageConflict(err) {
		return fail(ErrStorage)
	}
	existing, getErr := s.backend.Get(channelstore.ChannelStorageNamespace, key)
	if getErr != nil {
		return fail(ErrStorage)
	}
	if existing == nil {
		return fail(ErrCorruptRecord)
	}
	if envelopeErr := requireEnvelope(*existing, key, contentType); envelopeErr != nil {
		return envelopeErr
	}
	if !bytesEqual(existing.Body, body) {
		return fail(conflictCode)
	}
	return nil
}

// PrepareRotationCandidate builds one pure custody candidate from a trusted
// D18Q plan and consumes that rotation.
//
// Two orderings matter here and they are different. D18Q grants are ordered by
// *raw* receiver ID, because that is the order D18Q itself produces and the
// order the grants must be replayed in. The public D18T plan entries are sorted
// by receiver ID *hash*, because the plan must not reveal the raw roster. The
// two orders are unrelated, so the code derives entries from the D18Q order and
// lets NewActivationPlan re-sort for the wire.
func PrepareRotationCandidate(definition channelstore.ChannelDefinition, baseEpoch uint64, targetRoster []channelstore.ReceiverIdentity, rotation *channelcrypto.RotationPlan) (*PreparedEpoch, error) {
	defer rotation.Destroy()

	grants := rotation.Grants()
	if len(targetRoster) < 1 || len(targetRoster) > MaxPlanReceivers || len(targetRoster) != len(grants) {
		return nil, fail(ErrInvalidPlan)
	}
	roster := make([]channelstore.ReceiverIdentity, len(targetRoster))
	copy(roster, targetRoster)
	sortReceiversByAgentID(roster)
	for index := 1; index < len(roster); index++ {
		if bytesEqual(roster[index-1].AgentID(), roster[index].AgentID()) {
			return nil, fail(ErrInvalidPlan)
		}
	}
	for index, receiver := range roster {
		grant := grants[index]
		// The epoch check lives here, not in VerifyGrantSignature: D18Q's
		// signature covers the epoch but the verifier deliberately takes no
		// expected epoch, so a validly signed grant for the wrong epoch would
		// otherwise pass. D18T step 5 owns this comparison.
		if !bytesEqual(receiver.AgentID(), grant.ReceiverID()) || grant.KeyEpoch() != rotation.NewEpoch() {
			return nil, fail(ErrInvalidPlan)
		}
		if err := verifyGrantPublic(definition, grant, receiver.AgentID()); err != nil {
			return nil, err
		}
	}
	if baseEpoch == MaxU64 {
		return nil, fail(ErrEpochExhausted)
	}
	if rotation.NewEpoch() != baseEpoch+1 {
		return nil, fail(ErrUnexpectedEpoch)
	}
	grantBytes := make([][]byte, len(grants))
	entries := make([]ActivationPlanEntry, len(grants))
	for index, grant := range grants {
		data, err := channelcrypto.GrantSerialize(grant)
		if err != nil {
			return nil, fail(ErrCrypto)
		}
		grantBytes[index] = data
		receiverHash := sha256.Sum256(grant.ReceiverID())
		grantHash := sha256.Sum256(data)
		entry, entryErr := NewActivationPlanEntry(receiverHash[:], grantHash[:])
		if entryErr != nil {
			return nil, fail(ErrInvalidPlan)
		}
		entries[index] = entry
	}
	plan, err := NewActivationPlan(definition.ChannelID(), baseEpoch, rotation.NewEpoch(), entries)
	if err != nil {
		return nil, fail(ErrInvalidPlan)
	}
	planBytes, err := ActivationPlanSerialize(plan)
	if err != nil {
		return nil, fail(ErrInvalidPlan)
	}
	public := NewPublicPreparation(definition.ChannelID(), baseEpoch, rotation.NewEpoch(), planBytes, grantBytes)
	cmk, err := rotation.NewCMK()
	if err != nil {
		return nil, fail(ErrCrypto)
	}
	defer cmk.Destroy()
	prepared, err := NewPreparedEpoch(public, cmk)
	if err != nil {
		return nil, fail(ErrCustody)
	}
	return prepared, nil
}

// validatePublicPreparation re-derives the entire plan from the durable grants
// and requires it to equal the stored plan bytes.
//
// This runs on every replay, including recovery after a crash, and it is
// deliberately not a shortcut comparison of the plan commitment. Recomputing
// from the grants is what makes a tampered custody bundle detectable.
func validatePublicPreparation(definition channelstore.ChannelDefinition, prepared PublicPreparation) (ActivationPlan, error) {
	grants := prepared.Grants()
	if !bytesEqual(prepared.ChannelID(), definition.ChannelID()) ||
		prepared.BaseEpoch() == MaxU64 ||
		prepared.NewEpoch() != prepared.BaseEpoch()+1 ||
		len(grants) < 1 || len(grants) > MaxPlanReceivers {
		return ActivationPlan{}, fail(ErrInvalidPlan)
	}
	plan, err := ActivationPlanDeserialize(prepared.PlanBytes())
	if err != nil {
		return ActivationPlan{}, fail(ErrCorruptRecord)
	}
	if !bytesEqual(plan.ChannelID(), prepared.ChannelID()) ||
		plan.BaseEpoch() != prepared.BaseEpoch() ||
		plan.NewEpoch() != prepared.NewEpoch() ||
		len(plan.Receivers()) != len(grants) {
		return ActivationPlan{}, fail(ErrInvalidPlan)
	}
	var prior []byte
	entries := make([]ActivationPlanEntry, 0, len(grants))
	for _, data := range grants {
		grant, grantErr := channelcrypto.GrantDeserialize(data)
		if grantErr != nil {
			return ActivationPlan{}, fail(ErrCrypto)
		}
		if !bytesEqual(grant.ChannelID(), prepared.ChannelID()) ||
			grant.KeyEpoch() != prepared.NewEpoch() ||
			(prior != nil && string(prior) >= string(grant.ReceiverID())) {
			return ActivationPlan{}, fail(ErrInvalidPlan)
		}
		if err := verifyGrantPublic(definition, grant, grant.ReceiverID()); err != nil {
			return ActivationPlan{}, err
		}
		prior = grant.ReceiverID()
		receiverHash := sha256.Sum256(grant.ReceiverID())
		grantHash := sha256.Sum256(data)
		entry, entryErr := NewActivationPlanEntry(receiverHash[:], grantHash[:])
		if entryErr != nil {
			return ActivationPlan{}, fail(ErrInvalidPlan)
		}
		entries = append(entries, entry)
	}
	expected, err := NewActivationPlan(prepared.ChannelID(), prepared.BaseEpoch(), prepared.NewEpoch(), entries)
	if err != nil {
		return ActivationPlan{}, fail(ErrInvalidPlan)
	}
	if !plan.Equal(expected) {
		return ActivationPlan{}, fail(ErrInvalidPlan)
	}
	return plan, nil
}

func verifyGrantPublic(definition channelstore.ChannelDefinition, grant channelcrypto.PortableKeyGrant, receiverID []byte) error {
	err := channelcrypto.VerifyGrantSignature(
		grant,
		definition.Originator().AgentID(),
		receiverID,
		definition.ChannelID(),
		definition.Originator().PublicKey(),
	)
	if err != nil {
		return fail(ErrCrypto)
	}
	return nil
}

func publicPut(key, contentType string, body []byte, ifAbsent bool, ifRevision string) channelstore.StoragePut {
	return channelstore.StoragePut{
		Namespace:   channelstore.ChannelStorageNamespace,
		Key:         key,
		ContentType: contentType,
		Body:        append([]byte(nil), body...),
		IfAbsent:    ifAbsent,
		IfRevision:  ifRevision,
	}
}

func requireEnvelope(record channelstore.StorageRecord, key, contentType string) error {
	if record.Namespace != channelstore.ChannelStorageNamespace || record.Key != key || record.ContentType != contentType {
		return fail(ErrCorruptRecord)
	}
	return nil
}

func revisionOf(record *channelstore.StorageRecord) string {
	if record == nil {
		return ""
	}
	return record.Revision
}

// translateStoreError maps D18P codes onto the D18T roster. Anything without a
// D18T meaning becomes storage_error rather than leaking a foreign code.
func translateStoreError(err error) error {
	if err == nil {
		return nil
	}
	switch {
	case channelstore.ErrorIs(err, channelstore.ErrChannelDestroyed):
		return fail(ErrChannelDestroyed)
	case channelstore.ErrorIs(err, channelstore.ErrConflictingDefinition),
		channelstore.ErrorIs(err, channelstore.ErrDefinitionChanged):
		return fail(ErrInvalidPlan)
	case channelstore.ErrorIs(err, channelstore.ErrCorruptDefinition),
		channelstore.ErrorIs(err, channelstore.ErrCorruptRecord):
		return fail(ErrCorruptRecord)
	case channelstore.ErrorIs(err, channelstore.ErrDefinitionNotFound),
		channelstore.ErrorIs(err, channelstore.ErrNotInitialized):
		return fail(ErrNotInitialized)
	default:
		return fail(ErrStorage)
	}
}

func definitionsEqual(left, right channelstore.ChannelDefinition) bool {
	leftBytes, leftErr := channelstore.ChannelDefinitionSerialize(left)
	rightBytes, rightErr := channelstore.ChannelDefinitionSerialize(right)
	return leftErr == nil && rightErr == nil && bytesEqual(leftBytes, rightBytes)
}

func sortReceiversByAgentID(receivers []channelstore.ReceiverIdentity) {
	for outer := 1; outer < len(receivers); outer++ {
		current := receivers[outer]
		inner := outer - 1
		for inner >= 0 && string(receivers[inner].AgentID()) > string(current.AgentID()) {
			receivers[inner+1] = receivers[inner]
			inner--
		}
		receivers[inner+1] = current
	}
}
