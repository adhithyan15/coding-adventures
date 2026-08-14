// Package channelstore implements the portable D18P durable-channel profile.
package channelstore

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
	"sort"
	"unicode/utf8"

	sha256 "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"
)

const (
	ChannelStorageNamespace      = "chief-channels"
	ChannelDefinitionContentType = "application/vnd.coding-adventures.chief-channel-definition-v1"
	ChannelStateContentType      = "application/vnd.coding-adventures.chief-channel-state-v1"
	ChannelMessageContentType    = "application/vnd.coding-adventures.chief-channel-message-v1"
	ChannelGrantContentType      = "application/vnd.coding-adventures.chief-channel-key-grant-v1"
	ChannelAckContentType        = "application/vnd.coding-adventures.chief-channel-ack-v1"
	MaxIdentityBytes             = 4 * 1024
	MaxContentTypeBytes          = 1024
	MaxChannelReceivers          = 1024
	MaxPendingHeaderBytes        = 16 * 1024
	MaxChannelCASAttempts        = 16
	MaxDefinitionCASAttempts     = 16
)

// ErrorCode is the stable, portable failure classification defined by D18P.
type ErrorCode string

const (
	ErrInvalidDefinition         ErrorCode = "invalid_definition"
	ErrInvalidMessageID          ErrorCode = "invalid_message_id"
	ErrDefinitionNotFound        ErrorCode = "definition_not_found"
	ErrConflictingDefinition     ErrorCode = "conflicting_definition"
	ErrCorruptDefinition         ErrorCode = "corrupt_definition"
	ErrDefinitionChanged         ErrorCode = "definition_changed"
	ErrChannelDestroyed          ErrorCode = "channel_destroyed"
	ErrUnauthorizedOriginator    ErrorCode = "unauthorized_originator"
	ErrUnauthorizedReceiver      ErrorCode = "unauthorized_receiver"
	ErrPublicKeyMismatch         ErrorCode = "public_key_mismatch"
	ErrMissingKeyGrant           ErrorCode = "missing_key_grant"
	ErrUnknownMessageID          ErrorCode = "unknown_message_id"
	ErrUnauthorizedMessage       ErrorCode = "unauthorized_message"
	ErrNotInitialized            ErrorCode = "not_initialized"
	ErrCorruptRecord             ErrorCode = "corrupt_record"
	ErrPendingAppend             ErrorCode = "pending_append"
	ErrNoPendingAppend           ErrorCode = "no_pending_append"
	ErrPendingHeaderMismatch     ErrorCode = "pending_header_mismatch"
	ErrConflictingRecord         ErrorCode = "conflicting_record"
	ErrConcurrentUpdate          ErrorCode = "concurrent_update"
	ErrInvalidReceiverID         ErrorCode = "invalid_receiver_id"
	ErrInvalidPageSize           ErrorCode = "invalid_page_size"
	ErrAcknowledgementRegression ErrorCode = "acknowledgement_regression"
	ErrAcknowledgementAhead      ErrorCode = "acknowledgement_ahead"
	ErrAcknowledgementPending    ErrorCode = "acknowledgement_pending"
	ErrSequenceExhausted         ErrorCode = "sequence_exhausted"
	ErrStorage                   ErrorCode = "storage_error"
	ErrWire                      ErrorCode = "wire_error"
	ErrCrypto                    ErrorCode = "crypto_error"
	ErrMetadata                  ErrorCode = "metadata_error"
)

var ChannelErrorCodes = []ErrorCode{
	ErrInvalidDefinition, ErrInvalidMessageID, ErrDefinitionNotFound,
	ErrConflictingDefinition, ErrCorruptDefinition, ErrDefinitionChanged,
	ErrChannelDestroyed, ErrUnauthorizedOriginator, ErrUnauthorizedReceiver,
	ErrPublicKeyMismatch, ErrMissingKeyGrant, ErrUnknownMessageID,
	ErrUnauthorizedMessage, ErrNotInitialized, ErrCorruptRecord,
	ErrPendingAppend, ErrNoPendingAppend, ErrPendingHeaderMismatch,
	ErrConflictingRecord, ErrConcurrentUpdate, ErrInvalidReceiverID,
	ErrInvalidPageSize, ErrAcknowledgementRegression, ErrAcknowledgementAhead,
	ErrAcknowledgementPending, ErrSequenceExhausted, ErrStorage, ErrWire,
	ErrCrypto, ErrMetadata,
}

// ProfileError is one fail-closed D18P operation error.
type ProfileError struct{ Code ErrorCode }

func (e *ProfileError) Error() string { return string(e.Code) }

func fail(code ErrorCode) error { return &ProfileError{Code: code} }

// ErrorIs reports whether err has the requested portable code.
func ErrorIs(err error, code ErrorCode) bool {
	var profileErr *ProfileError
	return errors.As(err, &profileErr) && profileErr.Code == code
}

type ChannelLifecycle byte

const (
	LifecycleActive ChannelLifecycle = iota
	LifecycleDestroyed
)

type OriginatorIdentity struct {
	agentID   []byte
	publicKey [32]byte
}

func NewOriginatorIdentity(agentID, publicKey []byte) (OriginatorIdentity, error) {
	if len(publicKey) != 32 {
		return OriginatorIdentity{}, fail(ErrInvalidDefinition)
	}
	var key [32]byte
	copy(key[:], publicKey)
	return OriginatorIdentity{agentID: clone(agentID), publicKey: key}, nil
}

func (i OriginatorIdentity) AgentID() []byte   { return clone(i.agentID) }
func (i OriginatorIdentity) PublicKey() []byte { return clone(i.publicKey[:]) }

type ReceiverIdentity struct {
	agentID   []byte
	publicKey [32]byte
}

func NewReceiverIdentity(agentID, publicKey []byte) (ReceiverIdentity, error) {
	if len(publicKey) != 32 {
		return ReceiverIdentity{}, fail(ErrInvalidDefinition)
	}
	var key [32]byte
	copy(key[:], publicKey)
	return ReceiverIdentity{agentID: clone(agentID), publicKey: key}, nil
}

func (i ReceiverIdentity) AgentID() []byte   { return clone(i.agentID) }
func (i ReceiverIdentity) PublicKey() []byte { return clone(i.publicKey[:]) }

// ChannelDefinition is immutable canonical D18C membership.
type ChannelDefinition struct {
	channelID   [16]byte
	originator  OriginatorIdentity
	receivers   []ReceiverIdentity
	createdAtNS uint64
	keyEpoch    uint64
	lifecycle   ChannelLifecycle
}

func NewChannelDefinition(channelID []byte, originator OriginatorIdentity, receivers []ReceiverIdentity, createdAtNS, keyEpoch uint64, lifecycle ChannelLifecycle) (ChannelDefinition, error) {
	if err := validateUUIDv7(channelID, ErrInvalidDefinition); err != nil {
		return ChannelDefinition{}, err
	}
	if err := validateAgentID(originator.agentID, ErrInvalidDefinition); err != nil || len(originator.publicKey) != 32 {
		return ChannelDefinition{}, fail(ErrInvalidDefinition)
	}
	if len(receivers) < 1 || len(receivers) > MaxChannelReceivers || (lifecycle != LifecycleActive && lifecycle != LifecycleDestroyed) {
		return ChannelDefinition{}, fail(ErrInvalidDefinition)
	}
	owned := make([]ReceiverIdentity, len(receivers))
	for index, receiver := range receivers {
		if err := validateAgentID(receiver.agentID, ErrInvalidDefinition); err != nil || bytes.Equal(receiver.agentID, originator.agentID) {
			return ChannelDefinition{}, fail(ErrInvalidDefinition)
		}
		owned[index] = ReceiverIdentity{agentID: clone(receiver.agentID), publicKey: receiver.publicKey}
	}
	sort.Slice(owned, func(left, right int) bool { return bytes.Compare(owned[left].agentID, owned[right].agentID) < 0 })
	for index := 1; index < len(owned); index++ {
		if bytes.Equal(owned[index-1].agentID, owned[index].agentID) {
			return ChannelDefinition{}, fail(ErrInvalidDefinition)
		}
	}
	var id [16]byte
	copy(id[:], channelID)
	return ChannelDefinition{
		channelID:  id,
		originator: OriginatorIdentity{agentID: clone(originator.agentID), publicKey: originator.publicKey},
		receivers:  owned, createdAtNS: createdAtNS, keyEpoch: keyEpoch, lifecycle: lifecycle,
	}, nil
}

func (d ChannelDefinition) ChannelID() []byte { return clone(d.channelID[:]) }
func (d ChannelDefinition) Originator() OriginatorIdentity {
	return OriginatorIdentity{clone(d.originator.agentID), d.originator.publicKey}
}
func (d ChannelDefinition) CreatedAtNS() uint64           { return d.createdAtNS }
func (d ChannelDefinition) KeyEpoch() uint64              { return d.keyEpoch }
func (d ChannelDefinition) Lifecycle() ChannelLifecycle   { return d.lifecycle }
func (d ChannelDefinition) Receivers() []ReceiverIdentity { return cloneReceivers(d.receivers) }
func (d ChannelDefinition) Receiver(agentID []byte) (ReceiverIdentity, bool) {
	for _, receiver := range d.receivers {
		if bytes.Equal(receiver.agentID, agentID) {
			return ReceiverIdentity{clone(receiver.agentID), receiver.publicKey}, true
		}
	}
	return ReceiverIdentity{}, false
}
func (d ChannelDefinition) WithLifecycle(lifecycle ChannelLifecycle) (ChannelDefinition, error) {
	return NewChannelDefinition(d.channelID[:], d.originator, d.receivers, d.createdAtNS, d.keyEpoch, lifecycle)
}
func (d ChannelDefinition) Equal(other ChannelDefinition) bool {
	left, leftErr := ChannelDefinitionSerialize(d)
	right, rightErr := ChannelDefinitionSerialize(other)
	return leftErr == nil && rightErr == nil && bytes.Equal(left, right)
}

type MessageHeader struct {
	messageID     [16]byte
	timestampNS   uint64
	originatorID  []byte
	channelID     [16]byte
	sequence      uint64
	keyEpoch      uint64
	contentType   string
	plaintextHash [32]byte
}

func NewMessageHeader(messageID []byte, timestampNS uint64, originatorID, channelID []byte, sequence, keyEpoch uint64, contentType string, plaintextHash []byte) (MessageHeader, error) {
	if len(messageID) != 16 || len(channelID) != 16 || len(plaintextHash) != 32 || len(originatorID) > MaxIdentityBytes || !utf8.ValidString(contentType) || len([]byte(contentType)) > MaxContentTypeBytes {
		return MessageHeader{}, fail(ErrWire)
	}
	var messageArray, channelArray [16]byte
	var hash [32]byte
	copy(messageArray[:], messageID)
	copy(channelArray[:], channelID)
	copy(hash[:], plaintextHash)
	return MessageHeader{messageArray, timestampNS, clone(originatorID), channelArray, sequence, keyEpoch, contentType, hash}, nil
}

func (h MessageHeader) MessageID() []byte     { return clone(h.messageID[:]) }
func (h MessageHeader) TimestampNS() uint64   { return h.timestampNS }
func (h MessageHeader) OriginatorID() []byte  { return clone(h.originatorID) }
func (h MessageHeader) ChannelID() []byte     { return clone(h.channelID[:]) }
func (h MessageHeader) Sequence() uint64      { return h.sequence }
func (h MessageHeader) KeyEpoch() uint64      { return h.keyEpoch }
func (h MessageHeader) ContentType() string   { return h.contentType }
func (h MessageHeader) PlaintextHash() []byte { return clone(h.plaintextHash[:]) }
func (h MessageHeader) Equal(other MessageHeader) bool {
	left, leftErr := MessageHeaderSerialize(h)
	right, rightErr := MessageHeaderSerialize(other)
	return leftErr == nil && rightErr == nil && bytes.Equal(left, right)
}

type ChannelState struct {
	NextSequence  uint64
	PendingHeader *MessageHeader
}

func ChannelDefinitionSerialize(definition ChannelDefinition) ([]byte, error) {
	writer := &byteWriter{}
	writer.write([]byte("D18C"))
	writer.u8(1)
	writer.write(definition.channelID[:])
	writer.sized32(definition.originator.agentID)
	writer.write(definition.originator.publicKey[:])
	writer.u32(uint32(len(definition.receivers)))
	for _, receiver := range definition.receivers {
		writer.sized32(receiver.agentID)
		writer.write(receiver.publicKey[:])
	}
	writer.u64(definition.createdAtNS)
	writer.u64(definition.keyEpoch)
	writer.u8(byte(definition.lifecycle))
	return writer.finish(), nil
}

func ChannelDefinitionDeserialize(data []byte) (definition ChannelDefinition, err error) {
	defer func() {
		if recover() != nil {
			definition = ChannelDefinition{}
			err = fail(ErrCorruptDefinition)
		}
	}()
	reader := newByteReader(data, ErrCorruptDefinition)
	if err = reader.magic("D18C"); err != nil {
		return ChannelDefinition{}, err
	}
	if err = reader.version(); err != nil {
		return ChannelDefinition{}, err
	}
	channelID, err := reader.take(16)
	if err != nil {
		return ChannelDefinition{}, err
	}
	originatorID, err := reader.sized32(MaxIdentityBytes)
	if err != nil {
		return ChannelDefinition{}, err
	}
	publicKey, err := reader.take(32)
	if err != nil {
		return ChannelDefinition{}, err
	}
	originator, _ := NewOriginatorIdentity(originatorID, publicKey)
	count, err := reader.u32()
	if err != nil || count < 1 || count > uint32(MaxChannelReceivers) {
		return ChannelDefinition{}, fail(ErrCorruptDefinition)
	}
	receivers := make([]ReceiverIdentity, 0, count)
	for range count {
		agentID, readErr := reader.sized32(MaxIdentityBytes)
		if readErr != nil {
			return ChannelDefinition{}, fail(ErrCorruptDefinition)
		}
		key, readErr := reader.take(32)
		if readErr != nil {
			return ChannelDefinition{}, fail(ErrCorruptDefinition)
		}
		receiver, makeErr := NewReceiverIdentity(agentID, key)
		if makeErr != nil {
			return ChannelDefinition{}, fail(ErrCorruptDefinition)
		}
		receivers = append(receivers, receiver)
	}
	created, err := reader.u64()
	if err != nil {
		return ChannelDefinition{}, err
	}
	epoch, err := reader.u64()
	if err != nil {
		return ChannelDefinition{}, err
	}
	lifecycle, err := reader.u8()
	if err != nil || lifecycle > 1 {
		return ChannelDefinition{}, fail(ErrCorruptDefinition)
	}
	if err = reader.finish(); err != nil {
		return ChannelDefinition{}, err
	}
	definition, err = NewChannelDefinition(channelID, originator, receivers, created, epoch, ChannelLifecycle(lifecycle))
	if err != nil {
		return ChannelDefinition{}, fail(ErrCorruptDefinition)
	}
	return definition, nil
}

func MessageHeaderSerialize(header MessageHeader) ([]byte, error) {
	writer := &byteWriter{}
	writer.write([]byte("D18H"))
	writer.u8(1)
	writer.write(header.messageID[:])
	writer.u64(header.timestampNS)
	writer.sized32(header.originatorID)
	writer.write(header.channelID[:])
	writer.u64(header.sequence)
	writer.u64(header.keyEpoch)
	writer.sized32([]byte(header.contentType))
	writer.write(header.plaintextHash[:])
	return writer.finish(), nil
}

func MessageHeaderDeserialize(data []byte) (MessageHeader, error) {
	reader := newByteReader(data, ErrWire)
	if err := reader.magic("D18H"); err != nil {
		return MessageHeader{}, err
	}
	if err := reader.version(); err != nil {
		return MessageHeader{}, err
	}
	messageID, err := reader.take(16)
	if err != nil {
		return MessageHeader{}, err
	}
	timestamp, err := reader.u64()
	if err != nil {
		return MessageHeader{}, err
	}
	originator, err := reader.sized32(MaxIdentityBytes)
	if err != nil {
		return MessageHeader{}, err
	}
	channelID, err := reader.take(16)
	if err != nil {
		return MessageHeader{}, err
	}
	sequence, err := reader.u64()
	if err != nil {
		return MessageHeader{}, err
	}
	epoch, err := reader.u64()
	if err != nil {
		return MessageHeader{}, err
	}
	contentBytes, err := reader.sized32(MaxContentTypeBytes)
	if err != nil || !utf8.Valid(contentBytes) {
		return MessageHeader{}, fail(ErrWire)
	}
	hash, err := reader.take(32)
	if err != nil {
		return MessageHeader{}, err
	}
	if err = reader.finish(); err != nil {
		return MessageHeader{}, err
	}
	return NewMessageHeader(messageID, timestamp, originator, channelID, sequence, epoch, string(contentBytes), hash)
}

func ChannelStateSerialize(state ChannelState) ([]byte, error) {
	writer := &byteWriter{}
	writer.write([]byte("D18S"))
	writer.u8(1)
	writer.u64(state.NextSequence)
	if state.PendingHeader == nil {
		writer.u8(0)
		return writer.finish(), nil
	}
	header, err := MessageHeaderSerialize(*state.PendingHeader)
	if err != nil || len(header) > MaxPendingHeaderBytes {
		return nil, fail(ErrCorruptRecord)
	}
	writer.u8(1)
	writer.u32(uint32(len(header)))
	writer.write(header)
	return writer.finish(), nil
}

func ChannelStateDeserialize(data, channelID []byte) (ChannelState, error) {
	reader := newByteReader(data, ErrCorruptRecord)
	if err := reader.magic("D18S"); err != nil {
		return ChannelState{}, err
	}
	if err := reader.version(); err != nil {
		return ChannelState{}, err
	}
	next, err := reader.u64()
	if err != nil {
		return ChannelState{}, err
	}
	flag, err := reader.u8()
	if err != nil {
		return ChannelState{}, err
	}
	state := ChannelState{NextSequence: next}
	if flag == 0 {
		if err = reader.finish(); err != nil {
			return ChannelState{}, err
		}
		return state, nil
	}
	if flag != 1 {
		return ChannelState{}, fail(ErrCorruptRecord)
	}
	length, err := reader.u32()
	if err != nil || length > uint32(MaxPendingHeaderBytes) {
		return ChannelState{}, fail(ErrCorruptRecord)
	}
	headerBytes, err := reader.take(int(length))
	if err != nil {
		return ChannelState{}, fail(ErrCorruptRecord)
	}
	header, err := MessageHeaderDeserialize(headerBytes)
	if err != nil {
		return ChannelState{}, fail(ErrCorruptRecord)
	}
	if err = reader.finish(); err != nil || !bytes.Equal(header.channelID[:], channelID) || header.sequence == ^uint64(0) || header.sequence+1 != next {
		return ChannelState{}, fail(ErrCorruptRecord)
	}
	state.PendingHeader = &header
	return state, nil
}

func ReceiverCursorSerialize(firstUnread uint64) []byte {
	result := make([]byte, 13)
	copy(result, []byte("D18A"))
	result[4] = 1
	binary.BigEndian.PutUint64(result[5:], firstUnread)
	return result
}

func ReceiverCursorDeserialize(data []byte) (uint64, error) {
	if len(data) != 13 || !bytes.Equal(data[:4], []byte("D18A")) || data[4] != 1 {
		return 0, fail(ErrCorruptRecord)
	}
	return binary.BigEndian.Uint64(data[5:]), nil
}

func ChannelDefinitionRecordKey(channelID []byte) (string, error) {
	if len(channelID) != 16 {
		return "", fail(ErrInvalidDefinition)
	}
	return fmt.Sprintf("%x/definition", channelID), nil
}
func SequenceStateRecordKey(channelID []byte) (string, error) {
	if len(channelID) != 16 {
		return "", fail(ErrInvalidDefinition)
	}
	return fmt.Sprintf("%x/state/next-sequence", channelID), nil
}
func MessageRecordPrefix(channelID []byte) (string, error) {
	if len(channelID) != 16 {
		return "", fail(ErrInvalidDefinition)
	}
	return fmt.Sprintf("%x/messages/", channelID), nil
}
func MessageRecordKey(channelID []byte, sequence uint64) (string, error) {
	prefix, err := MessageRecordPrefix(channelID)
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("%s%020d", prefix, sequence), nil
}
func KeyGrantRecordKey(channelID []byte, epoch uint64, receiverID []byte) (string, error) {
	if err := validateAgentID(receiverID, ErrInvalidReceiverID); err != nil {
		return "", err
	}
	if len(channelID) != 16 {
		return "", fail(ErrInvalidDefinition)
	}
	hash := sha256.Sum256(receiverID)
	return fmt.Sprintf("%x/grants/%020d/%x", channelID, epoch, hash), nil
}
func ReceiverAckRecordKey(channelID, receiverID []byte) (string, error) {
	if err := validateAgentID(receiverID, ErrInvalidReceiverID); err != nil {
		return "", err
	}
	if len(channelID) != 16 {
		return "", fail(ErrInvalidDefinition)
	}
	hash := sha256.Sum256(receiverID)
	return fmt.Sprintf("%x/receivers/%x/ack", channelID, hash), nil
}

func validateUUIDv7(value []byte, code ErrorCode) error {
	if len(value) != 16 || value[6]>>4 != 7 || value[8]&0xc0 != 0x80 {
		return fail(code)
	}
	return nil
}
func validateAgentID(value []byte, code ErrorCode) error {
	if len(value) == 0 || len(value) > MaxIdentityBytes {
		return fail(code)
	}
	return nil
}
func clone(value []byte) []byte { return append([]byte(nil), value...) }
func cloneReceivers(values []ReceiverIdentity) []ReceiverIdentity {
	result := make([]ReceiverIdentity, len(values))
	for index, value := range values {
		result[index] = ReceiverIdentity{clone(value.agentID), value.publicKey}
	}
	return result
}

type byteWriter struct{ value []byte }

func (w *byteWriter) write(value []byte) { w.value = append(w.value, value...) }
func (w *byteWriter) u8(value byte)      { w.value = append(w.value, value) }
func (w *byteWriter) u32(value uint32) {
	buffer := make([]byte, 4)
	binary.BigEndian.PutUint32(buffer, value)
	w.write(buffer)
}
func (w *byteWriter) u64(value uint64) {
	buffer := make([]byte, 8)
	binary.BigEndian.PutUint64(buffer, value)
	w.write(buffer)
}
func (w *byteWriter) sized32(value []byte) { w.u32(uint32(len(value))); w.write(value) }
func (w *byteWriter) finish() []byte       { return clone(w.value) }

type byteReader struct {
	data     []byte
	position int
	code     ErrorCode
}

func newByteReader(data []byte, code ErrorCode) *byteReader {
	return &byteReader{data: clone(data), code: code}
}
func (r *byteReader) take(length int) ([]byte, error) {
	if length < 0 || r.position+length > len(r.data) {
		return nil, fail(r.code)
	}
	value := clone(r.data[r.position : r.position+length])
	r.position += length
	return value, nil
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
func (r *byteReader) sized32(maximum int) ([]byte, error) {
	length, err := r.u32()
	if err != nil || uint64(length) > uint64(maximum) {
		return nil, fail(r.code)
	}
	return r.take(int(length))
}
func (r *byteReader) magic(expected string) error {
	value, err := r.take(4)
	if err != nil || string(value) != expected {
		return fail(r.code)
	}
	return nil
}
func (r *byteReader) version() error {
	value, err := r.u8()
	if err != nil || value != 1 {
		return fail(r.code)
	}
	return nil
}
func (r *byteReader) finish() error {
	if r.position != len(r.data) {
		return fail(r.code)
	}
	return nil
}
