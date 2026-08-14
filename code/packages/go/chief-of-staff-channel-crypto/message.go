// Package channelcrypto implements the portable D18F encrypted-message profile.
package channelcrypto

import (
	"bytes"
	"crypto/subtle"
	"encoding/base64"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"strconv"
	"strings"
	"unicode/utf16"
	"unicode/utf8"

	chacha20poly1305 "github.com/adhithyan15/coding-adventures/code/packages/go/chacha20-poly1305"
	sha256 "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"
	ed25519 "github.com/example/coding-adventures/code/packages/go/ed25519"
)

const (
	MaxMessageJSONBytes = 90 * 1024 * 1024
	maxIdentityBytes    = 4 * 1024
	maxContentTypeBytes = 1024
	maxCiphertextBytes  = 64 * 1024 * 1024
	maxUUIDTimestamp    = uint64(1<<48) - 1
	wireVersion         = byte(1)
)

var (
	messageMagic   = []byte("D18M")
	messageContext = []byte("chief-channel-message-v1")
)

// ErrorCode is the stable, portable failure classification defined by D18F.
type ErrorCode string

const (
	ErrInvalidMagic          ErrorCode = "invalid_magic"
	ErrUnsupportedVersion    ErrorCode = "unsupported_version"
	ErrTruncatedRecord       ErrorCode = "truncated_record"
	ErrTrailingBytes         ErrorCode = "trailing_bytes"
	ErrLengthLimitExceeded   ErrorCode = "length_limit_exceeded"
	ErrInvalidUTF8           ErrorCode = "invalid_utf8"
	ErrInvalidField          ErrorCode = "invalid_field"
	ErrInvalidJSON           ErrorCode = "invalid_json"
	ErrMissingEpochKey       ErrorCode = "missing_epoch_key"
	ErrInvalidSignature      ErrorCode = "invalid_signature"
	ErrAuthenticationFailed  ErrorCode = "authentication_failed"
	ErrPlaintextHashMismatch ErrorCode = "plaintext_hash_mismatch"
)

// ProfileError is one fail-closed D18F operation error.
type ProfileError struct{ Code ErrorCode }

func (e *ProfileError) Error() string { return string(e.Code) }

func fail(code ErrorCode) error { return &ProfileError{Code: code} }

// ErrorIs reports whether err has the requested portable code.
func ErrorIs(err error, code ErrorCode) bool {
	var profileErr *ProfileError
	return errors.As(err, &profileErr) && profileErr.Code == code
}

// MessageFields holds the immutable high-level fields supplied before creation.
type MessageFields struct {
	messageID    [16]byte
	timestampNS  uint64
	originatorID []byte
	channelID    [16]byte
	sequence     uint64
	keyEpoch     uint64
	contentType  string
}

// NewMessageFields copies all caller-owned bytes into a structurally valid value.
func NewMessageFields(messageID []byte, timestampNS uint64, originatorID, channelID []byte, sequence, keyEpoch uint64, contentType string) (MessageFields, error) {
	if len(messageID) != 16 || len(channelID) != 16 || !utf8.ValidString(contentType) {
		return MessageFields{}, fail(ErrInvalidField)
	}
	if len(originatorID) > maxIdentityBytes || len([]byte(contentType)) > maxContentTypeBytes {
		return MessageFields{}, fail(ErrLengthLimitExceeded)
	}
	var messageIDArray, channelIDArray [16]byte
	copy(messageIDArray[:], messageID)
	copy(channelIDArray[:], channelID)
	return MessageFields{
		messageID: messageIDArray, timestampNS: timestampNS,
		originatorID: clone(originatorID), channelID: channelIDArray,
		sequence: sequence, keyEpoch: keyEpoch, contentType: contentType,
	}, nil
}

func (f MessageFields) MessageID() []byte    { return clone(f.messageID[:]) }
func (f MessageFields) TimestampNS() uint64  { return f.timestampNS }
func (f MessageFields) OriginatorID() []byte { return clone(f.originatorID) }
func (f MessageFields) ChannelID() []byte    { return clone(f.channelID[:]) }
func (f MessageFields) Sequence() uint64     { return f.sequence }
func (f MessageFields) KeyEpoch() uint64     { return f.keyEpoch }
func (f MessageFields) ContentType() string  { return f.contentType }

// SourcedMessageFields contains creation fields whose identifier and clock are injected.
type SourcedMessageFields struct {
	originatorID []byte
	channelID    [16]byte
	sequence     uint64
	keyEpoch     uint64
	contentType  string
}

func NewSourcedMessageFields(originatorID, channelID []byte, sequence, keyEpoch uint64, contentType string) (SourcedMessageFields, error) {
	fields, err := NewMessageFields(make([]byte, 16), 0, originatorID, channelID, sequence, keyEpoch, contentType)
	if err != nil {
		return SourcedMessageFields{}, err
	}
	return SourcedMessageFields{clone(fields.originatorID), fields.channelID, sequence, keyEpoch, contentType}, nil
}

// D18Message is a structurally immutable encrypted-message value. Every byte
// accessor returns a defensive copy.
type D18Message struct {
	fields              MessageFields
	plaintextHash       [32]byte
	ciphertext          []byte
	authenticationTag   [16]byte
	originatorSignature [64]byte
}

func newD18Message(fields MessageFields, plaintextHash, ciphertext, authenticationTag, originatorSignature []byte) (D18Message, error) {
	if len(plaintextHash) != 32 || len(authenticationTag) != 16 || len(originatorSignature) != 64 {
		return D18Message{}, fail(ErrInvalidField)
	}
	if len(ciphertext) > maxCiphertextBytes {
		return D18Message{}, fail(ErrLengthLimitExceeded)
	}
	var result D18Message
	result.fields = copyFields(fields)
	copy(result.plaintextHash[:], plaintextHash)
	result.ciphertext = clone(ciphertext)
	copy(result.authenticationTag[:], authenticationTag)
	copy(result.originatorSignature[:], originatorSignature)
	return result, nil
}

func (m D18Message) Fields() MessageFields       { return copyFields(m.fields) }
func (m D18Message) MessageID() []byte           { return m.fields.MessageID() }
func (m D18Message) TimestampNS() uint64         { return m.fields.timestampNS }
func (m D18Message) OriginatorID() []byte        { return m.fields.OriginatorID() }
func (m D18Message) ChannelID() []byte           { return m.fields.ChannelID() }
func (m D18Message) Sequence() uint64            { return m.fields.sequence }
func (m D18Message) KeyEpoch() uint64            { return m.fields.keyEpoch }
func (m D18Message) ContentType() string         { return m.fields.contentType }
func (m D18Message) PlaintextHash() []byte       { return clone(m.plaintextHash[:]) }
func (m D18Message) Ciphertext() []byte          { return clone(m.ciphertext) }
func (m D18Message) AuthenticationTag() []byte   { return clone(m.authenticationTag[:]) }
func (m D18Message) OriginatorSignature() []byte { return clone(m.originatorSignature[:]) }

func copyFields(fields MessageFields) MessageFields {
	fields.originatorID = clone(fields.originatorID)
	return fields
}

// UUIDv7Source supplies immutable UUID-v7 bytes to MessageCreateWithSources.
type UUIDv7Source interface{ NextUUIDv7() ([]byte, error) }

// MonotonicNanosecondSource supplies the monotonic timestamp used for creation.
type MonotonicNanosecondSource interface{ NowNanoseconds() uint64 }

// MonotonicUUIDv7Generator creates RFC 9562 UUID-v7 identifiers and increments
// the 74 random bits when the supplied millisecond does not advance.
type MonotonicUUIDv7Generator struct {
	hasLast       bool
	lastTimestamp uint64
	lastRandom    [10]byte // the leading six bits are always zero
}

func (g *MonotonicUUIDv7Generator) Next(timestampMS uint64, entropy []byte) ([]byte, error) {
	if timestampMS > maxUUIDTimestamp || len(entropy) != 10 {
		return nil, fail(ErrInvalidField)
	}
	random := [10]byte{}
	copy(random[:], entropy)
	random[0] &= 0x03
	effectiveTimestamp := timestampMS
	if g.hasLast && timestampMS <= g.lastTimestamp {
		effectiveTimestamp = g.lastTimestamp
		random = g.lastRandom
		if !incrementRandom(&random) {
			if effectiveTimestamp == maxUUIDTimestamp {
				return nil, fail(ErrInvalidField)
			}
			effectiveTimestamp++
			random = [10]byte{}
		}
	}
	g.hasLast, g.lastTimestamp, g.lastRandom = true, effectiveTimestamp, random
	result := make([]byte, 16)
	result[0] = byte(effectiveTimestamp >> 40)
	result[1] = byte(effectiveTimestamp >> 32)
	result[2] = byte(effectiveTimestamp >> 24)
	result[3] = byte(effectiveTimestamp >> 16)
	result[4] = byte(effectiveTimestamp >> 8)
	result[5] = byte(effectiveTimestamp)
	result[6] = 0x70 | (random[0] << 2) | (random[1] >> 6)
	result[7] = (random[1] << 2) | (random[2] >> 6)
	result[8] = 0x80 | (random[2] & 0x3f)
	copy(result[9:], random[3:])
	return result, nil
}

func incrementRandom(value *[10]byte) bool {
	for i := len(value) - 1; i >= 0; i-- {
		if value[i] != 0xff {
			value[i]++
			return i != 0 || value[0] <= 0x03
		}
		value[i] = 0
	}
	return false
}

// ValidateMessageFields applies the high-level delivery rules omitted by the
// structural binary decoder.
func ValidateMessageFields(fields MessageFields) error {
	if err := validateUUIDv7(fields.messageID[:]); err != nil {
		return err
	}
	if err := validateUUIDv7(fields.channelID[:]); err != nil {
		return err
	}
	if len(fields.originatorID) == 0 {
		return fail(ErrInvalidField)
	}
	if len(fields.originatorID) > maxIdentityBytes || len([]byte(fields.contentType)) > maxContentTypeBytes {
		return fail(ErrLengthLimitExceeded)
	}
	return validateMIME(fields.contentType)
}

// MessageCreate validates, hashes, signs, and encrypts one D18F message.
func MessageCreate(fields MessageFields, plaintext, signingSecretKey, channelMasterKey []byte) (D18Message, error) {
	if err := ValidateMessageFields(fields); err != nil {
		return D18Message{}, err
	}
	if len(plaintext) > maxCiphertextBytes {
		return D18Message{}, fail(ErrLengthLimitExceeded)
	}
	if len(signingSecretKey) != 64 || len(channelMasterKey) != 32 {
		return D18Message{}, fail(ErrInvalidField)
	}
	hash := sha256.Sum256(clone(plaintext))
	header := authenticatedHeader(fields, hash[:])
	nonce := messageNonce(fields.channelID, fields.sequence)
	ciphertext, tag, err := chacha20poly1305.XChaCha20Poly1305AEADEncrypt(clone(plaintext), clone(channelMasterKey), nonce, header)
	if err != nil {
		return D18Message{}, fail(ErrInvalidField)
	}
	var secret [64]byte
	copy(secret[:], signingSecretKey)
	signature := ed25519.Sign(header, secret)
	return newD18Message(fields, hash[:], ciphertext, tag, signature[:])
}

// MessageCreateWithSources creates a message with injected identifiers and time.
func MessageCreateWithSources(fields SourcedMessageFields, plaintext, signingSecretKey, channelMasterKey []byte, uuidSource UUIDv7Source, clock MonotonicNanosecondSource) (D18Message, error) {
	messageID, err := uuidSource.NextUUIDv7()
	if err != nil {
		return D18Message{}, err
	}
	complete, err := NewMessageFields(messageID, clock.NowNanoseconds(), fields.originatorID, fields.channelID[:], fields.sequence, fields.keyEpoch, fields.contentType)
	if err != nil {
		return D18Message{}, err
	}
	return MessageCreate(complete, plaintext, signingSecretKey, channelMasterKey)
}

// MessageVerify verifies and decrypts using an explicitly selected epoch key.
func MessageVerify(message D18Message, originatorPublicKey, channelMasterKey []byte) ([]byte, error) {
	if err := ValidateMessageFields(message.fields); err != nil {
		return nil, err
	}
	if len(channelMasterKey) != 32 {
		return nil, fail(ErrInvalidField)
	}
	return verifyCryptography(message, originatorPublicKey, channelMasterKey)
}

// MessageVerifyWithKeyResolver resolves the named epoch before cryptographic checks.
func MessageVerifyWithKeyResolver(message D18Message, originatorPublicKey []byte, keyForEpoch func(uint64) []byte) ([]byte, error) {
	if err := ValidateMessageFields(message.fields); err != nil {
		return nil, err
	}
	key := keyForEpoch(message.fields.keyEpoch)
	if key == nil {
		return nil, fail(ErrMissingEpochKey)
	}
	if len(key) != 32 {
		return nil, fail(ErrInvalidField)
	}
	return verifyCryptography(message, originatorPublicKey, key)
}

func verifyCryptography(message D18Message, originatorPublicKey, channelMasterKey []byte) ([]byte, error) {
	if len(originatorPublicKey) != 32 {
		return nil, fail(ErrInvalidField)
	}
	header := MessageAuthenticatedHeader(message)
	var public [32]byte
	copy(public[:], originatorPublicKey)
	if !ed25519.Verify(header, message.originatorSignature, public) {
		return nil, fail(ErrInvalidSignature)
	}
	plaintext, err := chacha20poly1305.XChaCha20Poly1305AEADDecrypt(message.ciphertext, clone(channelMasterKey), messageNonce(message.fields.channelID, message.fields.sequence), header, message.authenticationTag[:])
	if err != nil {
		return nil, fail(ErrAuthenticationFailed)
	}
	hash := sha256.Sum256(plaintext)
	if subtle.ConstantTimeCompare(hash[:], message.plaintextHash[:]) != 1 {
		return nil, fail(ErrPlaintextHashMismatch)
	}
	return plaintext, nil
}

// MessageAuthenticatedHeader returns the exact length-framed D18F AAD.
func MessageAuthenticatedHeader(message D18Message) []byte {
	return authenticatedHeader(message.fields, message.plaintextHash[:])
}

func authenticatedHeader(fields MessageFields, plaintextHash []byte) []byte {
	parts := [][]byte{
		messageContext, fields.messageID[:], u64be(fields.timestampNS), fields.originatorID,
		fields.channelID[:], u64be(fields.sequence), u64be(fields.keyEpoch), []byte(fields.contentType), plaintextHash,
	}
	var result []byte
	for _, part := range parts {
		result = append(result, u64be(uint64(len(part)))...)
		result = append(result, part...)
	}
	return result
}

func messageNonce(channelID [16]byte, sequence uint64) []byte {
	return append(clone(channelID[:]), u64be(sequence)...)
}

// MessageSerialize encodes one unchanged D18M version 1 record.
func MessageSerialize(message D18Message) ([]byte, error) {
	if len(message.fields.originatorID) > maxIdentityBytes || len([]byte(message.fields.contentType)) > maxContentTypeBytes || len(message.ciphertext) > maxCiphertextBytes {
		return nil, fail(ErrLengthLimitExceeded)
	}
	var result []byte
	result = append(result, messageMagic...)
	result = append(result, wireVersion)
	result = append(result, message.fields.messageID[:]...)
	result = append(result, u64be(message.fields.timestampNS)...)
	result = append(result, u32be(uint32(len(message.fields.originatorID)))...)
	result = append(result, message.fields.originatorID...)
	result = append(result, message.fields.channelID[:]...)
	result = append(result, u64be(message.fields.sequence)...)
	result = append(result, u64be(message.fields.keyEpoch)...)
	result = append(result, u32be(uint32(len([]byte(message.fields.contentType))))...)
	result = append(result, []byte(message.fields.contentType)...)
	result = append(result, message.plaintextHash[:]...)
	result = append(result, u64be(uint64(len(message.ciphertext)))...)
	result = append(result, message.ciphertext...)
	result = append(result, message.authenticationTag[:]...)
	result = append(result, message.originatorSignature[:]...)
	return result, nil
}

type decoder struct {
	data []byte
	pos  int
}

func (d *decoder) take(length int) ([]byte, error) {
	if length < 0 || length > len(d.data)-d.pos {
		return nil, fail(ErrTruncatedRecord)
	}
	value := clone(d.data[d.pos : d.pos+length])
	d.pos += length
	return value, nil
}

func (d *decoder) u64() (uint64, error) {
	value, err := d.take(8)
	if err != nil {
		return 0, err
	}
	return binary.BigEndian.Uint64(value), nil
}

func (d *decoder) bounded32(maximum uint64) ([]byte, error) {
	value, err := d.take(4)
	if err != nil {
		return nil, err
	}
	return d.bounded(uint64(binary.BigEndian.Uint32(value)), maximum)
}

func (d *decoder) bounded64(maximum uint64) ([]byte, error) {
	length, err := d.u64()
	if err != nil {
		return nil, err
	}
	return d.bounded(length, maximum)
}

func (d *decoder) bounded(length, maximum uint64) ([]byte, error) {
	if length > maximum {
		return nil, fail(ErrLengthLimitExceeded)
	}
	if length > uint64(len(d.data)-d.pos) {
		return nil, fail(ErrTruncatedRecord)
	}
	return d.take(int(length))
}

// MessageDeserialize structurally decodes one D18M version 1 record.
func MessageDeserialize(data []byte) (D18Message, error) {
	d := &decoder{data: clone(data)}
	magic, err := d.take(4)
	if err != nil {
		return D18Message{}, err
	}
	if !bytes.Equal(magic, messageMagic) {
		return D18Message{}, fail(ErrInvalidMagic)
	}
	version, err := d.take(1)
	if err != nil {
		return D18Message{}, err
	}
	if version[0] != wireVersion {
		return D18Message{}, fail(ErrUnsupportedVersion)
	}
	messageID, err := d.take(16)
	if err != nil {
		return D18Message{}, err
	}
	timestampNS, err := d.u64()
	if err != nil {
		return D18Message{}, err
	}
	originatorID, err := d.bounded32(maxIdentityBytes)
	if err != nil {
		return D18Message{}, err
	}
	channelID, err := d.take(16)
	if err != nil {
		return D18Message{}, err
	}
	sequence, err := d.u64()
	if err != nil {
		return D18Message{}, err
	}
	keyEpoch, err := d.u64()
	if err != nil {
		return D18Message{}, err
	}
	contentBytes, err := d.bounded32(maxContentTypeBytes)
	if err != nil {
		return D18Message{}, err
	}
	if !utf8.Valid(contentBytes) {
		return D18Message{}, fail(ErrInvalidUTF8)
	}
	plaintextHash, err := d.take(32)
	if err != nil {
		return D18Message{}, err
	}
	ciphertext, err := d.bounded64(maxCiphertextBytes)
	if err != nil {
		return D18Message{}, err
	}
	tag, err := d.take(16)
	if err != nil {
		return D18Message{}, err
	}
	signature, err := d.take(64)
	if err != nil {
		return D18Message{}, err
	}
	if d.pos != len(d.data) {
		return D18Message{}, fail(ErrTrailingBytes)
	}
	fields, err := NewMessageFields(messageID, timestampNS, originatorID, channelID, sequence, keyEpoch, string(contentBytes))
	if err != nil {
		return D18Message{}, err
	}
	return newD18Message(fields, plaintextHash, ciphertext, tag, signature)
}

var jsonFields = []string{
	"record_type", "wire_version", "message_id", "timestamp_ns", "originator_id_b64",
	"channel_id", "sequence", "key_epoch", "content_type", "plaintext_hash_hex",
	"ciphertext_b64", "authentication_tag_b64", "originator_signature_b64",
}

// MessageToJSON encodes one message as canonical, lossless D18F JSON bytes.
func MessageToJSON(message D18Message) ([]byte, error) {
	values := []string{
		"D18M", "1", uuidString(message.fields.messageID), strconv.Quote(strconv.FormatUint(message.fields.timestampNS, 10)),
		strconv.Quote(base64.StdEncoding.EncodeToString(message.fields.originatorID)), uuidString(message.fields.channelID),
		strconv.Quote(strconv.FormatUint(message.fields.sequence, 10)), strconv.Quote(strconv.FormatUint(message.fields.keyEpoch, 10)),
		jsonString(message.fields.contentType), strconv.Quote(hex.EncodeToString(message.plaintextHash[:])),
		strconv.Quote(base64.StdEncoding.EncodeToString(message.ciphertext)), strconv.Quote(base64.StdEncoding.EncodeToString(message.authenticationTag[:])),
		strconv.Quote(base64.StdEncoding.EncodeToString(message.originatorSignature[:])),
	}
	values[0] = strconv.Quote(values[0])
	values[2] = strconv.Quote(values[2])
	values[5] = strconv.Quote(values[5])
	var result strings.Builder
	result.WriteByte('{')
	for i, name := range jsonFields {
		if i != 0 {
			result.WriteByte(',')
		}
		result.WriteString(strconv.Quote(name))
		result.WriteByte(':')
		result.WriteString(values[i])
	}
	result.WriteByte('}')
	encoded := []byte(result.String())
	if len(encoded) > MaxMessageJSONBytes {
		return nil, fail(ErrLengthLimitExceeded)
	}
	return encoded, nil
}

func jsonString(value string) string {
	var result strings.Builder
	result.WriteByte('"')
	for _, r := range value {
		switch r {
		case '"':
			result.WriteString(`\"`)
		case '\\':
			result.WriteString(`\\`)
		case '\b':
			result.WriteString(`\b`)
		case '\f':
			result.WriteString(`\f`)
		case '\n':
			result.WriteString(`\n`)
		case '\r':
			result.WriteString(`\r`)
		case '\t':
			result.WriteString(`\t`)
		default:
			if r < 0x20 {
				fmt.Fprintf(&result, `\u%04x`, r)
			} else {
				result.WriteRune(r)
			}
		}
	}
	result.WriteByte('"')
	return result.String()
}

// MessageFromJSON structurally decodes strict D18F JSON.
func MessageFromJSON(data []byte) (D18Message, error) {
	if len(data) > MaxMessageJSONBytes {
		return D18Message{}, fail(ErrLengthLimitExceeded)
	}
	if !utf8.Valid(data) {
		return D18Message{}, fail(ErrInvalidJSON)
	}
	values, err := decodeJSONObject(data)
	if err != nil {
		return D18Message{}, err
	}
	if len(values) != len(jsonFields) {
		return D18Message{}, fail(ErrInvalidJSON)
	}
	for _, name := range jsonFields {
		if _, ok := values[name]; !ok {
			return D18Message{}, fail(ErrInvalidJSON)
		}
	}
	recordType, err := decodeJSONString(values["record_type"])
	if err != nil {
		return D18Message{}, fail(ErrInvalidJSON)
	}
	if recordType != "D18M" {
		return D18Message{}, fail(ErrInvalidMagic)
	}
	var version uint64
	if err := json.Unmarshal(values["wire_version"], &version); err != nil {
		return D18Message{}, fail(ErrInvalidJSON)
	}
	if string(values["wire_version"]) != "1" || version != 1 {
		return D18Message{}, fail(ErrUnsupportedVersion)
	}
	messageIDText, err := stringJSONField(values, "message_id")
	if err != nil {
		return D18Message{}, err
	}
	messageID, err := decodeUUIDv7(messageIDText)
	if err != nil {
		return D18Message{}, err
	}
	timestamp, err := decimalJSONField(values, "timestamp_ns")
	if err != nil {
		return D18Message{}, err
	}
	originator, err := base64JSONField(values, "originator_id_b64", maxIdentityBytes, -1)
	if err != nil {
		return D18Message{}, err
	}
	channelText, err := stringJSONField(values, "channel_id")
	if err != nil {
		return D18Message{}, err
	}
	channelID, err := decodeUUIDv7(channelText)
	if err != nil {
		return D18Message{}, err
	}
	sequence, err := decimalJSONField(values, "sequence")
	if err != nil {
		return D18Message{}, err
	}
	epoch, err := decimalJSONField(values, "key_epoch")
	if err != nil {
		return D18Message{}, err
	}
	contentType, err := stringJSONField(values, "content_type")
	if err != nil {
		return D18Message{}, err
	}
	if len([]byte(contentType)) > maxContentTypeBytes {
		return D18Message{}, fail(ErrLengthLimitExceeded)
	}
	hashText, err := stringJSONField(values, "plaintext_hash_hex")
	if err != nil {
		return D18Message{}, err
	}
	hash, err := decodeHexExact(hashText, 32)
	if err != nil {
		return D18Message{}, err
	}
	ciphertext, err := base64JSONField(values, "ciphertext_b64", maxCiphertextBytes, -1)
	if err != nil {
		return D18Message{}, err
	}
	tag, err := base64JSONField(values, "authentication_tag_b64", 16, 16)
	if err != nil {
		return D18Message{}, err
	}
	signature, err := base64JSONField(values, "originator_signature_b64", 64, 64)
	if err != nil {
		return D18Message{}, err
	}
	fields, err := NewMessageFields(messageID, timestamp, originator, channelID, sequence, epoch, contentType)
	if err != nil {
		return D18Message{}, err
	}
	return newD18Message(fields, hash, ciphertext, tag, signature)
}

func decodeJSONObject(data []byte) (map[string]json.RawMessage, error) {
	dec := json.NewDecoder(bytes.NewReader(data))
	token, err := dec.Token()
	if err != nil || token != json.Delim('{') {
		return nil, fail(ErrInvalidJSON)
	}
	result := make(map[string]json.RawMessage)
	for dec.More() {
		keyToken, err := dec.Token()
		key, ok := keyToken.(string)
		if err != nil || !ok {
			return nil, fail(ErrInvalidJSON)
		}
		if _, exists := result[key]; exists {
			return nil, fail(ErrInvalidJSON)
		}
		var raw json.RawMessage
		if err := dec.Decode(&raw); err != nil {
			return nil, fail(ErrInvalidJSON)
		}
		result[key] = raw
	}
	if token, err = dec.Token(); err != nil || token != json.Delim('}') {
		return nil, fail(ErrInvalidJSON)
	}
	if token, err = dec.Token(); err != io.EOF || token != nil {
		return nil, fail(ErrInvalidJSON)
	}
	return result, nil
}

func decodeJSONString(raw []byte) (string, error) {
	if !validJSONStringEscapes(raw) {
		return "", fail(ErrInvalidField)
	}
	var value string
	if err := json.Unmarshal(raw, &value); err != nil || !utf8.ValidString(value) {
		return "", fail(ErrInvalidField)
	}
	return value, nil
}

func validJSONStringEscapes(raw []byte) bool {
	if len(raw) < 2 || raw[0] != '"' || raw[len(raw)-1] != '"' {
		return false
	}
	for i := 1; i < len(raw)-1; {
		if raw[i] < 0x20 {
			return false
		}
		if raw[i] != '\\' {
			_, size := utf8.DecodeRune(raw[i : len(raw)-1])
			if size == 0 {
				return false
			}
			i += size
			continue
		}
		i++
		if i >= len(raw)-1 {
			return false
		}
		if strings.ContainsRune(`"\\/bfnrt`, rune(raw[i])) {
			i++
			continue
		}
		if raw[i] != 'u' || i+4 >= len(raw) {
			return false
		}
		code, ok := hex4(raw[i+1 : i+5])
		if !ok {
			return false
		}
		i += 5
		if code >= 0xd800 && code <= 0xdbff {
			if i+5 >= len(raw) || raw[i] != '\\' || raw[i+1] != 'u' {
				return false
			}
			low, ok := hex4(raw[i+2 : i+6])
			if !ok || low < 0xdc00 || low > 0xdfff || !utf8.ValidRune(utf16.DecodeRune(rune(code), rune(low))) {
				return false
			}
			i += 6
		} else if code >= 0xdc00 && code <= 0xdfff {
			return false
		}
	}
	return true
}

func hex4(value []byte) (uint16, bool) {
	if len(value) != 4 {
		return 0, false
	}
	decoded, err := strconv.ParseUint(string(value), 16, 16)
	return uint16(decoded), err == nil
}

func stringJSONField(values map[string]json.RawMessage, name string) (string, error) {
	value, err := decodeJSONString(values[name])
	if err != nil {
		return "", err
	}
	return value, nil
}

func decimalJSONField(values map[string]json.RawMessage, name string) (uint64, error) {
	value, err := stringJSONField(values, name)
	if err != nil {
		return 0, err
	}
	if value == "" || (len(value) > 1 && value[0] == '0') {
		return 0, fail(ErrInvalidField)
	}
	for _, c := range value {
		if c < '0' || c > '9' {
			return 0, fail(ErrInvalidField)
		}
	}
	decoded, err := strconv.ParseUint(value, 10, 64)
	if err != nil {
		return 0, fail(ErrInvalidField)
	}
	return decoded, nil
}

func base64JSONField(values map[string]json.RawMessage, name string, maximum, exact int) ([]byte, error) {
	value, err := stringJSONField(values, name)
	if err != nil {
		return nil, err
	}
	if len(value)%4 != 0 {
		return nil, fail(ErrInvalidField)
	}
	if len(value)/4*3 > maximum+2 {
		return nil, fail(ErrLengthLimitExceeded)
	}
	decoded, err := base64.StdEncoding.Strict().DecodeString(value)
	if err != nil {
		return nil, fail(ErrInvalidField)
	}
	if len(decoded) > maximum {
		return nil, fail(ErrLengthLimitExceeded)
	}
	if exact >= 0 && len(decoded) != exact {
		return nil, fail(ErrInvalidField)
	}
	if base64.StdEncoding.EncodeToString(decoded) != value {
		return nil, fail(ErrInvalidField)
	}
	return decoded, nil
}

func decodeHexExact(value string, length int) ([]byte, error) {
	if len(value) != length*2 || value != strings.ToLower(value) {
		return nil, fail(ErrInvalidField)
	}
	decoded, err := hex.DecodeString(value)
	if err != nil {
		return nil, fail(ErrInvalidField)
	}
	return decoded, nil
}

func validateUUIDv7(value []byte) error {
	if len(value) != 16 || value[6]>>4 != 7 || value[8]&0xc0 != 0x80 {
		return fail(ErrInvalidField)
	}
	return nil
}

func decodeUUIDv7(value string) ([]byte, error) {
	if len(value) != 36 || value != strings.ToLower(value) || value[8] != '-' || value[13] != '-' || value[18] != '-' || value[23] != '-' {
		return nil, fail(ErrInvalidField)
	}
	compact := strings.ReplaceAll(value, "-", "")
	decoded, err := hex.DecodeString(compact)
	if err != nil {
		return nil, fail(ErrInvalidField)
	}
	if err := validateUUIDv7(decoded); err != nil {
		return nil, err
	}
	return decoded, nil
}

func uuidString(value [16]byte) string {
	hexValue := hex.EncodeToString(value[:])
	return hexValue[:8] + "-" + hexValue[8:12] + "-" + hexValue[12:16] + "-" + hexValue[16:20] + "-" + hexValue[20:]
}

func validateMIME(value string) error {
	encoded := []byte(value)
	if len(encoded) == 0 || !utf8.Valid(encoded) {
		return fail(ErrInvalidField)
	}
	for _, b := range encoded {
		if b < 0x20 || b > 0x7e {
			return fail(ErrInvalidField)
		}
	}
	index, ok := consumeToken(encoded, 0)
	if !ok || index >= len(encoded) || encoded[index] != '/' {
		return fail(ErrInvalidField)
	}
	index, ok = consumeToken(encoded, index+1)
	if !ok {
		return fail(ErrInvalidField)
	}
	for index < len(encoded) {
		index = consumeSpaces(encoded, index)
		if index >= len(encoded) || encoded[index] != ';' {
			return fail(ErrInvalidField)
		}
		index = consumeSpaces(encoded, index+1)
		index, ok = consumeToken(encoded, index)
		if !ok {
			return fail(ErrInvalidField)
		}
		index = consumeSpaces(encoded, index)
		if index >= len(encoded) || encoded[index] != '=' {
			return fail(ErrInvalidField)
		}
		index = consumeSpaces(encoded, index+1)
		if index < len(encoded) && encoded[index] == '"' {
			index++
			for {
				if index >= len(encoded) {
					return fail(ErrInvalidField)
				}
				if encoded[index] == '"' {
					index++
					break
				}
				if encoded[index] == '\\' {
					index++
					if index >= len(encoded) {
						return fail(ErrInvalidField)
					}
				}
				index++
			}
		} else {
			index, ok = consumeToken(encoded, index)
			if !ok {
				return fail(ErrInvalidField)
			}
		}
	}
	return nil
}

func consumeToken(value []byte, index int) (int, bool) {
	start := index
	for index < len(value) && isMIMEToken(value[index]) {
		index++
	}
	return index, index != start
}

func consumeSpaces(value []byte, index int) int {
	for index < len(value) && value[index] == ' ' {
		index++
	}
	return index
}

func isMIMEToken(b byte) bool {
	return b >= '0' && b <= '9' || b >= 'A' && b <= 'Z' || b >= 'a' && b <= 'z' || strings.ContainsRune("!#$%&'*+-.^_`|~", rune(b))
}

func clone(value []byte) []byte { return append([]byte(nil), value...) }
func u64be(value uint64) []byte {
	result := make([]byte, 8)
	binary.BigEndian.PutUint64(result, value)
	return result
}
func u32be(value uint32) []byte {
	result := make([]byte, 4)
	binary.BigEndian.PutUint32(result, value)
	return result
}
