// Package epochactivation implements the portable D18T durable channel
// epoch-activation profile.
//
// # The problem D18T solves
//
// D18P makes a channel's messages, grants, cursors, and sequence reservations
// durable. D18Q can mint a fresh channel master key (CMK) for epoch E+1 and
// seal one grant per authorized receiver. Neither can make E+1 *current*.
//
// That gap is not cosmetic. Consider the obvious implementation — write a
// "current epoch" record, then publish with the new key:
//
//	crash here  ->  the new epoch is visible, but its CMK was never
//	                durably stored, so nobody can decrypt anything
//	crash here  ->  a concurrent publisher reserved a slot at epoch E
//	                while activation committed E+1; whose key is right?
//
// D18T defines the missing transaction, and does it without assuming the
// storage backend offers multi-record transactions (most object stores do not).
// Three authorities cooperate:
//
//	D18P channel store   owns public records and the publish-reservation CAS
//	injected custody     atomically owns prepared CMKs and recovery bundles
//	D18Q                 owns grant creation, parsing, and verification
//
// # The one idea worth remembering
//
// The active epoch lives in the *same versioned record* as the pending publish
// reservation. That is deliberate and it is load-bearing. A separate mutable
// "epoch head" record would not be conforming, because two independent
// compare-and-swap operations cannot exclude each other: a publisher could
// reserve a slot against the old epoch in the window between activation
// reading the head and writing it. Putting both fields behind one revision
// means publication and activation contend on a single CAS, and exactly one
// wins.
//
// This file implements the two wire formats that carry that state.
package epochactivation

import (
	"encoding/binary"
	"fmt"
	"sort"

	channelstore "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-store"
)

const (
	// EpochStateContentType tags the D18S version 2 state record. A version 1
	// reader must refuse this rather than misread it, which is why the content
	// type changes alongside the version byte.
	EpochStateContentType = "application/vnd.coding-adventures.chief-channel-state-v2"

	// ActivationPlanContentType tags the immutable D18T version 1 plan record.
	ActivationPlanContentType = "application/vnd.coding-adventures.chief-channel-epoch-activation-v1"

	// MaxPlanReceivers bounds a single rotation. A plan is O(n) in receivers
	// and is replayed on every recovery, so the bound is part of the wire
	// contract rather than a local implementation choice.
	MaxPlanReceivers = 1024

	// MaxU64 is the largest epoch or sequence value. Reaching it is a stable
	// error, never a wraparound.
	MaxU64 = ^uint64(0)

	stateMagic = "D18S"
	planMagic  = "D18T"
)

// WireError is the stable failure for a malformed or non-canonical public
// record. It always carries corrupt_record: D18T deliberately does not
// distinguish "truncated" from "bad version" from "trailing bytes" on the
// wire, because that distinction would tell an attacker how far their forgery
// got.
type WireError struct{}

func (*WireError) Error() string { return string(ErrCorruptRecord) }

// Code reports the stable D18T error code.
func (*WireError) Code() ErrorCode { return ErrCorruptRecord }

func wireFail() error { return &WireError{} }

// EpochState is the decoded D18S version 2 record: which epoch is current,
// what sequence number comes next, and whether a publish is mid-flight.
//
//	offset  field           encoding
//	0       magic           ascii("D18S")
//	4       version         0x02
//	5       active_epoch    u64be
//	13      next_sequence   u64be
//	21      pending_flag    u8: 0 none, 1 header follows
//	22      header_length   u32be, max 16384   (only when flag = 1)
//	26      reserved_header exact D18H v1 bytes (only when flag = 1)
//
// With no pending header the record is exactly 22 octets; with one it is
// exactly 26 + header_length. Trailing bytes are never permitted.
type EpochState struct {
	activeEpoch   uint64
	nextSequence  uint64
	pendingHeader *channelstore.MessageHeader
}

// NewEpochState validates every cross-field invariant D18T requires of a state
// record. A pending header must name this channel, must sit exactly one below
// next_sequence, and must have been reserved at the currently active epoch --
// that last check is what prevents a reservation from surviving across an
// activation it never agreed to.
func NewEpochState(channelID []byte, activeEpoch, nextSequence uint64, pendingHeader *channelstore.MessageHeader) (EpochState, error) {
	if len(channelID) != 16 {
		return EpochState{}, wireFail()
	}
	if pendingHeader != nil {
		header := *pendingHeader
		if !bytesEqual(header.ChannelID(), channelID) ||
			header.Sequence() == MaxU64 ||
			header.Sequence()+1 != nextSequence ||
			header.KeyEpoch() != activeEpoch {
			return EpochState{}, wireFail()
		}
		copied := header
		return EpochState{activeEpoch, nextSequence, &copied}, nil
	}
	return EpochState{activeEpoch, nextSequence, nil}, nil
}

// ActiveEpoch reports the only authority for a new publish.
func (s EpochState) ActiveEpoch() uint64 { return s.activeEpoch }

// NextSequence reports the next unused message sequence.
func (s EpochState) NextSequence() uint64 { return s.nextSequence }

// PendingHeader returns a defensive copy of the in-flight reservation, or nil.
func (s EpochState) PendingHeader() *channelstore.MessageHeader {
	if s.pendingHeader == nil {
		return nil
	}
	copied := *s.pendingHeader
	return &copied
}

// WithActiveEpoch is the activation transition: change only the epoch.
func (s EpochState) WithActiveEpoch(channelID []byte, activeEpoch uint64) (EpochState, error) {
	return NewEpochState(channelID, activeEpoch, s.nextSequence, s.pendingHeader)
}

// WithPending is the reservation transition: change only the sequence and the
// pending header, never the epoch.
func (s EpochState) WithPending(channelID []byte, nextSequence uint64, pendingHeader *channelstore.MessageHeader) (EpochState, error) {
	return NewEpochState(channelID, s.activeEpoch, nextSequence, pendingHeader)
}

// Equal compares two states by their canonical encodings, so two states are
// equal exactly when they would be stored identically.
func (s EpochState) Equal(other EpochState) bool {
	left, leftErr := EpochStateSerialize(s)
	right, rightErr := EpochStateSerialize(other)
	return leftErr == nil && rightErr == nil && bytesEqual(left, right)
}

// ActivationPlanEntry commits to one receiver and the exact grant they were
// issued, by hash. The plan carries no raw receiver ID and no grant body --
// only commitments, so the public plan record leaks neither the membership
// roster nor any key material.
type ActivationPlanEntry struct {
	receiverIDHash [32]byte
	grantHash      [32]byte
}

// NewActivationPlanEntry builds one commitment pair from 32-octet hashes.
func NewActivationPlanEntry(receiverIDHash, grantHash []byte) (ActivationPlanEntry, error) {
	if len(receiverIDHash) != 32 || len(grantHash) != 32 {
		return ActivationPlanEntry{}, wireFail()
	}
	var entry ActivationPlanEntry
	copy(entry.receiverIDHash[:], receiverIDHash)
	copy(entry.grantHash[:], grantHash)
	return entry, nil
}

// ReceiverIDHash returns SHA-256 of the raw receiver ID.
func (e ActivationPlanEntry) ReceiverIDHash() []byte {
	return append([]byte(nil), e.receiverIDHash[:]...)
}

// GrantHash returns SHA-256 of the exact D18G grant bytes.
func (e ActivationPlanEntry) GrantHash() []byte { return append([]byte(nil), e.grantHash[:]...) }

// ActivationPlan is the immutable D18T version 1 record naming the epoch
// transition and committing to every grant issued for it.
//
//	offset  field            encoding
//	0       magic            ascii("D18T")
//	4       version          0x01
//	5       channel_id       bytes[16], UUID v7
//	21      base_epoch       u64be
//	29      new_epoch        u64be
//	37      receiver_count   u32be, 1 through 1024
//	41      receivers        repeated: receiver_id_hash[32] grant_hash[32]
//
// Entries are strictly sorted by receiver_id_hash with no duplicate receiver
// or grant commitment. Strict sorting makes the encoding canonical: the same
// rotation always produces the same bytes, so a byte comparison is a complete
// equality test during replay.
type ActivationPlan struct {
	channelID [16]byte
	baseEpoch uint64
	newEpoch  uint64
	receivers []ActivationPlanEntry
}

// NewActivationPlan sorts, validates, and owns the entries.
//
// Note what the duplicate checks are really for. Two distinct receiver IDs that
// hashed to the same value would be a SHA-256 collision -- but D18T does not
// treat a collision as equal authorization, it treats it as invalid input.
// Rejecting rather than merging is the fail-closed choice.
func NewActivationPlan(channelID []byte, baseEpoch, newEpoch uint64, receivers []ActivationPlanEntry) (ActivationPlan, error) {
	if len(channelID) != 16 {
		return ActivationPlan{}, wireFail()
	}
	if baseEpoch == MaxU64 || newEpoch != baseEpoch+1 {
		return ActivationPlan{}, wireFail()
	}
	if len(receivers) < 1 || len(receivers) > MaxPlanReceivers {
		return ActivationPlan{}, wireFail()
	}
	ordered := make([]ActivationPlanEntry, len(receivers))
	copy(ordered, receivers)
	sort.Slice(ordered, func(left, right int) bool {
		return string(ordered[left].receiverIDHash[:]) < string(ordered[right].receiverIDHash[:])
	})
	seenReceiver := make(map[[32]byte]struct{}, len(ordered))
	seenGrant := make(map[[32]byte]struct{}, len(ordered))
	for _, entry := range ordered {
		if _, exists := seenReceiver[entry.receiverIDHash]; exists {
			return ActivationPlan{}, wireFail()
		}
		if _, exists := seenGrant[entry.grantHash]; exists {
			return ActivationPlan{}, wireFail()
		}
		seenReceiver[entry.receiverIDHash] = struct{}{}
		seenGrant[entry.grantHash] = struct{}{}
	}
	plan := ActivationPlan{baseEpoch: baseEpoch, newEpoch: newEpoch, receivers: ordered}
	copy(plan.channelID[:], channelID)
	return plan, nil
}

// ChannelID returns a defensive copy of the channel identifier.
func (p ActivationPlan) ChannelID() []byte { return append([]byte(nil), p.channelID[:]...) }

// BaseEpoch reports the epoch this plan rotates away from.
func (p ActivationPlan) BaseEpoch() uint64 { return p.baseEpoch }

// NewEpoch reports the epoch this plan would make current.
func (p ActivationPlan) NewEpoch() uint64 { return p.newEpoch }

// Receivers returns a defensive copy of the sorted commitment entries.
func (p ActivationPlan) Receivers() []ActivationPlanEntry {
	return append([]ActivationPlanEntry(nil), p.receivers...)
}

// Equal compares canonical encodings.
func (p ActivationPlan) Equal(other ActivationPlan) bool {
	left, leftErr := ActivationPlanSerialize(p)
	right, rightErr := ActivationPlanSerialize(other)
	return leftErr == nil && rightErr == nil && bytesEqual(left, right)
}

// EpochStateSerialize encodes canonical D18S version 2 bytes.
func EpochStateSerialize(state EpochState) ([]byte, error) {
	out := make([]byte, 0, 22)
	out = append(out, stateMagic...)
	out = append(out, 0x02)
	out = appendU64(out, state.activeEpoch)
	out = appendU64(out, state.nextSequence)
	if state.pendingHeader == nil {
		return append(out, 0x00), nil
	}
	header, err := channelstore.MessageHeaderSerialize(*state.pendingHeader)
	if err != nil {
		return nil, wireFail()
	}
	if len(header) > channelstore.MaxPendingHeaderBytes {
		return nil, wireFail()
	}
	out = append(out, 0x01)
	out = appendU32(out, uint32(len(header)))
	return append(out, header...), nil
}

// EpochStateDeserialize decodes and fully validates canonical D18S version 2
// bytes. Every rejection is corrupt_record.
func EpochStateDeserialize(data, channelID []byte) (EpochState, error) {
	reader := &byteReader{source: data}
	magic, err := reader.take(4)
	if err != nil || string(magic) != stateMagic {
		return EpochState{}, wireFail()
	}
	version, err := reader.u8()
	if err != nil || version != 2 {
		return EpochState{}, wireFail()
	}
	activeEpoch, err := reader.u64()
	if err != nil {
		return EpochState{}, wireFail()
	}
	nextSequence, err := reader.u64()
	if err != nil {
		return EpochState{}, wireFail()
	}
	flag, err := reader.u8()
	if err != nil {
		return EpochState{}, wireFail()
	}
	var pending *channelstore.MessageHeader
	switch flag {
	case 0:
	case 1:
		length, lengthErr := reader.u32()
		if lengthErr != nil || int(length) > channelstore.MaxPendingHeaderBytes {
			return EpochState{}, wireFail()
		}
		body, bodyErr := reader.take(int(length))
		if bodyErr != nil {
			return EpochState{}, wireFail()
		}
		header, headerErr := channelstore.MessageHeaderDeserialize(body)
		if headerErr != nil {
			return EpochState{}, wireFail()
		}
		pending = &header
	default:
		return EpochState{}, wireFail()
	}
	if err := reader.finish(); err != nil {
		return EpochState{}, wireFail()
	}
	return NewEpochState(channelID, activeEpoch, nextSequence, pending)
}

// ActivationPlanSerialize encodes canonical D18T version 1 bytes.
func ActivationPlanSerialize(plan ActivationPlan) ([]byte, error) {
	if len(plan.receivers) < 1 || len(plan.receivers) > MaxPlanReceivers {
		return nil, wireFail()
	}
	out := make([]byte, 0, 41+64*len(plan.receivers))
	out = append(out, planMagic...)
	out = append(out, 0x01)
	out = append(out, plan.channelID[:]...)
	out = appendU64(out, plan.baseEpoch)
	out = appendU64(out, plan.newEpoch)
	out = appendU32(out, uint32(len(plan.receivers)))
	for _, entry := range plan.receivers {
		out = append(out, entry.receiverIDHash[:]...)
		out = append(out, entry.grantHash[:]...)
	}
	return out, nil
}

// ActivationPlanDeserialize decodes canonical D18T version 1 bytes.
//
// Sort order is checked on the wire before the entries are handed to
// NewActivationPlan. That ordering matters: NewActivationPlan *sorts* its
// input, so it would happily accept a mis-ordered record and silently
// canonicalize it. Rejecting first is what makes the encoding canonical rather
// than merely normalized -- a record that is not already sorted is corrupt, not
// repairable.
func ActivationPlanDeserialize(data []byte) (ActivationPlan, error) {
	reader := &byteReader{source: data}
	magic, err := reader.take(4)
	if err != nil || string(magic) != planMagic {
		return ActivationPlan{}, wireFail()
	}
	version, err := reader.u8()
	if err != nil || version != 1 {
		return ActivationPlan{}, wireFail()
	}
	channelID, err := reader.take(16)
	if err != nil {
		return ActivationPlan{}, wireFail()
	}
	baseEpoch, err := reader.u64()
	if err != nil {
		return ActivationPlan{}, wireFail()
	}
	newEpoch, err := reader.u64()
	if err != nil {
		return ActivationPlan{}, wireFail()
	}
	count, err := reader.u32()
	if err != nil || count < 1 || count > MaxPlanReceivers {
		return ActivationPlan{}, wireFail()
	}
	entries := make([]ActivationPlanEntry, 0, count)
	for index := uint32(0); index < count; index++ {
		receiverHash, receiverErr := reader.take(32)
		if receiverErr != nil {
			return ActivationPlan{}, wireFail()
		}
		grantHash, grantErr := reader.take(32)
		if grantErr != nil {
			return ActivationPlan{}, wireFail()
		}
		entry, entryErr := NewActivationPlanEntry(receiverHash, grantHash)
		if entryErr != nil {
			return ActivationPlan{}, wireFail()
		}
		entries = append(entries, entry)
	}
	if err := reader.finish(); err != nil {
		return ActivationPlan{}, wireFail()
	}
	for index := 1; index < len(entries); index++ {
		if string(entries[index-1].receiverIDHash[:]) >= string(entries[index].receiverIDHash[:]) {
			return ActivationPlan{}, wireFail()
		}
	}
	plan, err := NewActivationPlan(channelID, baseEpoch, newEpoch, entries)
	if err != nil {
		return ActivationPlan{}, err
	}
	// NewActivationPlan re-sorted a copy; if that changed anything the record
	// was not canonical after all.
	for index := range entries {
		if entries[index] != plan.receivers[index] {
			return ActivationPlan{}, wireFail()
		}
	}
	return plan, nil
}

// ActivationPlanRecordKey builds the deterministic storage key for a plan.
// The epoch is zero-padded to 20 digits so that lexicographic key order and
// numeric epoch order agree, which is what lets a prefix listing walk epochs
// in sequence.
func ActivationPlanRecordKey(channelID []byte, newEpoch uint64) (string, error) {
	if len(channelID) != 16 {
		return "", wireFail()
	}
	return fmt.Sprintf("%x/epochs/%020d/activation", channelID, newEpoch), nil
}

type byteReader struct {
	source []byte
	offset int
}

func (r *byteReader) take(length int) ([]byte, error) {
	if length < 0 || r.offset+length > len(r.source) {
		return nil, wireFail()
	}
	out := r.source[r.offset : r.offset+length]
	r.offset += length
	return out, nil
}

func (r *byteReader) u8() (byte, error) {
	value, err := r.take(1)
	if err != nil {
		return 0, err
	}
	return value[0], nil
}

func (r *byteReader) u32() (uint32, error) {
	value, err := r.take(4)
	if err != nil {
		return 0, err
	}
	return binary.BigEndian.Uint32(value), nil
}

func (r *byteReader) u64() (uint64, error) {
	value, err := r.take(8)
	if err != nil {
		return 0, err
	}
	return binary.BigEndian.Uint64(value), nil
}

func (r *byteReader) finish() error {
	if r.offset != len(r.source) {
		return wireFail()
	}
	return nil
}

func appendU32(destination []byte, value uint32) []byte {
	var scratch [4]byte
	binary.BigEndian.PutUint32(scratch[:], value)
	return append(destination, scratch[:]...)
}

func appendU64(destination []byte, value uint64) []byte {
	var scratch [8]byte
	binary.BigEndian.PutUint64(scratch[:], value)
	return append(destination, scratch[:]...)
}

func bytesEqual(left, right []byte) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if left[index] != right[index] {
			return false
		}
	}
	return true
}
