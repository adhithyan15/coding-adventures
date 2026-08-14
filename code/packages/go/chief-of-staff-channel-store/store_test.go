package channelstore

import (
	"bytes"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"os"
	"strconv"
	"testing"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
	ed25519 "github.com/example/coding-adventures/code/packages/go/ed25519"
)

type fixtureManifest struct {
	FixtureFormat     string           `json:"fixture_format"`
	GeneratorBlobSHA1 string           `json:"generator_blob_sha1"`
	Constants         fixtureConstants `json:"constants"`
	StableErrorCodes  []ErrorCode      `json:"stable_error_codes"`
	DefinitionCases   []struct {
		Name                 string   `json:"name"`
		Lifecycle            string   `json:"lifecycle"`
		D18C                 string   `json:"d18c_b64"`
		CanonicalReceiverIDs []string `json:"canonical_receiver_ids_b64"`
	} `json:"definition_cases"`
	StateCases []struct {
		Name         string `json:"name"`
		NextSequence string `json:"next_sequence"`
		D18S         string `json:"d18s_b64"`
		D18H         string `json:"d18h_b64"`
		Pending      bool   `json:"pending"`
	} `json:"state_cases"`
	CursorCases []struct {
		FirstUnread string `json:"first_unread_sequence"`
		D18A        string `json:"d18a_b64"`
	} `json:"cursor_cases"`
	StorageKeyCases []struct {
		Name        string `json:"name"`
		ExpectedKey string `json:"expected_key"`
	} `json:"storage_key_cases"`
	CodecNegativeCases []struct {
		Name          string `json:"name"`
		Kind          string `json:"kind"`
		Record        string `json:"record_b64"`
		ExpectedError string `json:"expected_error"`
	} `json:"codec_negative_cases"`
	OversizeRecipes []struct {
		Field          string `json:"field"`
		DeclaredLength string `json:"declared_length"`
		ExpectedError  string `json:"expected_error"`
	} `json:"oversize_recipes"`
	OperationCases    []map[string]any `json:"operation_cases"`
	OperationNegative []struct {
		Name          string `json:"name"`
		ExpectedError string `json:"expected_error"`
	} `json:"operation_negative_cases"`
	TestKeys struct {
		OriginatorSeed   string `json:"originator_signing_seed_hex"`
		OriginatorPublic string `json:"originator_public_key_hex"`
		MasterKey        string `json:"channel_master_key_hex"`
	} `json:"test_keys"`
}

type fixtureConstants struct {
	StorageNamespace string            `json:"storage_namespace"`
	ContentTypes     map[string]string `json:"content_types"`
	MaxReceivers     string            `json:"max_receivers"`
	MaxPending       string            `json:"max_pending_header_bytes"`
	MaxStoreCAS      string            `json:"max_store_cas_attempts"`
	MaxDefinitionCAS string            `json:"max_definition_cas_attempts"`
}

type testContext struct {
	fixture                                                   fixtureManifest
	definition                                                ChannelDefinition
	channelID, originatorID, binaryReceiverID, textReceiverID []byte
	signingSecret, masterKey                                  []byte
}

func loadTestContext(t *testing.T) testContext {
	t.Helper()
	data, err := os.ReadFile("../../../fixtures/chief-of-staff-channel/v1/manifest.json")
	if err != nil {
		t.Fatal(err)
	}
	var fixture fixtureManifest
	if err = json.Unmarshal(data, &fixture); err != nil {
		t.Fatal(err)
	}
	active := decodeB64(t, fixture.DefinitionCases[0].D18C)
	definition, err := ChannelDefinitionDeserialize(active)
	if err != nil {
		t.Fatal(err)
	}
	seedBytes, _ := hex.DecodeString(fixture.TestKeys.OriginatorSeed)
	var seed [32]byte
	copy(seed[:], seedBytes)
	_, secret := ed25519.GenerateKeypair(seed)
	master, _ := hex.DecodeString(fixture.TestKeys.MasterKey)
	return testContext{fixture: fixture, definition: definition, channelID: definition.ChannelID(), originatorID: definition.Originator().AgentID(), binaryReceiverID: decodeB64(t, fixture.DefinitionCases[0].CanonicalReceiverIDs[0]), textReceiverID: decodeB64(t, fixture.DefinitionCases[0].CanonicalReceiverIDs[1]), signingSecret: secret[:], masterKey: master}
}

func decodeB64(t *testing.T, value string) []byte {
	t.Helper()
	decoded, err := base64.StdEncoding.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return decoded
}

func requireCode(t *testing.T, err error, code ErrorCode) {
	t.Helper()
	if !ErrorIs(err, code) {
		t.Fatalf("got %v, want %s", err, code)
	}
	if err.Error() != string(code) {
		t.Fatalf("error leaked details: %q", err)
	}
}

func TestFixtureCodecsKeysBoundsAndErrorRoster(t *testing.T) {
	c := loadTestContext(t)
	if c.fixture.FixtureFormat != "D18P-durable-channel-fixtures-v1" || len(c.fixture.GeneratorBlobSHA1) != 40 {
		t.Fatal("fixture provenance mismatch")
	}
	if c.fixture.Constants.StorageNamespace != ChannelStorageNamespace ||
		c.fixture.Constants.ContentTypes["definition"] != ChannelDefinitionContentType ||
		c.fixture.Constants.ContentTypes["state"] != ChannelStateContentType ||
		c.fixture.Constants.ContentTypes["message"] != ChannelMessageContentType ||
		c.fixture.Constants.ContentTypes["grant"] != ChannelGrantContentType ||
		c.fixture.Constants.ContentTypes["ack"] != ChannelAckContentType ||
		c.fixture.Constants.MaxReceivers != strconv.Itoa(MaxChannelReceivers) ||
		c.fixture.Constants.MaxPending != strconv.Itoa(MaxPendingHeaderBytes) ||
		c.fixture.Constants.MaxStoreCAS != strconv.Itoa(MaxChannelCASAttempts) ||
		c.fixture.Constants.MaxDefinitionCAS != strconv.Itoa(MaxDefinitionCASAttempts) {
		t.Fatal("fixture constants mismatch")
	}
	if !equalCodes(c.fixture.StableErrorCodes, ChannelErrorCodes) {
		t.Fatal("stable error roster mismatch")
	}
	expectedOperations := map[string]ErrorCode{
		"conflicting-definition":        ErrConflictingDefinition,
		"session-delivery-enforcement":  ErrUnknownMessageID,
		"unauthorized-originator":       ErrUnauthorizedOriginator,
		"unauthorized-receiver":         ErrUnauthorizedReceiver,
		"receiver-public-key-mismatch":  ErrPublicKeyMismatch,
		"channel-destroyed":             ErrChannelDestroyed,
		"missing-key-grant":             ErrMissingKeyGrant,
		"pending-append":                ErrPendingAppend,
		"acknowledgement-pending":       ErrAcknowledgementPending,
		"pending-header-mismatch":       ErrPendingHeaderMismatch,
		"no-pending-append":             ErrNoPendingAppend,
		"invalid-page-size":             ErrInvalidPageSize,
		"invalid-receiver-id":           ErrInvalidReceiverID,
		"acknowledgement-ahead":         ErrAcknowledgementAhead,
		"acknowledgement-regression":    ErrAcknowledgementRegression,
		"message-key-body-mismatch":     ErrCorruptRecord,
		"message-content-type-mismatch": ErrCorruptRecord,
	}
	if len(c.fixture.OperationNegative) != len(expectedOperations) {
		t.Fatal("operation error roster size mismatch")
	}
	for _, item := range c.fixture.OperationNegative {
		if expectedOperations[item.Name] != ErrorCode(item.ExpectedError) {
			t.Fatalf("operation error %s mismatch", item.Name)
		}
	}
	for _, item := range c.fixture.DefinitionCases {
		encoded := decodeB64(t, item.D18C)
		definition, err := ChannelDefinitionDeserialize(encoded)
		if err != nil {
			t.Fatal(err)
		}
		actual, _ := ChannelDefinitionSerialize(definition)
		if !bytes.Equal(actual, encoded) {
			t.Fatalf("definition %s changed", item.Name)
		}
	}
	for _, item := range c.fixture.StateCases {
		encoded := decodeB64(t, item.D18S)
		state, err := ChannelStateDeserialize(encoded, c.channelID)
		if err != nil {
			t.Fatal(err)
		}
		want, _ := strconv.ParseUint(item.NextSequence, 10, 64)
		if state.NextSequence != want || (state.PendingHeader != nil) != item.Pending {
			t.Fatalf("state %s mismatch", item.Name)
		}
		actual, _ := ChannelStateSerialize(state)
		if !bytes.Equal(actual, encoded) {
			t.Fatalf("state %s changed", item.Name)
		}
	}
	for _, item := range c.fixture.CursorCases {
		encoded := decodeB64(t, item.D18A)
		cursor, err := ReceiverCursorDeserialize(encoded)
		if err != nil {
			t.Fatal(err)
		}
		want, _ := strconv.ParseUint(item.FirstUnread, 10, 64)
		if cursor != want || !bytes.Equal(ReceiverCursorSerialize(cursor), encoded) {
			t.Fatal("cursor mismatch")
		}
	}
	actualKeys := map[string]string{}
	actualKeys["definition"], _ = ChannelDefinitionRecordKey(c.channelID)
	actualKeys["state"], _ = SequenceStateRecordKey(c.channelID)
	actualKeys["message-zero"], _ = MessageRecordKey(c.channelID, 0)
	actualKeys["message-max"], _ = MessageRecordKey(c.channelID, ^uint64(0))
	actualKeys["message-prefix"], _ = MessageRecordPrefix(c.channelID)
	actualKeys["grant"], _ = KeyGrantRecordKey(c.channelID, 7, c.binaryReceiverID)
	actualKeys["ack-binary-receiver"], _ = ReceiverAckRecordKey(c.channelID, c.binaryReceiverID)
	for _, item := range c.fixture.StorageKeyCases {
		if actualKeys[item.Name] != item.ExpectedKey {
			t.Fatalf("key %s mismatch", item.Name)
		}
	}
	for _, item := range c.fixture.CodecNegativeCases {
		t.Run(item.Name, func(t *testing.T) {
			data := decodeB64(t, item.Record)
			var err error
			switch item.Kind {
			case "definition":
				_, err = ChannelDefinitionDeserialize(data)
			case "state":
				_, err = ChannelStateDeserialize(data, c.channelID)
			default:
				_, err = ReceiverCursorDeserialize(data)
			}
			requireCode(t, err, ErrorCode(item.ExpectedError))
		})
	}
	oversizedID := make([]byte, MaxIdentityBytes+1)
	originator, _ := NewOriginatorIdentity(oversizedID, c.definition.Originator().PublicKey())
	_, err := NewChannelDefinition(c.channelID, originator, c.definition.Receivers(), 0, 0, LifecycleActive)
	requireCode(t, err, ErrInvalidDefinition)
	receivers := make([]ReceiverIdentity, MaxChannelReceivers+1)
	for index := range receivers {
		receivers[index], _ = NewReceiverIdentity([]byte{byte(index >> 8), byte(index)}, make([]byte, 32))
	}
	_, err = NewChannelDefinition(c.channelID, c.definition.Originator(), receivers, 0, 0, LifecycleActive)
	requireCode(t, err, ErrInvalidDefinition)
	_, err = ChannelStateDeserialize([]byte{68, 49, 56, 83, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 64, 1}, c.channelID)
	requireCode(t, err, ErrCorruptRecord)
}

func equalCodes(left, right []ErrorCode) bool {
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

func uuid7(value byte) []byte {
	result := bytes.Repeat([]byte{value}, 16)
	result[6] = 0x70 | value&0x0f
	result[8] = 0x80 | value&0x3f
	return result
}

func request(c testContext, value byte, timestamp uint64) AppendRequest {
	return AppendRequest{MessageID: uuid7(value), TimestampNS: timestamp, OriginatorID: c.originatorID, ContentType: "text/plain"}
}

func operationCase(c testContext, name string) map[string]any {
	for _, item := range c.fixture.OperationCases {
		if item["name"] == name {
			return item
		}
	}
	return nil
}

func TestStoreRecoveryRetryGapPagingAndAcknowledgement(t *testing.T) {
	c := loadTestContext(t)
	backend := NewMemoryChannelStorage()
	store, _ := NewChannelStore(backend, c.channelID)
	state, err := store.Initialize()
	if err != nil || state.NextSequence != 0 {
		t.Fatal(err)
	}
	header, err := store.ReserveAppend(request(c, 20, 20_000_000_020), []byte("recoverable"))
	if err != nil {
		t.Fatal(err)
	}
	recovered, _ := NewChannelStore(backend, c.channelID)
	state, err = recovered.Initialize()
	if err != nil || state.PendingHeader == nil || !state.PendingHeader.Equal(header) {
		t.Fatal("pending recovery mismatch")
	}
	_, err = store.ReserveAppend(request(c, 21, 20_000_000_021), []byte("pending"))
	requireCode(t, err, ErrPendingAppend)
	_, err = store.Acknowledge(c.binaryReceiverID, 0)
	requireCode(t, err, ErrAcknowledgementPending)
	mismatch, _ := NewMessageHeader(uuid7(22), 20_000_000_022, c.originatorID, c.channelID, 0, 0, "text/plain", header.PlaintextHash())
	_, err = recovered.CommitReserved(mismatch, []byte("recoverable"), c.masterKey, c.signingSecret)
	requireCode(t, err, ErrPendingHeaderMismatch)
	first, err := recovered.CommitReserved(header, []byte("recoverable"), c.masterKey, c.signingSecret)
	if err != nil {
		t.Fatal(err)
	}
	retry, err := recovered.CommitReserved(header, []byte("recoverable"), c.masterKey, c.signingSecret)
	if err != nil {
		t.Fatal(err)
	}
	firstBytes, _ := channelcrypto.MessageSerialize(first)
	retryBytes, _ := channelcrypto.MessageSerialize(retry)
	if !bytes.Equal(firstBytes, retryBytes) {
		t.Fatal("commit retry changed bytes")
	}
	expected := operationCase(c, "reserve-recover-complete-retry-abandon-gap")
	if !bytes.Equal(firstBytes, decodeB64(t, expected["first_d18m_b64"].(string))) {
		t.Fatal("D18M fixture mismatch")
	}
	abandoned, err := recovered.ReserveAppend(request(c, 23, 20_000_000_023), []byte("abandoned"))
	if err != nil {
		t.Fatal(err)
	}
	cleared, err := recovered.AbandonPending()
	if err != nil || cleared == nil || cleared.Sequence() != 1 {
		t.Fatal("abandon mismatch")
	}
	_, err = recovered.CommitReserved(abandoned, []byte("abandoned"), c.masterKey, c.signingSecret)
	requireCode(t, err, ErrNoPendingAppend)
	afterGap, err := recovered.Append(request(c, 24, 20_000_000_024), []byte("after gap"), c.masterKey, c.signingSecret)
	if err != nil || afterGap.Sequence() != 2 {
		t.Fatal("gap was not permanent")
	}
	page, err := recovered.ReadMessages(0, 10)
	if err != nil || len(page.Messages) != 2 || page.Messages[0].Sequence() != 0 || page.Messages[1].Sequence() != 2 {
		t.Fatal("ordered paging mismatch")
	}
	page, err = recovered.ReadMessages(0, 1)
	if err != nil || page.NextStart == nil || *page.NextStart != 1 {
		t.Fatal("continuation mismatch")
	}
	page, err = recovered.ReadMessages(*page.NextStart, 1)
	if err != nil || len(page.Messages) != 1 || page.Messages[0].Sequence() != 2 {
		t.Fatal("gap continuation mismatch")
	}
	page, err = recovered.ReadMessages(2, 10)
	if err != nil || len(page.Messages) != 1 {
		t.Fatal("random access mismatch")
	}
	page, err = recovered.ReadMessages(3, 10)
	if err != nil || len(page.Messages) != 0 {
		t.Fatal("empty continuation mismatch")
	}
	_, err = recovered.ReadMessages(0, 0)
	requireCode(t, err, ErrInvalidPageSize)
	_, err = recovered.Acknowledge(c.binaryReceiverID, 3)
	requireCode(t, err, ErrAcknowledgementAhead)
	if cursor, err := recovered.Acknowledge(c.binaryReceiverID, 0); err != nil || cursor != 1 {
		t.Fatal(err)
	}
	if cursor, err := recovered.Acknowledge(c.binaryReceiverID, 2); err != nil || cursor != 3 {
		t.Fatal(err)
	}
	_, err = recovered.Acknowledge(c.binaryReceiverID, 0)
	requireCode(t, err, ErrAcknowledgementRegression)
	_, err = recovered.ReceiverCursor(nil)
	requireCode(t, err, ErrInvalidReceiverID)
}

type metadataSource struct{ values []MessageMetadata }

func (s *metadataSource) Next() (MessageMetadata, error) {
	if len(s.values) == 0 {
		return MessageMetadata{}, os.ErrNotExist
	}
	value := s.values[0]
	s.values = s.values[1:]
	return value, nil
}

type receiverProvider struct {
	publicKey, key []byte
	fail           bool
}

func (p receiverProvider) PublicKey() []byte { return clone(p.publicKey) }
func (p receiverProvider) OpenGrant(uint64, []byte) ([]byte, error) {
	if p.fail {
		return nil, os.ErrPermission
	}
	return clone(p.key), nil
}

func provider(c testContext, receiverID []byte) receiverProvider {
	receiver, _ := c.definition.Receiver(receiverID)
	return receiverProvider{publicKey: receiver.PublicKey(), key: c.masterKey}
}

func TestEndpointsIndependentCursorsSessionsAndDestroy(t *testing.T) {
	c := loadTestContext(t)
	backend := NewMemoryChannelStorage()
	definitions := NewChannelDefinitionStore(backend)
	created, err := definitions.Create(c.definition)
	if err != nil || !created.Equal(c.definition) {
		t.Fatal(err)
	}
	retry, err := definitions.Create(c.definition)
	if err != nil || !retry.Equal(created) {
		t.Fatal(err)
	}
	conflict, _ := NewChannelDefinition(c.channelID, c.definition.Originator(), c.definition.Receivers(), c.definition.CreatedAtNS()+1, c.definition.KeyEpoch(), LifecycleActive)
	_, err = definitions.Create(conflict)
	requireCode(t, err, ErrConflictingDefinition)
	metadata := &metadataSource{values: []MessageMetadata{{MessageID: uuid7(1), TimestampNS: 10_000_000_001}, {MessageID: uuid7(2), TimestampNS: 10_000_000_002}}}
	originator, err := OpenDurableOriginator(backend, c.channelID, c.originatorID, c.signingSecret, c.masterKey, metadata)
	if err != nil {
		t.Fatal(err)
	}
	if err = originator.SaveReceiverGrant(c.binaryReceiverID, []byte{1}); err != nil {
		t.Fatal(err)
	}
	if err = originator.SaveReceiverGrant(c.textReceiverID, []byte{2}); err != nil {
		t.Fatal(err)
	}
	first, err := originator.Publish([]byte("message zero"), "text/plain")
	if err != nil {
		t.Fatal(err)
	}
	second, err := originator.Publish([]byte("message one"), "application/octet-stream")
	if err != nil || first.Sequence != 0 || second.Sequence != 1 {
		t.Fatal("publish mismatch")
	}
	binary, err := OpenDurableReceiver(backend, c.channelID, c.binaryReceiverID, provider(c, c.binaryReceiverID))
	if err != nil {
		t.Fatal(err)
	}
	messages, err := binary.Receive(1)
	if err != nil || len(messages) != 1 || !bytes.Equal(messages[0].Payload, []byte("message zero")) {
		t.Fatal("receive mismatch")
	}
	if cursor, err := binary.Acknowledge(messages[0].MessageID); err != nil || cursor != 1 {
		t.Fatal(err)
	}
	messages, err = binary.Receive(10)
	if err != nil || len(messages) != 1 || messages[0].Sequence != 1 {
		t.Fatal("second receive mismatch")
	}
	if cursor, err := binary.Acknowledge(messages[0].MessageID); err != nil || cursor != 2 {
		t.Fatal(err)
	}
	if cursor, err := binary.Acknowledge(messages[0].MessageID); err != nil || cursor != 2 {
		t.Fatal(err)
	}
	messages, err = binary.Receive(10)
	if err != nil || len(messages) != 0 {
		t.Fatal("expected empty continuation")
	}
	textReceiver, err := OpenDurableReceiver(backend, c.channelID, c.textReceiverID, provider(c, c.textReceiverID))
	if err != nil {
		t.Fatal(err)
	}
	messages, err = textReceiver.Receive(10)
	if err != nil || len(messages) != 2 {
		t.Fatal("independent cursor mismatch")
	}
	if cursor, err := textReceiver.Acknowledge(messages[0].MessageID); err != nil || cursor != 1 {
		t.Fatal(err)
	}
	store, _ := NewChannelStore(backend, c.channelID)
	if cursor, _ := store.ReceiverCursor(c.binaryReceiverID); cursor != 2 {
		t.Fatal("binary cursor changed")
	}
	if cursor, _ := store.ReceiverCursor(c.textReceiverID); cursor != 1 {
		t.Fatal("text cursor changed")
	}
	fresh, _ := OpenDurableReceiver(backend, c.channelID, c.binaryReceiverID, provider(c, c.binaryReceiverID))
	_, err = fresh.Acknowledge(first.MessageID)
	requireCode(t, err, ErrUnknownMessageID)
	_, err = OpenDurableOriginator(backend, c.channelID, []byte("intruder"), c.signingSecret, c.masterKey, metadata)
	requireCode(t, err, ErrUnauthorizedOriginator)
	_, err = OpenDurableReceiver(backend, c.channelID, []byte("intruder"), provider(c, c.binaryReceiverID))
	requireCode(t, err, ErrUnauthorizedReceiver)
	_, err = OpenDurableReceiver(backend, c.channelID, c.binaryReceiverID, receiverProvider{publicKey: make([]byte, 32), key: c.masterKey})
	requireCode(t, err, ErrPublicKeyMismatch)
	destroyed, err := definitions.Destroy(c.channelID)
	if err != nil || destroyed.Lifecycle() != LifecycleDestroyed {
		t.Fatal(err)
	}
	destroyedAgain, err := definitions.Destroy(c.channelID)
	if err != nil || !destroyedAgain.Equal(destroyed) {
		t.Fatal(err)
	}
	page, err := store.ReadMessages(0, 10)
	if err != nil || len(page.Messages) != 2 {
		t.Fatal("destroy removed history")
	}
	_, err = originator.PublishWithMetadata(MessageMetadata{MessageID: uuid7(9), TimestampNS: 9}, []byte("denied"), "text/plain")
	requireCode(t, err, ErrChannelDestroyed)
}

func backendWithMessage(t *testing.T, c testContext) *MemoryChannelStorage {
	t.Helper()
	backend := NewMemoryChannelStorage()
	store, _ := NewChannelStore(backend, c.channelID)
	if _, err := store.Initialize(); err != nil {
		t.Fatal(err)
	}
	if _, err := store.Append(request(c, 30, 30), []byte("record"), c.masterKey, c.signingSecret); err != nil {
		t.Fatal(err)
	}
	return backend
}

func TestMissingGrantsCorruptEnvelopesAndOpaqueBoundariesFailClosed(t *testing.T) {
	c := loadTestContext(t)
	backend := NewMemoryChannelStorage()
	if _, err := NewChannelDefinitionStore(backend).Create(c.definition); err != nil {
		t.Fatal(err)
	}
	originator, err := OpenDurableOriginator(backend, c.channelID, c.originatorID, c.signingSecret, c.masterKey, &metadataSource{values: []MessageMetadata{{MessageID: uuid7(9), TimestampNS: 9}}})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = originator.Publish([]byte("no grant"), "text/plain"); err != nil {
		t.Fatal(err)
	}
	receiver, err := OpenDurableReceiver(backend, c.channelID, c.binaryReceiverID, provider(c, c.binaryReceiverID))
	if err != nil {
		t.Fatal(err)
	}
	_, err = receiver.Receive(1)
	requireCode(t, err, ErrMissingKeyGrant)
	if err = originator.SaveReceiverGrant([]byte("intruder"), []byte{1}); !ErrorIs(err, ErrUnauthorizedReceiver) {
		t.Fatalf("unexpected grant error: %v", err)
	}

	failing := provider(c, c.binaryReceiverID)
	failing.fail = true
	failingReceiver, _ := OpenDurableReceiver(backend, c.channelID, c.binaryReceiverID, failing)
	if err = originator.SaveReceiverGrant(c.binaryReceiverID, []byte{1}); err != nil {
		t.Fatal(err)
	}
	_, err = failingReceiver.Receive(1)
	requireCode(t, err, ErrCrypto)

	keyMismatch := backendWithMessage(t, c)
	key0, _ := MessageRecordKey(c.channelID, 0)
	record, _ := keyMismatch.Get(ChannelStorageNamespace, key0)
	key1, _ := MessageRecordKey(c.channelID, 1)
	record.Key = key1
	keyMismatch.Corrupt(*record)
	store, _ := NewChannelStore(keyMismatch, c.channelID)
	_, err = store.ReadMessages(0, 10)
	requireCode(t, err, ErrCorruptRecord)

	typeMismatch := backendWithMessage(t, c)
	record, _ = typeMismatch.Get(ChannelStorageNamespace, key0)
	record.ContentType = "application/octet-stream"
	typeMismatch.Corrupt(*record)
	store, _ = NewChannelStore(typeMismatch, c.channelID)
	_, err = store.ReadMessages(0, 10)
	requireCode(t, err, ErrCorruptRecord)

	store, _ = NewChannelStore(backend, c.channelID)
	grant := OpaqueKeyGrant{ChannelID: c.channelID, ReceiverID: c.binaryReceiverID, Body: []byte{1}, KeyEpoch: c.definition.KeyEpoch()}
	if err = store.SaveKeyGrant(grant); err != nil {
		t.Fatal(err)
	}
	if err = store.SaveKeyGrant(grant); err != nil {
		t.Fatal(err)
	}
	loaded, err := store.KeyGrant(grant.KeyEpoch, grant.ReceiverID)
	if err != nil || !bytes.Equal(loaded, grant.Body) {
		t.Fatal("opaque grant changed")
	}
	grant.Body = []byte{2}
	requireCode(t, store.SaveKeyGrant(grant), ErrConflictingRecord)
}

func TestBackendConditionsCopiesAndEndpointInputFailures(t *testing.T) {
	c := loadTestContext(t)
	backend := NewMemoryChannelStorage()
	_, err := backend.Put(StoragePut{Namespace: "n", Key: "k"})
	if err == nil {
		t.Fatal("missing condition accepted")
	}
	_, err = backend.Put(StoragePut{Namespace: "n", Key: "k", IfAbsent: true, IfRevision: "r1"})
	if err == nil {
		t.Fatal("two conditions accepted")
	}
	body := []byte{1}
	record, err := backend.Put(StoragePut{Namespace: "n", Key: "b", ContentType: "c", Body: body, IfAbsent: true})
	if err != nil {
		t.Fatal(err)
	}
	body[0] = 9
	record.Body[0] = 8
	loaded, _ := backend.Get("n", "b")
	if loaded.Body[0] != 1 {
		t.Fatal("backend aliased mutable bytes")
	}
	_, err = backend.Put(StoragePut{Namespace: "n", Key: "b", IfAbsent: true})
	if !IsStorageConflict(err) {
		t.Fatal("create conflict missing")
	}
	_, err = backend.Put(StoragePut{Namespace: "n", Key: "b", IfRevision: "wrong"})
	if !IsStorageConflict(err) {
		t.Fatal("revision conflict missing")
	}
	updated, err := backend.Put(StoragePut{Namespace: "n", Key: "b", ContentType: "c", Body: []byte{2}, IfRevision: loaded.Revision})
	if err != nil || updated.Revision == loaded.Revision {
		t.Fatal("CAS update failed")
	}
	if _, err = backend.List("n", StorageListOptions{Recursive: true, PageSize: 0}); err == nil {
		t.Fatal("invalid page accepted")
	}
	page, err := backend.List("n", StorageListOptions{Prefix: "", Recursive: true, PageSize: 1})
	if err != nil || len(page.Records) != 1 {
		t.Fatal("backend list failed")
	}

	definitionBackend := NewMemoryChannelStorage()
	if _, err = NewChannelDefinitionStore(definitionBackend).Create(c.definition); err != nil {
		t.Fatal(err)
	}
	badSecret := clone(c.signingSecret)
	badSecret[63] ^= 1
	_, err = OpenDurableOriginator(definitionBackend, c.channelID, c.originatorID, badSecret, c.masterKey, &metadataSource{})
	requireCode(t, err, ErrPublicKeyMismatch)
	_, err = OpenDurableOriginator(definitionBackend, c.channelID, c.originatorID, c.signingSecret, []byte{1}, &metadataSource{})
	requireCode(t, err, ErrCrypto)
	originator, err := OpenDurableOriginator(definitionBackend, c.channelID, c.originatorID, c.signingSecret, c.masterKey, &metadataSource{})
	if err != nil {
		t.Fatal(err)
	}
	_, err = originator.Publish([]byte("x"), "text/plain")
	requireCode(t, err, ErrMetadata)
	missingID := uuid7(99)
	_, err = NewChannelDefinitionStore(definitionBackend).Load(missingID)
	if err != nil {
		t.Fatal(err)
	}
	_, err = OpenDurableReceiver(definitionBackend, missingID, c.binaryReceiverID, provider(c, c.binaryReceiverID))
	requireCode(t, err, ErrDefinitionNotFound)
}

func TestPublicValuesAreDefensiveAndEndpointIdentityIsBound(t *testing.T) {
	c := loadTestContext(t)
	receiver, _ := c.definition.Receiver(c.binaryReceiverID)
	receiverID := receiver.AgentID()
	receiverID[0] ^= 0xff
	if bytes.Equal(receiverID, receiver.AgentID()) {
		t.Fatal("receiver ID accessor aliases state")
	}
	pending, err := ChannelStateDeserialize(decodeB64(t, c.fixture.StateCases[1].D18S), c.channelID)
	if err != nil || pending.PendingHeader == nil {
		t.Fatal(err)
	}
	header := pending.PendingHeader
	if len(header.MessageID()) != 16 || header.TimestampNS() != header.timestampNS || !bytes.Equal(header.OriginatorID(), header.originatorID) || !bytes.Equal(header.ChannelID(), header.channelID[:]) || header.KeyEpoch() != header.keyEpoch || header.ContentType() != header.contentType {
		t.Fatal("header accessors changed values")
	}
	messageID := header.MessageID()
	messageID[0] ^= 0xff
	if bytes.Equal(messageID, header.MessageID()) {
		t.Fatal("header accessor aliases state")
	}

	backend := NewMemoryChannelStorage()
	if _, err = NewChannelDefinitionStore(backend).Create(c.definition); err != nil {
		t.Fatal(err)
	}
	originator, err := OpenDurableOriginator(backend, c.channelID, c.originatorID, c.signingSecret, c.masterKey, &metadataSource{})
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(originator.ID(), c.originatorID) || !bytes.Equal(originator.ChannelID(), c.channelID) || !bytes.Equal(originator.PublicKey(), c.definition.Originator().PublicKey()) {
		t.Fatal("originator identity mismatch")
	}
	receiverEndpoint, err := OpenDurableReceiver(backend, c.channelID, c.binaryReceiverID, provider(c, c.binaryReceiverID))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(receiverEndpoint.ID(), c.binaryReceiverID) || !bytes.Equal(receiverEndpoint.ChannelID(), c.channelID) || !bytes.Equal(receiverEndpoint.PublicKey(), receiver.PublicKey()) {
		t.Fatal("receiver identity mismatch")
	}
}
