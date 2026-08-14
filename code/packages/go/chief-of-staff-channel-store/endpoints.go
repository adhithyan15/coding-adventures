package channelstore

import (
	"bytes"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
)

type MessageMetadata struct {
	MessageID   []byte
	TimestampNS uint64
}

type MessageMetadataSource interface {
	Next() (MessageMetadata, error)
}

type PublishedMessage struct {
	MessageID   []byte
	Sequence    uint64
	TimestampNS uint64
}

type ReceivedMessage struct {
	PublishedMessage
	ContentType string
	Payload     []byte
}

type ReceiverEpochKeyProvider interface {
	PublicKey() []byte
	OpenGrant(keyEpoch uint64, grantBody []byte) ([]byte, error)
}

type ChannelDefinitionStore struct{ backend ChannelStorageBackend }

func NewChannelDefinitionStore(backend ChannelStorageBackend) *ChannelDefinitionStore {
	return &ChannelDefinitionStore{backend: backend}
}

func (s *ChannelDefinitionStore) Create(definition ChannelDefinition) (ChannelDefinition, error) {
	if definition.lifecycle != LifecycleActive {
		return ChannelDefinition{}, fail(ErrInvalidDefinition)
	}
	if err := s.backend.Initialize(); err != nil {
		return ChannelDefinition{}, fail(ErrStorage)
	}
	key, _ := ChannelDefinitionRecordKey(definition.channelID[:])
	body, _ := ChannelDefinitionSerialize(definition)
	record, err := s.backend.Put(storagePut(key, ChannelDefinitionContentType, body, true, ""))
	var persisted ChannelDefinition
	if err == nil {
		persisted, err = requireDefinitionRecord(record, definition.channelID[:])
	} else if IsStorageConflict(err) {
		existing, getErr := s.backend.Get(ChannelStorageNamespace, key)
		if getErr != nil {
			return ChannelDefinition{}, fail(ErrStorage)
		}
		if existing == nil {
			return ChannelDefinition{}, fail(ErrDefinitionNotFound)
		}
		if existing.ContentType != ChannelDefinitionContentType {
			return ChannelDefinition{}, fail(ErrCorruptDefinition)
		}
		if !bytes.Equal(existing.Body, body) {
			return ChannelDefinition{}, fail(ErrConflictingDefinition)
		}
		persisted, err = requireDefinitionRecord(*existing, definition.channelID[:])
	} else {
		return ChannelDefinition{}, fail(ErrStorage)
	}
	if err != nil {
		return ChannelDefinition{}, err
	}
	if !persisted.Equal(definition) {
		return ChannelDefinition{}, fail(ErrConflictingDefinition)
	}
	store, _ := NewChannelStore(s.backend, definition.channelID[:])
	if _, err = store.Initialize(); err != nil {
		return ChannelDefinition{}, err
	}
	return s.RequireCurrent(definition)
}

func (s *ChannelDefinitionStore) Load(channelID []byte) (*ChannelDefinition, error) {
	if err := s.backend.Initialize(); err != nil {
		return nil, fail(ErrStorage)
	}
	loaded, err := s.loadRecord(channelID)
	if err != nil || loaded == nil {
		return nil, err
	}
	definition := loaded.definition
	return &definition, nil
}

func (s *ChannelDefinitionStore) Destroy(channelID []byte) (ChannelDefinition, error) {
	if err := s.backend.Initialize(); err != nil {
		return ChannelDefinition{}, fail(ErrStorage)
	}
	for range MaxDefinitionCASAttempts {
		loaded, err := s.loadRecord(channelID)
		if err != nil {
			return ChannelDefinition{}, err
		}
		if loaded == nil {
			return ChannelDefinition{}, fail(ErrDefinitionNotFound)
		}
		if loaded.definition.lifecycle == LifecycleDestroyed {
			return loaded.definition, nil
		}
		destroyed, _ := loaded.definition.WithLifecycle(LifecycleDestroyed)
		body, _ := ChannelDefinitionSerialize(destroyed)
		key, _ := ChannelDefinitionRecordKey(channelID)
		record, putErr := s.backend.Put(storagePut(key, ChannelDefinitionContentType, body, false, loaded.revision))
		if putErr == nil {
			return requireDefinitionRecord(record, channelID)
		}
		if !IsStorageConflict(putErr) {
			return ChannelDefinition{}, fail(ErrStorage)
		}
	}
	return ChannelDefinition{}, fail(ErrConcurrentUpdate)
}

func (s *ChannelDefinitionStore) RequireCurrent(expected ChannelDefinition) (ChannelDefinition, error) {
	actual, err := s.Load(expected.channelID[:])
	if err != nil {
		return ChannelDefinition{}, err
	}
	if actual == nil {
		return ChannelDefinition{}, fail(ErrDefinitionNotFound)
	}
	if actual.lifecycle == LifecycleDestroyed {
		return ChannelDefinition{}, fail(ErrChannelDestroyed)
	}
	if !actual.Equal(expected) {
		return ChannelDefinition{}, fail(ErrDefinitionChanged)
	}
	return *actual, nil
}

type loadedDefinition struct {
	definition ChannelDefinition
	revision   string
}

func (s *ChannelDefinitionStore) loadRecord(channelID []byte) (*loadedDefinition, error) {
	key, err := ChannelDefinitionRecordKey(channelID)
	if err != nil {
		return nil, err
	}
	record, err := s.backend.Get(ChannelStorageNamespace, key)
	if err != nil {
		return nil, fail(ErrStorage)
	}
	if record == nil {
		return nil, nil
	}
	definition, err := requireDefinitionRecord(*record, channelID)
	if err != nil {
		return nil, err
	}
	return &loadedDefinition{definition: definition, revision: record.Revision}, nil
}

type DurableOriginator struct {
	backend          ChannelStorageBackend
	definition       ChannelDefinition
	signingSecretKey []byte
	channelMasterKey []byte
	metadataSource   MessageMetadataSource
}

func OpenDurableOriginator(backend ChannelStorageBackend, channelID, agentID, signingSecretKey, channelMasterKey []byte, metadataSource MessageMetadataSource) (*DurableOriginator, error) {
	definition, err := activeDefinition(backend, channelID)
	if err != nil {
		return nil, err
	}
	if !bytes.Equal(definition.originator.agentID, agentID) {
		return nil, fail(ErrUnauthorizedOriginator)
	}
	if len(signingSecretKey) != 64 || !bytes.Equal(definition.originator.publicKey[:], signingSecretKey[32:]) {
		return nil, fail(ErrPublicKeyMismatch)
	}
	if len(channelMasterKey) != 32 {
		return nil, fail(ErrCrypto)
	}
	store, _ := NewChannelStore(backend, channelID)
	if _, err = store.Initialize(); err != nil {
		return nil, err
	}
	return &DurableOriginator{backend: backend, definition: definition, signingSecretKey: clone(signingSecretKey), channelMasterKey: clone(channelMasterKey), metadataSource: metadataSource}, nil
}

func (o *DurableOriginator) ID() []byte        { return clone(o.definition.originator.agentID) }
func (o *DurableOriginator) ChannelID() []byte { return clone(o.definition.channelID[:]) }
func (o *DurableOriginator) PublicKey() []byte { return clone(o.definition.originator.publicKey[:]) }

func (o *DurableOriginator) Publish(payload []byte, contentType string) (PublishedMessage, error) {
	metadata, err := o.metadataSource.Next()
	if err != nil {
		return PublishedMessage{}, fail(ErrMetadata)
	}
	return o.PublishWithMetadata(metadata, payload, contentType)
}

func (o *DurableOriginator) PublishWithMetadata(metadata MessageMetadata, payload []byte, contentType string) (PublishedMessage, error) {
	if err := validateUUIDv7(metadata.MessageID, ErrInvalidMessageID); err != nil {
		return PublishedMessage{}, err
	}
	if _, err := NewChannelDefinitionStore(o.backend).RequireCurrent(o.definition); err != nil {
		return PublishedMessage{}, err
	}
	store, _ := NewChannelStore(o.backend, o.definition.channelID[:])
	message, err := store.Append(AppendRequest{MessageID: clone(metadata.MessageID), TimestampNS: metadata.TimestampNS, OriginatorID: clone(o.definition.originator.agentID), KeyEpoch: o.definition.keyEpoch, ContentType: contentType}, payload, o.channelMasterKey, o.signingSecretKey)
	if err != nil {
		return PublishedMessage{}, err
	}
	return PublishedMessage{MessageID: clone(metadata.MessageID), Sequence: message.Sequence(), TimestampNS: metadata.TimestampNS}, nil
}

func (o *DurableOriginator) SaveReceiverGrant(receiverID, grantBody []byte) error {
	definition, err := NewChannelDefinitionStore(o.backend).RequireCurrent(o.definition)
	if err != nil {
		return err
	}
	if _, ok := definition.Receiver(receiverID); !ok {
		return fail(ErrUnauthorizedReceiver)
	}
	store, _ := NewChannelStore(o.backend, definition.channelID[:])
	return store.SaveKeyGrant(OpaqueKeyGrant{ChannelID: definition.channelID[:], KeyEpoch: definition.keyEpoch, ReceiverID: receiverID, Body: grantBody})
}

type DurableReceiver struct {
	backend     ChannelStorageBackend
	definition  ChannelDefinition
	receiverID  []byte
	keyProvider ReceiverEpochKeyProvider
	delivered   map[string]uint64
}

func OpenDurableReceiver(backend ChannelStorageBackend, channelID, receiverID []byte, keyProvider ReceiverEpochKeyProvider) (*DurableReceiver, error) {
	if err := validateAgentID(receiverID, ErrInvalidReceiverID); err != nil {
		return nil, err
	}
	definition, err := activeDefinition(backend, channelID)
	if err != nil {
		return nil, err
	}
	receiver, ok := definition.Receiver(receiverID)
	if !ok {
		return nil, fail(ErrUnauthorizedReceiver)
	}
	if !bytes.Equal(receiver.publicKey[:], keyProvider.PublicKey()) {
		return nil, fail(ErrPublicKeyMismatch)
	}
	store, _ := NewChannelStore(backend, channelID)
	if _, err = store.Initialize(); err != nil {
		return nil, err
	}
	return &DurableReceiver{backend: backend, definition: definition, receiverID: clone(receiverID), keyProvider: keyProvider, delivered: make(map[string]uint64)}, nil
}

func (r *DurableReceiver) ID() []byte        { return clone(r.receiverID) }
func (r *DurableReceiver) ChannelID() []byte { return clone(r.definition.channelID[:]) }
func (r *DurableReceiver) PublicKey() []byte { return clone(r.keyProvider.PublicKey()) }

func (r *DurableReceiver) Receive(limit int) ([]ReceivedMessage, error) {
	if _, err := NewChannelDefinitionStore(r.backend).RequireCurrent(r.definition); err != nil {
		return nil, err
	}
	store, _ := NewChannelStore(r.backend, r.definition.channelID[:])
	page, err := store.ReadForReceiver(r.receiverID, limit)
	if err != nil {
		return nil, err
	}
	result := make([]ReceivedMessage, 0, len(page.Messages))
	for _, message := range page.Messages {
		if !bytes.Equal(message.ChannelID(), r.definition.channelID[:]) || !bytes.Equal(message.OriginatorID(), r.definition.originator.agentID) || message.KeyEpoch() > r.definition.keyEpoch {
			return nil, fail(ErrUnauthorizedMessage)
		}
		grant, err := store.KeyGrant(message.KeyEpoch(), r.receiverID)
		if err != nil {
			return nil, err
		}
		if grant == nil {
			return nil, fail(ErrMissingKeyGrant)
		}
		channelKey, err := r.keyProvider.OpenGrant(message.KeyEpoch(), grant)
		if err != nil {
			return nil, fail(ErrCrypto)
		}
		if channelKey == nil {
			return nil, fail(ErrMissingKeyGrant)
		}
		payload, err := channelcrypto.MessageVerify(message, r.definition.originator.publicKey[:], channelKey)
		if err != nil {
			return nil, fail(ErrCrypto)
		}
		if err = validateUUIDv7(message.MessageID(), ErrInvalidMessageID); err != nil {
			return nil, err
		}
		id := string(message.MessageID())
		if previous, ok := r.delivered[id]; ok && previous != message.Sequence() {
			return nil, fail(ErrUnauthorizedMessage)
		}
		r.delivered[id] = message.Sequence()
		result = append(result, ReceivedMessage{PublishedMessage: PublishedMessage{MessageID: message.MessageID(), Sequence: message.Sequence(), TimestampNS: message.TimestampNS()}, ContentType: message.ContentType(), Payload: clone(payload)})
	}
	return result, nil
}

func (r *DurableReceiver) Acknowledge(messageID []byte) (uint64, error) {
	if err := validateUUIDv7(messageID, ErrInvalidMessageID); err != nil {
		return 0, err
	}
	if _, err := NewChannelDefinitionStore(r.backend).RequireCurrent(r.definition); err != nil {
		return 0, err
	}
	sequence, ok := r.delivered[string(messageID)]
	if !ok {
		return 0, fail(ErrUnknownMessageID)
	}
	store, _ := NewChannelStore(r.backend, r.definition.channelID[:])
	return store.Acknowledge(r.receiverID, sequence)
}

func activeDefinition(backend ChannelStorageBackend, channelID []byte) (ChannelDefinition, error) {
	definition, err := NewChannelDefinitionStore(backend).Load(channelID)
	if err != nil {
		return ChannelDefinition{}, err
	}
	if definition == nil {
		return ChannelDefinition{}, fail(ErrDefinitionNotFound)
	}
	if definition.lifecycle == LifecycleDestroyed {
		return ChannelDefinition{}, fail(ErrChannelDestroyed)
	}
	return *definition, nil
}

func requireDefinitionRecord(record StorageRecord, channelID []byte) (ChannelDefinition, error) {
	if record.ContentType != ChannelDefinitionContentType {
		return ChannelDefinition{}, fail(ErrCorruptDefinition)
	}
	definition, err := ChannelDefinitionDeserialize(record.Body)
	if err != nil {
		return ChannelDefinition{}, err
	}
	key, _ := ChannelDefinitionRecordKey(channelID)
	if !bytes.Equal(definition.channelID[:], channelID) || record.Key != key {
		return ChannelDefinition{}, fail(ErrCorruptDefinition)
	}
	return definition, nil
}
