package channelstore

import (
	"bytes"
	"errors"
	"fmt"
	"sort"
	"sync"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
	sha256 "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"
)

type StorageRecord struct {
	Namespace, Key, ContentType, Revision string
	Body                                  []byte
}

type StoragePut struct {
	Namespace, Key, ContentType string
	Body                        []byte
	IfAbsent                    bool
	IfRevision                  string
}

type StorageListOptions struct {
	Prefix    string
	Recursive bool
	PageSize  int
	Cursor    string
}

type StoragePage struct {
	Records    []StorageRecord
	NextCursor string
}

type ChannelStorageBackend interface {
	Initialize() error
	Get(namespace, key string) (*StorageRecord, error)
	Put(value StoragePut) (StorageRecord, error)
	List(namespace string, options StorageListOptions) (StoragePage, error)
}

type StorageConflictError struct{}

func (*StorageConflictError) Error() string { return "storage condition failed" }

func IsStorageConflict(err error) bool {
	var conflict *StorageConflictError
	return errors.As(err, &conflict)
}

type MemoryChannelStorage struct {
	mu       sync.Mutex
	records  map[string]StorageRecord
	revision uint64
}

func NewMemoryChannelStorage() *MemoryChannelStorage {
	return &MemoryChannelStorage{records: make(map[string]StorageRecord)}
}

func (s *MemoryChannelStorage) Initialize() error { return nil }

func (s *MemoryChannelStorage) Get(namespace, key string) (*StorageRecord, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	record, ok := s.records[namespace+"\x00"+key]
	if !ok {
		return nil, nil
	}
	copy := cloneRecord(record)
	return &copy, nil
}

func (s *MemoryChannelStorage) Put(value StoragePut) (StorageRecord, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if value.IfAbsent == (value.IfRevision != "") {
		return StorageRecord{}, fmt.Errorf("exactly one storage condition is required")
	}
	mapKey := value.Namespace + "\x00" + value.Key
	current, exists := s.records[mapKey]
	if value.IfAbsent {
		if exists {
			return StorageRecord{}, &StorageConflictError{}
		}
	} else if !exists || current.Revision != value.IfRevision {
		return StorageRecord{}, &StorageConflictError{}
	}
	s.revision++
	record := StorageRecord{
		Namespace:   value.Namespace,
		Key:         value.Key,
		ContentType: value.ContentType,
		Body:        clone(value.Body),
		Revision:    fmt.Sprintf("r%d", s.revision),
	}
	s.records[mapKey] = record
	return cloneRecord(record), nil
}

func (s *MemoryChannelStorage) List(namespace string, options StorageListOptions) (StoragePage, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if options.PageSize <= 0 || !options.Recursive {
		return StoragePage{}, fmt.Errorf("invalid backend list options")
	}
	var records []StorageRecord
	for _, record := range s.records {
		if record.Namespace == namespace && len(record.Key) >= len(options.Prefix) && record.Key[:len(options.Prefix)] == options.Prefix && (options.Cursor == "" || record.Key > options.Cursor) {
			records = append(records, cloneRecord(record))
		}
	}
	sort.Slice(records, func(left, right int) bool { return records[left].Key < records[right].Key })
	page := StoragePage{}
	if len(records) > options.PageSize {
		page.Records = records[:options.PageSize]
		page.NextCursor = page.Records[len(page.Records)-1].Key
	} else {
		page.Records = records
	}
	return page, nil
}

func (s *MemoryChannelStorage) Corrupt(record StorageRecord) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.records[record.Namespace+"\x00"+record.Key] = cloneRecord(record)
}

type AppendRequest struct {
	MessageID, OriginatorID []byte
	TimestampNS, KeyEpoch   uint64
	ContentType             string
}

type MessagePage struct {
	Messages  []channelcrypto.D18Message
	NextStart *uint64
}

type OpaqueKeyGrant struct {
	ChannelID, ReceiverID []byte
	KeyEpoch              uint64
	Body                  []byte
}

type ChannelStore struct {
	backend   ChannelStorageBackend
	channelID []byte
}

func NewChannelStore(backend ChannelStorageBackend, channelID []byte) (*ChannelStore, error) {
	if err := validateUUIDv7(channelID, ErrCorruptRecord); err != nil {
		return nil, err
	}
	return &ChannelStore{backend: backend, channelID: clone(channelID)}, nil
}

func (s *ChannelStore) Initialize() (ChannelState, error) {
	if err := s.backend.Initialize(); err != nil {
		return ChannelState{}, fail(ErrStorage)
	}
	record, err := s.stateRecord()
	if err != nil {
		return ChannelState{}, err
	}
	if record != nil {
		return decodeStateRecord(*record, s.channelID)
	}
	body, _ := ChannelStateSerialize(ChannelState{})
	key, _ := SequenceStateRecordKey(s.channelID)
	created, err := s.backend.Put(storagePut(key, ChannelStateContentType, body, true, ""))
	if err == nil {
		return decodeStateRecord(created, s.channelID)
	}
	if IsStorageConflict(err) {
		return s.State()
	}
	return ChannelState{}, fail(ErrStorage)
}

func (s *ChannelStore) State() (ChannelState, error) {
	record, err := s.stateRecord()
	if err != nil {
		return ChannelState{}, err
	}
	if record == nil {
		return ChannelState{}, fail(ErrNotInitialized)
	}
	return decodeStateRecord(*record, s.channelID)
}

func (s *ChannelStore) ReserveAppend(request AppendRequest, plaintext []byte) (MessageHeader, error) {
	if err := validateUUIDv7(request.MessageID, ErrInvalidMessageID); err != nil {
		return MessageHeader{}, err
	}
	probe, err := channelcrypto.NewMessageFields(request.MessageID, request.TimestampNS, request.OriginatorID, s.channelID, 0, request.KeyEpoch, request.ContentType)
	if err != nil || channelcrypto.ValidateMessageFields(probe) != nil {
		return MessageHeader{}, fail(ErrCrypto)
	}
	for range MaxChannelCASAttempts {
		record, err := s.stateRecord()
		if err != nil {
			return MessageHeader{}, err
		}
		if record == nil {
			return MessageHeader{}, fail(ErrNotInitialized)
		}
		current, err := decodeStateRecord(*record, s.channelID)
		if err != nil {
			return MessageHeader{}, err
		}
		if current.PendingHeader != nil {
			return MessageHeader{}, fail(ErrPendingAppend)
		}
		if current.NextSequence == ^uint64(0) {
			return MessageHeader{}, fail(ErrSequenceExhausted)
		}
		hash := sha256.Sum256(clone(plaintext))
		header, err := NewMessageHeader(request.MessageID, request.TimestampNS, request.OriginatorID, s.channelID, current.NextSequence, request.KeyEpoch, request.ContentType, hash[:])
		if err != nil {
			return MessageHeader{}, fail(ErrCrypto)
		}
		body, err := ChannelStateSerialize(ChannelState{NextSequence: current.NextSequence + 1, PendingHeader: &header})
		if err != nil {
			return MessageHeader{}, err
		}
		key, _ := SequenceStateRecordKey(s.channelID)
		_, err = s.backend.Put(storagePut(key, ChannelStateContentType, body, false, record.Revision))
		if err == nil {
			return header, nil
		}
		if !IsStorageConflict(err) {
			return MessageHeader{}, fail(ErrStorage)
		}
	}
	return MessageHeader{}, fail(ErrConcurrentUpdate)
}

func (s *ChannelStore) CommitReserved(header MessageHeader, plaintext, channelMasterKey, signingSecretKey []byte) (channelcrypto.D18Message, error) {
	if !bytes.Equal(header.channelID[:], s.channelID) {
		return channelcrypto.D18Message{}, fail(ErrPendingHeaderMismatch)
	}
	state, err := s.State()
	if err != nil {
		return channelcrypto.D18Message{}, err
	}
	key, _ := MessageRecordKey(s.channelID, header.sequence)
	if state.PendingHeader == nil {
		record, getErr := s.backend.Get(ChannelStorageNamespace, key)
		if getErr != nil {
			return channelcrypto.D18Message{}, fail(ErrStorage)
		}
		if record == nil {
			return channelcrypto.D18Message{}, fail(ErrNoPendingAppend)
		}
		stored, err := decodeMessageRecord(*record)
		if err != nil {
			return channelcrypto.D18Message{}, err
		}
		if !messageMatchesHeader(stored, header) {
			return channelcrypto.D18Message{}, fail(ErrConflictingRecord)
		}
		expected, err := createMessage(header, plaintext, signingSecretKey, channelMasterKey)
		if err != nil {
			return channelcrypto.D18Message{}, err
		}
		want, _ := channelcrypto.MessageSerialize(expected)
		if !bytes.Equal(want, record.Body) {
			return channelcrypto.D18Message{}, fail(ErrConflictingRecord)
		}
		return stored, nil
	}
	if !state.PendingHeader.Equal(header) {
		return channelcrypto.D18Message{}, fail(ErrPendingHeaderMismatch)
	}
	message, err := createMessage(header, plaintext, signingSecretKey, channelMasterKey)
	if err != nil {
		return channelcrypto.D18Message{}, err
	}
	body, err := channelcrypto.MessageSerialize(message)
	if err != nil {
		return channelcrypto.D18Message{}, fail(ErrWire)
	}
	if err = s.putIdempotent(key, ChannelMessageContentType, body); err != nil {
		return channelcrypto.D18Message{}, err
	}
	if err = s.clearPending(header); err != nil {
		return channelcrypto.D18Message{}, err
	}
	return message, nil
}

func (s *ChannelStore) Append(request AppendRequest, plaintext, channelMasterKey, signingSecretKey []byte) (channelcrypto.D18Message, error) {
	header, err := s.ReserveAppend(request, plaintext)
	if err != nil {
		return channelcrypto.D18Message{}, err
	}
	return s.CommitReserved(header, plaintext, channelMasterKey, signingSecretKey)
}

func (s *ChannelStore) AbandonPending() (*MessageHeader, error) {
	for range MaxChannelCASAttempts {
		record, err := s.stateRecord()
		if err != nil {
			return nil, err
		}
		if record == nil {
			return nil, fail(ErrNotInitialized)
		}
		current, err := decodeStateRecord(*record, s.channelID)
		if err != nil {
			return nil, err
		}
		if current.PendingHeader == nil {
			return nil, nil
		}
		body, _ := ChannelStateSerialize(ChannelState{NextSequence: current.NextSequence})
		key, _ := SequenceStateRecordKey(s.channelID)
		_, err = s.backend.Put(storagePut(key, ChannelStateContentType, body, false, record.Revision))
		if err == nil {
			header := *current.PendingHeader
			return &header, nil
		}
		if !IsStorageConflict(err) {
			return nil, fail(ErrStorage)
		}
	}
	return nil, fail(ErrConcurrentUpdate)
}

func (s *ChannelStore) ReadMessages(start uint64, pageSize int) (MessagePage, error) {
	if pageSize <= 0 {
		return MessagePage{}, fail(ErrInvalidPageSize)
	}
	prefix, _ := MessageRecordPrefix(s.channelID)
	cursor := ""
	if start > 0 {
		cursor, _ = MessageRecordKey(s.channelID, start-1)
	}
	page, err := s.backend.List(ChannelStorageNamespace, StorageListOptions{Prefix: prefix, Recursive: true, PageSize: pageSize, Cursor: cursor})
	if err != nil {
		return MessagePage{}, fail(ErrStorage)
	}
	result := MessagePage{}
	for _, record := range page.Records {
		message, err := decodeMessageRecord(record)
		if err != nil {
			return MessagePage{}, err
		}
		expectedKey, _ := MessageRecordKey(s.channelID, message.Sequence())
		if !bytes.Equal(message.ChannelID(), s.channelID) || message.Sequence() < start || record.Key != expectedKey || (len(result.Messages) > 0 && result.Messages[len(result.Messages)-1].Sequence() >= message.Sequence()) {
			return MessagePage{}, fail(ErrCorruptRecord)
		}
		result.Messages = append(result.Messages, message)
	}
	if page.NextCursor != "" {
		if len(result.Messages) == 0 || result.Messages[len(result.Messages)-1].Sequence() == ^uint64(0) {
			return MessagePage{}, fail(ErrCorruptRecord)
		}
		next := result.Messages[len(result.Messages)-1].Sequence() + 1
		result.NextStart = &next
	}
	return result, nil
}

func (s *ChannelStore) ReadForReceiver(receiverID []byte, pageSize int) (MessagePage, error) {
	cursor, err := s.ReceiverCursor(receiverID)
	if err != nil {
		return MessagePage{}, err
	}
	return s.ReadMessages(cursor, pageSize)
}

func (s *ChannelStore) ReceiverCursor(receiverID []byte) (uint64, error) {
	if err := validateAgentID(receiverID, ErrInvalidReceiverID); err != nil {
		return 0, err
	}
	key, _ := ReceiverAckRecordKey(s.channelID, receiverID)
	record, err := s.backend.Get(ChannelStorageNamespace, key)
	if err != nil {
		return 0, fail(ErrStorage)
	}
	if record == nil {
		return 0, nil
	}
	if record.ContentType != ChannelAckContentType {
		return 0, fail(ErrCorruptRecord)
	}
	return ReceiverCursorDeserialize(record.Body)
}

func (s *ChannelStore) Acknowledge(receiverID []byte, acknowledged uint64) (uint64, error) {
	if err := validateAgentID(receiverID, ErrInvalidReceiverID); err != nil {
		return 0, err
	}
	state, err := s.State()
	if err != nil {
		return 0, err
	}
	if acknowledged >= state.NextSequence {
		return 0, fail(ErrAcknowledgementAhead)
	}
	if state.PendingHeader != nil && acknowledged >= state.PendingHeader.sequence {
		return 0, fail(ErrAcknowledgementPending)
	}
	if acknowledged == ^uint64(0) {
		return 0, fail(ErrSequenceExhausted)
	}
	desired := acknowledged + 1
	key, _ := ReceiverAckRecordKey(s.channelID, receiverID)
	for range MaxChannelCASAttempts {
		record, getErr := s.backend.Get(ChannelStorageNamespace, key)
		if getErr != nil {
			return 0, fail(ErrStorage)
		}
		if record == nil {
			_, putErr := s.backend.Put(storagePut(key, ChannelAckContentType, ReceiverCursorSerialize(desired), true, ""))
			if putErr == nil {
				return desired, nil
			}
			if IsStorageConflict(putErr) {
				continue
			}
			return 0, fail(ErrStorage)
		}
		if record.ContentType != ChannelAckContentType {
			return 0, fail(ErrCorruptRecord)
		}
		current, decodeErr := ReceiverCursorDeserialize(record.Body)
		if decodeErr != nil {
			return 0, decodeErr
		}
		if desired < current {
			return 0, fail(ErrAcknowledgementRegression)
		}
		if desired == current {
			return current, nil
		}
		_, putErr := s.backend.Put(storagePut(key, ChannelAckContentType, ReceiverCursorSerialize(desired), false, record.Revision))
		if putErr == nil {
			return desired, nil
		}
		if !IsStorageConflict(putErr) {
			return 0, fail(ErrStorage)
		}
	}
	return 0, fail(ErrConcurrentUpdate)
}

func (s *ChannelStore) SaveKeyGrant(grant OpaqueKeyGrant) error {
	if !bytes.Equal(grant.ChannelID, s.channelID) {
		return fail(ErrCorruptRecord)
	}
	if err := validateAgentID(grant.ReceiverID, ErrInvalidReceiverID); err != nil {
		return err
	}
	key, _ := KeyGrantRecordKey(s.channelID, grant.KeyEpoch, grant.ReceiverID)
	return s.putIdempotent(key, ChannelGrantContentType, grant.Body)
}

func (s *ChannelStore) KeyGrant(keyEpoch uint64, receiverID []byte) ([]byte, error) {
	if err := validateAgentID(receiverID, ErrInvalidReceiverID); err != nil {
		return nil, err
	}
	key, _ := KeyGrantRecordKey(s.channelID, keyEpoch, receiverID)
	record, err := s.backend.Get(ChannelStorageNamespace, key)
	if err != nil {
		return nil, fail(ErrStorage)
	}
	if record == nil {
		return nil, nil
	}
	if record.ContentType != ChannelGrantContentType {
		return nil, fail(ErrCorruptRecord)
	}
	return clone(record.Body), nil
}

func (s *ChannelStore) stateRecord() (*StorageRecord, error) {
	key, _ := SequenceStateRecordKey(s.channelID)
	record, err := s.backend.Get(ChannelStorageNamespace, key)
	if err != nil {
		return nil, fail(ErrStorage)
	}
	return record, nil
}

func (s *ChannelStore) putIdempotent(key, contentType string, body []byte) error {
	_, err := s.backend.Put(storagePut(key, contentType, body, true, ""))
	if err == nil {
		return nil
	}
	if !IsStorageConflict(err) {
		return fail(ErrStorage)
	}
	current, err := s.backend.Get(ChannelStorageNamespace, key)
	if err != nil {
		return fail(ErrStorage)
	}
	if current == nil || current.ContentType != contentType || !bytes.Equal(current.Body, body) {
		return fail(ErrConflictingRecord)
	}
	return nil
}

func (s *ChannelStore) clearPending(expected MessageHeader) error {
	for range MaxChannelCASAttempts {
		record, err := s.stateRecord()
		if err != nil {
			return err
		}
		if record == nil {
			return fail(ErrNotInitialized)
		}
		current, err := decodeStateRecord(*record, s.channelID)
		if err != nil {
			return err
		}
		if current.PendingHeader == nil {
			return nil
		}
		if !current.PendingHeader.Equal(expected) {
			return fail(ErrPendingHeaderMismatch)
		}
		body, _ := ChannelStateSerialize(ChannelState{NextSequence: current.NextSequence})
		key, _ := SequenceStateRecordKey(s.channelID)
		_, err = s.backend.Put(storagePut(key, ChannelStateContentType, body, false, record.Revision))
		if err == nil {
			return nil
		}
		if !IsStorageConflict(err) {
			return fail(ErrStorage)
		}
	}
	return fail(ErrConcurrentUpdate)
}

func createMessage(header MessageHeader, plaintext, signingSecretKey, channelMasterKey []byte) (channelcrypto.D18Message, error) {
	hash := sha256.Sum256(clone(plaintext))
	if !bytes.Equal(hash[:], header.plaintextHash[:]) {
		return channelcrypto.D18Message{}, fail(ErrCrypto)
	}
	fields, err := channelcrypto.NewMessageFields(header.messageID[:], header.timestampNS, header.originatorID, header.channelID[:], header.sequence, header.keyEpoch, header.contentType)
	if err != nil {
		return channelcrypto.D18Message{}, fail(ErrCrypto)
	}
	message, err := channelcrypto.MessageCreate(fields, plaintext, signingSecretKey, channelMasterKey)
	if err != nil {
		return channelcrypto.D18Message{}, fail(ErrCrypto)
	}
	return message, nil
}

func decodeMessageRecord(record StorageRecord) (channelcrypto.D18Message, error) {
	if record.ContentType != ChannelMessageContentType {
		return channelcrypto.D18Message{}, fail(ErrCorruptRecord)
	}
	message, err := channelcrypto.MessageDeserialize(record.Body)
	if err != nil {
		return channelcrypto.D18Message{}, fail(ErrWire)
	}
	return message, nil
}

func messageMatchesHeader(message channelcrypto.D18Message, header MessageHeader) bool {
	return bytes.Equal(message.MessageID(), header.messageID[:]) && message.TimestampNS() == header.timestampNS && bytes.Equal(message.OriginatorID(), header.originatorID) && bytes.Equal(message.ChannelID(), header.channelID[:]) && message.Sequence() == header.sequence && message.KeyEpoch() == header.keyEpoch && message.ContentType() == header.contentType && bytes.Equal(message.PlaintextHash(), header.plaintextHash[:])
}

func decodeStateRecord(record StorageRecord, channelID []byte) (ChannelState, error) {
	if record.ContentType != ChannelStateContentType {
		return ChannelState{}, fail(ErrCorruptRecord)
	}
	return ChannelStateDeserialize(record.Body, channelID)
}

func storagePut(key, contentType string, body []byte, ifAbsent bool, ifRevision string) StoragePut {
	return StoragePut{Namespace: ChannelStorageNamespace, Key: key, ContentType: contentType, Body: clone(body), IfAbsent: ifAbsent, IfRevision: ifRevision}
}

func cloneRecord(record StorageRecord) StorageRecord {
	record.Body = clone(record.Body)
	return record
}
