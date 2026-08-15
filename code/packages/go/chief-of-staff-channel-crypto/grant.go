package channelcrypto

import (
	"bytes"
	"crypto/rand"
	"crypto/subtle"
	"encoding/binary"
	"errors"
	"fmt"
	"sort"

	chacha20poly1305 "github.com/adhithyan15/coding-adventures/code/packages/go/chacha20-poly1305"
	hkdf "github.com/adhithyan15/coding-adventures/code/packages/go/hkdf"
	ed25519 "github.com/example/coding-adventures/code/packages/go/ed25519"
	x25519 "github.com/example/coding-adventures/code/packages/go/x25519"
)

const grantWireVersion = byte(1)

var (
	grantMagic      = []byte("D18G")
	keyGrantContext = []byte("chief-channel-key-grant-v1")
	keyWrapContext  = []byte("chief-channel-key-wrap-v1")
)

// KeyGrantErrorCode is the stable, portable failure classification defined by D18Q.
type KeyGrantErrorCode string

const (
	KeyGrantErrInvalidMagic          KeyGrantErrorCode = "invalid_magic"
	KeyGrantErrUnsupportedVersion    KeyGrantErrorCode = "unsupported_version"
	KeyGrantErrTruncatedRecord       KeyGrantErrorCode = "truncated_record"
	KeyGrantErrTrailingBytes         KeyGrantErrorCode = "trailing_bytes"
	KeyGrantErrLengthLimitExceeded   KeyGrantErrorCode = "length_limit_exceeded"
	KeyGrantErrInvalidField          KeyGrantErrorCode = "invalid_field"
	KeyGrantErrRandomnessUnavailable KeyGrantErrorCode = "randomness_unavailable"
	KeyGrantErrInvalidKeyAgreement   KeyGrantErrorCode = "invalid_key_agreement"
	KeyGrantErrKeyDerivationFailed   KeyGrantErrorCode = "key_derivation_failed"
	KeyGrantErrInvalidSignature      KeyGrantErrorCode = "invalid_signature"
	KeyGrantErrUnexpectedOriginator  KeyGrantErrorCode = "unexpected_originator"
	KeyGrantErrUnexpectedReceiver    KeyGrantErrorCode = "unexpected_receiver"
	KeyGrantErrUnexpectedChannel     KeyGrantErrorCode = "unexpected_channel"
	KeyGrantErrAuthenticationFailed  KeyGrantErrorCode = "authentication_failed"
	KeyGrantErrInvalidWrappedKey     KeyGrantErrorCode = "invalid_wrapped_key"
	KeyGrantErrConflictingGrant      KeyGrantErrorCode = "conflicting_grant"
	KeyGrantErrDecreasingEpoch       KeyGrantErrorCode = "decreasing_epoch"
	KeyGrantErrEpochExhausted        KeyGrantErrorCode = "epoch_exhausted"
	KeyGrantErrMissingEpochKey       KeyGrantErrorCode = "missing_epoch_key"
)

var keyGrantErrorCodeRoster = []KeyGrantErrorCode{
	KeyGrantErrInvalidMagic,
	KeyGrantErrUnsupportedVersion,
	KeyGrantErrTruncatedRecord,
	KeyGrantErrTrailingBytes,
	KeyGrantErrLengthLimitExceeded,
	KeyGrantErrInvalidField,
	KeyGrantErrRandomnessUnavailable,
	KeyGrantErrInvalidKeyAgreement,
	KeyGrantErrKeyDerivationFailed,
	KeyGrantErrInvalidSignature,
	KeyGrantErrUnexpectedOriginator,
	KeyGrantErrUnexpectedReceiver,
	KeyGrantErrUnexpectedChannel,
	KeyGrantErrAuthenticationFailed,
	KeyGrantErrInvalidWrappedKey,
	KeyGrantErrConflictingGrant,
	KeyGrantErrDecreasingEpoch,
	KeyGrantErrEpochExhausted,
	KeyGrantErrMissingEpochKey,
}

// KeyGrantErrorCodes returns the closed D18Q error vocabulary.
func KeyGrantErrorCodes() []KeyGrantErrorCode {
	return append([]KeyGrantErrorCode(nil), keyGrantErrorCodeRoster...)
}

// KeyGrantProfileError is one fail-closed D18Q operation error.
type KeyGrantProfileError struct{ Code KeyGrantErrorCode }

func (e *KeyGrantProfileError) Error() string { return string(e.Code) }

func grantFail(code KeyGrantErrorCode) error { return &KeyGrantProfileError{Code: code} }

// KeyGrantErrorIs reports whether err has the requested portable D18Q code.
func KeyGrantErrorIs(err error, code KeyGrantErrorCode) bool {
	var profileErr *KeyGrantProfileError
	return errors.As(err, &profileErr) && profileErr.Code == code
}

// SecureRandomSource fills one complete independent CSPRNG request.
type SecureRandomSource interface {
	Read([]byte) (int, error)
}

// ChannelMasterKey owns one 32-byte CMK with explicit logical destruction.
type ChannelMasterKey struct {
	value     [32]byte
	destroyed bool
}

// ChannelMasterKeyFromBytes copies exactly 32 caller-owned bytes.
func ChannelMasterKeyFromBytes(value []byte) (*ChannelMasterKey, error) {
	array, err := grantArray32(value)
	if err != nil {
		return nil, err
	}
	return &ChannelMasterKey{value: array}, nil
}

// GenerateChannelMasterKey obtains a new CMK from the operating-system CSPRNG.
func GenerateChannelMasterKey() (*ChannelMasterKey, error) {
	return GenerateChannelMasterKeyWithSource(rand.Reader)
}

// GenerateChannelMasterKeyWithSource obtains one complete injected CSPRNG request.
func GenerateChannelMasterKeyWithSource(source SecureRandomSource) (*ChannelMasterKey, error) {
	value, err := secureRandomBytes(source, 32)
	if err != nil {
		return nil, err
	}
	return ChannelMasterKeyFromBytes(value)
}

// Bytes returns a defensive copy of a live CMK.
func (k *ChannelMasterKey) Bytes() ([]byte, error) {
	if k == nil || k.destroyed {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	return clone(k.value[:]), nil
}

func (k *ChannelMasterKey) clone() (*ChannelMasterKey, error) {
	value, err := k.Bytes()
	if err != nil {
		return nil, err
	}
	return ChannelMasterKeyFromBytes(value)
}

// Destroy overwrites the owned CMK buffer and makes it unusable.
func (k *ChannelMasterKey) Destroy() {
	if k == nil {
		return
	}
	clear(k.value[:])
	k.destroyed = true
}

func (k ChannelMasterKey) String() string {
	if k.destroyed {
		return "ChannelMasterKey(<destroyed>)"
	}
	return "ChannelMasterKey(<secret>)"
}

func (k ChannelMasterKey) GoString() string { return k.String() }

// ReceiverKeyPair owns one X25519 private key without exposing it.
type ReceiverKeyPair struct {
	privateKey [32]byte
	publicKey  [32]byte
	destroyed  bool
}

// ReceiverKeyPairFromPrivateKey derives a key pair from exactly 32 bytes.
func ReceiverKeyPairFromPrivateKey(privateKey []byte) (*ReceiverKeyPair, error) {
	private, err := grantArray32(privateKey)
	if err != nil {
		return nil, err
	}
	public, err := x25519.GenerateKeypair(private)
	if err != nil {
		clear(private[:])
		return nil, grantFail(KeyGrantErrInvalidKeyAgreement)
	}
	return &ReceiverKeyPair{privateKey: private, publicKey: public}, nil
}

// GenerateReceiverKeyPair obtains a receiver key from the OS CSPRNG.
func GenerateReceiverKeyPair() (*ReceiverKeyPair, error) {
	return GenerateReceiverKeyPairWithSource(rand.Reader)
}

// GenerateReceiverKeyPairWithSource obtains one complete injected CSPRNG request.
func GenerateReceiverKeyPairWithSource(source SecureRandomSource) (*ReceiverKeyPair, error) {
	value, err := secureRandomBytes(source, 32)
	if err != nil {
		return nil, err
	}
	return ReceiverKeyPairFromPrivateKey(value)
}

// PublicKey returns a defensive copy of the live receiver public key.
func (k *ReceiverKeyPair) PublicKey() ([]byte, error) {
	if k == nil || k.destroyed {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	return clone(k.publicKey[:]), nil
}

func (k *ReceiverKeyPair) agree(peerPublicKey []byte) ([]byte, error) {
	if k == nil || k.destroyed {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	peer, err := grantArray32(peerPublicKey)
	if err != nil {
		return nil, err
	}
	shared, err := x25519.X25519(k.privateKey, peer)
	if err != nil {
		return nil, grantFail(KeyGrantErrInvalidKeyAgreement)
	}
	return clone(shared[:]), nil
}

func (k *ReceiverKeyPair) clone() (*ReceiverKeyPair, error) {
	if k == nil || k.destroyed {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	return &ReceiverKeyPair{privateKey: k.privateKey, publicKey: k.publicKey}, nil
}

// Destroy overwrites the owned X25519 private key.
func (k *ReceiverKeyPair) Destroy() {
	if k == nil {
		return
	}
	clear(k.privateKey[:])
	k.destroyed = true
}

func (k ReceiverKeyPair) String() string {
	if k.destroyed {
		return "ReceiverKeyPair(<destroyed>)"
	}
	return fmt.Sprintf("ReceiverKeyPair(<secret>, public_key=%x)", k.publicKey)
}

func (k ReceiverKeyPair) GoString() string { return k.String() }

// OriginatorSigningKey owns one Ed25519 secret key without exposing it.
type OriginatorSigningKey struct {
	secretKey [64]byte
	publicKey [32]byte
	destroyed bool
}

// OriginatorSigningKeyFromSeed derives an Ed25519 identity from exactly 32 bytes.
func OriginatorSigningKeyFromSeed(seed []byte) (*OriginatorSigningKey, error) {
	seedArray, err := grantArray32(seed)
	if err != nil {
		return nil, err
	}
	public, secret := ed25519.GenerateKeypair(seedArray)
	clear(seedArray[:])
	return &OriginatorSigningKey{secretKey: secret, publicKey: public}, nil
}

// GenerateOriginatorSigningKey obtains a signing identity from the OS CSPRNG.
func GenerateOriginatorSigningKey() (*OriginatorSigningKey, error) {
	return GenerateOriginatorSigningKeyWithSource(rand.Reader)
}

// GenerateOriginatorSigningKeyWithSource obtains one complete injected CSPRNG request.
func GenerateOriginatorSigningKeyWithSource(source SecureRandomSource) (*OriginatorSigningKey, error) {
	seed, err := secureRandomBytes(source, 32)
	if err != nil {
		return nil, err
	}
	return OriginatorSigningKeyFromSeed(seed)
}

// PublicKey returns a defensive copy of the live Ed25519 public key.
func (k *OriginatorSigningKey) PublicKey() ([]byte, error) {
	if k == nil || k.destroyed {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	return clone(k.publicKey[:]), nil
}

func (k *OriginatorSigningKey) sign(message []byte) ([]byte, error) {
	if k == nil || k.destroyed {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	signature := ed25519.Sign(clone(message), k.secretKey)
	return clone(signature[:]), nil
}

// Destroy overwrites the owned Ed25519 secret key.
func (k *OriginatorSigningKey) Destroy() {
	if k == nil {
		return
	}
	clear(k.secretKey[:])
	k.destroyed = true
}

func (k OriginatorSigningKey) String() string {
	if k.destroyed {
		return "OriginatorSigningKey(<destroyed>)"
	}
	return fmt.Sprintf("OriginatorSigningKey(<secret>, public_key=%x)", k.publicKey)
}

func (k OriginatorSigningKey) GoString() string { return k.String() }

// KeyGrantFields is an immutable-by-convention validated logical grant tuple.
type KeyGrantFields struct {
	originatorID []byte
	receiverID   []byte
	channelID    [16]byte
	keyEpoch     uint64
}

// NewKeyGrantFields validates and defensively copies high-level D18Q fields.
func NewKeyGrantFields(originatorID, receiverID, channelID []byte, keyEpoch uint64) (KeyGrantFields, error) {
	if err := validateGrantIdentity(originatorID); err != nil {
		return KeyGrantFields{}, err
	}
	if err := validateGrantIdentity(receiverID); err != nil {
		return KeyGrantFields{}, err
	}
	channel, err := grantChannelID(channelID)
	if err != nil {
		return KeyGrantFields{}, err
	}
	return KeyGrantFields{clone(originatorID), clone(receiverID), channel, keyEpoch}, nil
}

func (f KeyGrantFields) OriginatorID() []byte { return clone(f.originatorID) }
func (f KeyGrantFields) ReceiverID() []byte   { return clone(f.receiverID) }
func (f KeyGrantFields) ChannelID() []byte    { return clone(f.channelID[:]) }
func (f KeyGrantFields) KeyEpoch() uint64     { return f.keyEpoch }

// PortableKeyGrant is a structurally immutable public D18G grant.
type PortableKeyGrant struct {
	originatorID        []byte
	receiverID          []byte
	channelID           [16]byte
	keyEpoch            uint64
	ephemeralPublicKey  [32]byte
	wrappingNonce       [24]byte
	wrappedCMK          [48]byte
	originatorSignature [64]byte
}

func newPortableKeyGrant(originatorID, receiverID, channelID []byte, keyEpoch uint64, ephemeralPublicKey, wrappingNonce, wrappedCMK, signature []byte) (PortableKeyGrant, error) {
	if len(originatorID) > maxIdentityBytes || len(receiverID) > maxIdentityBytes {
		return PortableKeyGrant{}, grantFail(KeyGrantErrLengthLimitExceeded)
	}
	channel, err := grantArray16(channelID)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	ephemeral, err := grantArray32(ephemeralPublicKey)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	nonce, err := grantArray24(wrappingNonce)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	wrapped, err := grantArray48(wrappedCMK)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	signatureArray, err := grantArray64(signature)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	return PortableKeyGrant{
		originatorID: clone(originatorID), receiverID: clone(receiverID),
		channelID: channel, keyEpoch: keyEpoch, ephemeralPublicKey: ephemeral,
		wrappingNonce: nonce, wrappedCMK: wrapped, originatorSignature: signatureArray,
	}, nil
}

func (g PortableKeyGrant) OriginatorID() []byte        { return clone(g.originatorID) }
func (g PortableKeyGrant) ReceiverID() []byte          { return clone(g.receiverID) }
func (g PortableKeyGrant) ChannelID() []byte           { return clone(g.channelID[:]) }
func (g PortableKeyGrant) KeyEpoch() uint64            { return g.keyEpoch }
func (g PortableKeyGrant) EphemeralPublicKey() []byte  { return clone(g.ephemeralPublicKey[:]) }
func (g PortableKeyGrant) WrappingNonce() []byte       { return clone(g.wrappingNonce[:]) }
func (g PortableKeyGrant) WrappedCMK() []byte          { return clone(g.wrappedCMK[:]) }
func (g PortableKeyGrant) OriginatorSignature() []byte { return clone(g.originatorSignature[:]) }

// GrantDeserialize structurally decodes one complete bounded D18G v1 record.
func GrantDeserialize(data []byte) (PortableKeyGrant, error) {
	d := &grantDecoder{data: clone(data)}
	magic, err := d.take(4)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	if !bytes.Equal(magic, grantMagic) {
		return PortableKeyGrant{}, grantFail(KeyGrantErrInvalidMagic)
	}
	version, err := d.take(1)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	if version[0] != grantWireVersion {
		return PortableKeyGrant{}, grantFail(KeyGrantErrUnsupportedVersion)
	}
	originatorID, err := d.identity()
	if err != nil {
		return PortableKeyGrant{}, err
	}
	receiverID, err := d.identity()
	if err != nil {
		return PortableKeyGrant{}, err
	}
	channelID, err := d.take(16)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	epochBytes, err := d.take(8)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	ephemeral, err := d.take(32)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	nonce, err := d.take(24)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	wrapped, err := d.take(48)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	signature, err := d.take(64)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	if d.pos != len(d.data) {
		return PortableKeyGrant{}, grantFail(KeyGrantErrTrailingBytes)
	}
	return newPortableKeyGrant(originatorID, receiverID, channelID, binary.BigEndian.Uint64(epochBytes), ephemeral, nonce, wrapped, signature)
}

// GrantSerialize validates and encodes one immutable grant as exact D18G v1 bytes.
func GrantSerialize(grant PortableKeyGrant) ([]byte, error) {
	if err := validatePortableKeyGrant(grant); err != nil {
		return nil, err
	}
	result := make([]byte, 0, 205+len(grant.originatorID)+len(grant.receiverID))
	result = append(result, grantMagic...)
	result = append(result, grantWireVersion)
	result = append(result, u32be(uint32(len(grant.originatorID)))...)
	result = append(result, grant.originatorID...)
	result = append(result, u32be(uint32(len(grant.receiverID)))...)
	result = append(result, grant.receiverID...)
	result = append(result, grant.channelID[:]...)
	result = append(result, u64be(grant.keyEpoch)...)
	result = append(result, grant.ephemeralPublicKey[:]...)
	result = append(result, grant.wrappingNonce[:]...)
	result = append(result, grant.wrappedCMK[:]...)
	result = append(result, grant.originatorSignature[:]...)
	return result, nil
}

// SealChannelKey seals one CMK with fresh OS CSPRNG material.
func SealChannelKey(fields KeyGrantFields, cmk *ChannelMasterKey, receiverPublicKey []byte, signingKey *OriginatorSigningKey) (PortableKeyGrant, error) {
	return SealChannelKeyWithSource(fields, cmk, receiverPublicKey, signingKey, rand.Reader)
}

// SealChannelKeyWithSource seals one CMK with two complete injected CSPRNG requests.
func SealChannelKeyWithSource(fields KeyGrantFields, cmk *ChannelMasterKey, receiverPublicKey []byte, signingKey *OriginatorSigningKey, source SecureRandomSource) (PortableKeyGrant, error) {
	ephemeral, err := secureRandomBytes(source, 32)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	defer clear(ephemeral)
	nonce, err := secureRandomBytes(source, 24)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	return SealChannelKeyWithMaterial(fields, cmk, receiverPublicKey, signingKey, ephemeral, nonce)
}

// SealChannelKeyWithMaterial routes deterministic material through production primitives.
func SealChannelKeyWithMaterial(fields KeyGrantFields, cmk *ChannelMasterKey, receiverPublicKey []byte, signingKey *OriginatorSigningKey, ephemeralPrivateKey, wrappingNonce []byte) (PortableKeyGrant, error) {
	if err := validateKeyGrantFields(fields); err != nil {
		return PortableKeyGrant{}, err
	}
	receiverPublic, err := grantArray32(receiverPublicKey)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	ephemeralPrivate, err := grantArray32(ephemeralPrivateKey)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	defer clear(ephemeralPrivate[:])
	nonce, err := grantArray24(wrappingNonce)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	ephemeralPublic, err := x25519.GenerateKeypair(ephemeralPrivate)
	if err != nil {
		return PortableKeyGrant{}, grantFail(KeyGrantErrInvalidKeyAgreement)
	}
	shared, err := x25519.X25519(ephemeralPrivate, receiverPublic)
	if err != nil {
		return PortableKeyGrant{}, grantFail(KeyGrantErrInvalidKeyAgreement)
	}
	defer clear(shared[:])
	wrappingKey, err := deriveKeyGrantWrappingKey(shared[:], fields.channelID[:], fields.keyEpoch, fields.receiverID)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	defer clear(wrappingKey)
	aad := grantAAD(fields.originatorID, fields.receiverID, fields.channelID[:], fields.keyEpoch, ephemeralPublic[:])
	cmkBytes, err := cmk.Bytes()
	if err != nil {
		return PortableKeyGrant{}, err
	}
	defer clear(cmkBytes)
	ciphertext, tag, err := chacha20poly1305.XChaCha20Poly1305AEADEncrypt(cmkBytes, wrappingKey, nonce[:], aad)
	if err != nil {
		return PortableKeyGrant{}, grantFail(KeyGrantErrAuthenticationFailed)
	}
	wrapped := append(clone(ciphertext), tag...)
	signatureInput := grantSignatureInput(fields.originatorID, fields.receiverID, fields.channelID[:], fields.keyEpoch, ephemeralPublic[:], nonce[:], wrapped)
	signature, err := signingKey.sign(signatureInput)
	if err != nil {
		return PortableKeyGrant{}, err
	}
	return newPortableKeyGrant(fields.originatorID, fields.receiverID, fields.channelID[:], fields.keyEpoch, ephemeralPublic[:], nonce[:], wrapped, signature)
}

// OpenChannelKeyGrant verifies bindings in normative order before unwrapping a CMK.
func OpenChannelKeyGrant(grant PortableKeyGrant, expectedOriginatorID, expectedReceiverID, expectedChannelID []byte, receiverKeyPair *ReceiverKeyPair, originatorPublicKey []byte) (*ChannelMasterKey, error) {
	if err := validatePortableKeyGrant(grant); err != nil {
		return nil, err
	}
	channel, err := grantArray16(expectedChannelID)
	if err != nil {
		return nil, err
	}
	publicKey, err := grantArray32(originatorPublicKey)
	if err != nil {
		return nil, err
	}
	if !constantTimeEqual(grant.originatorID, expectedOriginatorID) {
		return nil, grantFail(KeyGrantErrUnexpectedOriginator)
	}
	if !constantTimeEqual(grant.receiverID, expectedReceiverID) {
		return nil, grantFail(KeyGrantErrUnexpectedReceiver)
	}
	if subtle.ConstantTimeCompare(grant.channelID[:], channel[:]) != 1 {
		return nil, grantFail(KeyGrantErrUnexpectedChannel)
	}
	signatureInput := grantSignatureInput(grant.originatorID, grant.receiverID, grant.channelID[:], grant.keyEpoch, grant.ephemeralPublicKey[:], grant.wrappingNonce[:], grant.wrappedCMK[:])
	if !ed25519.Verify(signatureInput, grant.originatorSignature, publicKey) {
		return nil, grantFail(KeyGrantErrInvalidSignature)
	}
	shared, err := receiverKeyPair.agree(grant.ephemeralPublicKey[:])
	if err != nil {
		return nil, err
	}
	defer clear(shared)
	wrappingKey, err := deriveKeyGrantWrappingKey(shared, grant.channelID[:], grant.keyEpoch, grant.receiverID)
	if err != nil {
		return nil, err
	}
	defer clear(wrappingKey)
	aad := grantAAD(grant.originatorID, grant.receiverID, grant.channelID[:], grant.keyEpoch, grant.ephemeralPublicKey[:])
	plaintext, err := chacha20poly1305.XChaCha20Poly1305AEADDecrypt(grant.wrappedCMK[:32], wrappingKey, grant.wrappingNonce[:], aad, grant.wrappedCMK[32:])
	if err != nil {
		return nil, grantFail(KeyGrantErrAuthenticationFailed)
	}
	defer clear(plaintext)
	if len(plaintext) != 32 {
		return nil, grantFail(KeyGrantErrInvalidWrappedKey)
	}
	return ChannelMasterKeyFromBytes(plaintext)
}

// GrantInstallOutcome is the stable receiver installation result.
type GrantInstallOutcome string

const (
	GrantInstalled  GrantInstallOutcome = "installed"
	GrantIdempotent GrantInstallOutcome = "idempotent"
)

// ReceiverEpochKeys retains receiver-local keys for one immutable identity tuple.
type ReceiverEpochKeys struct {
	originatorID        []byte
	receiverID          []byte
	channelID           [16]byte
	receiverKeyPair     *ReceiverKeyPair
	originatorPublicKey [32]byte
	epochKeys           map[uint64]*ChannelMasterKey
	latestGrant         *PortableKeyGrant
}

// NewReceiverEpochKeys constructs empty receiver-local state.
func NewReceiverEpochKeys(originatorID, receiverID, channelID []byte, receiverKeyPair *ReceiverKeyPair, originatorPublicKey []byte) (*ReceiverEpochKeys, error) {
	if err := validateGrantIdentity(originatorID); err != nil {
		return nil, err
	}
	if err := validateGrantIdentity(receiverID); err != nil {
		return nil, err
	}
	channel, err := grantChannelID(channelID)
	if err != nil {
		return nil, err
	}
	public, err := grantArray32(originatorPublicKey)
	if err != nil {
		return nil, err
	}
	receiverCopy, err := receiverKeyPair.clone()
	if err != nil {
		return nil, err
	}
	return &ReceiverEpochKeys{
		originatorID: clone(originatorID), receiverID: clone(receiverID), channelID: channel,
		receiverKeyPair: receiverCopy, originatorPublicKey: public,
		epochKeys: make(map[uint64]*ChannelMasterKey),
	}, nil
}

// ReceiverPublicKey returns the receiver public key used for grants.
func (s *ReceiverEpochKeys) ReceiverPublicKey() ([]byte, error) {
	if s == nil || s.receiverKeyPair == nil {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	return s.receiverKeyPair.PublicKey()
}

// LatestEpoch returns the newest installed epoch and whether one exists.
func (s *ReceiverEpochKeys) LatestEpoch() (uint64, bool) {
	if s == nil || s.latestGrant == nil {
		return 0, false
	}
	return s.latestGrant.keyEpoch, true
}

// InstallGrant atomically enforces retry, conflict, and monotonic ordering rules.
func (s *ReceiverEpochKeys) InstallGrant(grant PortableKeyGrant) (GrantInstallOutcome, error) {
	if s == nil || s.receiverKeyPair == nil {
		return "", grantFail(KeyGrantErrInvalidField)
	}
	if s.latestGrant != nil {
		if grant.keyEpoch < s.latestGrant.keyEpoch {
			return "", grantFail(KeyGrantErrDecreasingEpoch)
		}
		if grant.keyEpoch == s.latestGrant.keyEpoch {
			if grantsEqual(grant, *s.latestGrant) {
				return GrantIdempotent, nil
			}
			return "", grantFail(KeyGrantErrConflictingGrant)
		}
	}
	if err := validatePortableKeyGrant(grant); err != nil {
		return "", err
	}
	key, err := OpenChannelKeyGrant(grant, s.originatorID, s.receiverID, s.channelID[:], s.receiverKeyPair, s.originatorPublicKey[:])
	if err != nil {
		return "", err
	}
	retained := grant
	s.epochKeys[grant.keyEpoch] = key
	s.latestGrant = &retained
	return GrantInstalled, nil
}

// Key returns an independent managed copy of one retained epoch key.
func (s *ReceiverEpochKeys) Key(epoch uint64) (*ChannelMasterKey, error) {
	if s == nil {
		return nil, grantFail(KeyGrantErrMissingEpochKey)
	}
	key := s.epochKeys[epoch]
	if key == nil {
		return nil, grantFail(KeyGrantErrMissingEpochKey)
	}
	return key.clone()
}

// Destroy overwrites every retained local secret.
func (s *ReceiverEpochKeys) Destroy() {
	if s == nil {
		return
	}
	for epoch, key := range s.epochKeys {
		key.Destroy()
		delete(s.epochKeys, epoch)
	}
	if s.receiverKeyPair != nil {
		s.receiverKeyPair.Destroy()
	}
	s.latestGrant = nil
}

func (s ReceiverEpochKeys) String() string {
	latest := "none"
	if s.latestGrant != nil {
		latest = fmt.Sprintf("%d", s.latestGrant.keyEpoch)
	}
	return fmt.Sprintf("ReceiverEpochKeys(<secret>, latest_epoch=%s)", latest)
}

func (s ReceiverEpochKeys) GoString() string { return s.String() }

// RotationReceiver binds one authorized receiver to independent one-shot seal material.
type RotationReceiver struct {
	receiverID          []byte
	publicKey           [32]byte
	ephemeralPrivateKey [32]byte
	wrappingNonce       [24]byte
	destroyed           bool
}

// NewRotationReceiverWithMaterial validates deterministic rotation material.
func NewRotationReceiverWithMaterial(receiverID, publicKey, ephemeralPrivateKey, wrappingNonce []byte) (*RotationReceiver, error) {
	if err := validateGrantIdentity(receiverID); err != nil {
		return nil, err
	}
	public, err := grantArray32(publicKey)
	if err != nil {
		return nil, err
	}
	ephemeral, err := grantArray32(ephemeralPrivateKey)
	if err != nil {
		return nil, err
	}
	nonce, err := grantArray24(wrappingNonce)
	if err != nil {
		return nil, err
	}
	return &RotationReceiver{clone(receiverID), public, ephemeral, nonce, false}, nil
}

// GenerateRotationReceiver obtains independent seal material from the OS CSPRNG.
func GenerateRotationReceiver(receiverID, publicKey []byte) (*RotationReceiver, error) {
	return GenerateRotationReceiverWithSource(receiverID, publicKey, rand.Reader)
}

// GenerateRotationReceiverWithSource obtains two complete injected CSPRNG requests.
func GenerateRotationReceiverWithSource(receiverID, publicKey []byte, source SecureRandomSource) (*RotationReceiver, error) {
	ephemeral, err := secureRandomBytes(source, 32)
	if err != nil {
		return nil, err
	}
	defer clear(ephemeral)
	nonce, err := secureRandomBytes(source, 24)
	if err != nil {
		return nil, err
	}
	return NewRotationReceiverWithMaterial(receiverID, publicKey, ephemeral, nonce)
}

func (r *RotationReceiver) ReceiverID() []byte {
	if r == nil {
		return nil
	}
	return clone(r.receiverID)
}

// Destroy overwrites the receiver's one-shot ephemeral private key.
func (r *RotationReceiver) Destroy() {
	if r == nil {
		return
	}
	clear(r.ephemeralPrivateKey[:])
	r.destroyed = true
}

func (r RotationReceiver) String() string {
	state := "secret"
	if r.destroyed {
		state = "destroyed"
	}
	return fmt.Sprintf("RotationReceiver(<%s>, receiver_id=%x)", state, r.receiverID)
}

func (r RotationReceiver) GoString() string { return r.String() }

// RotationPlan is a pure prospective result; durable activation remains separate.
type RotationPlan struct {
	newEpoch uint64
	newCMK   *ChannelMasterKey
	grants   []PortableKeyGrant
}

func (p *RotationPlan) NewEpoch() uint64 {
	if p == nil {
		return 0
	}
	return p.newEpoch
}

// NewCMK returns an independent managed copy of the prospective CMK.
func (p *RotationPlan) NewCMK() (*ChannelMasterKey, error) {
	if p == nil || p.newCMK == nil {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	return p.newCMK.clone()
}

// Grants returns a copy of the receiver-sorted immutable grants.
func (p *RotationPlan) Grants() []PortableKeyGrant {
	if p == nil {
		return nil
	}
	return append([]PortableKeyGrant(nil), p.grants...)
}

// Destroy overwrites the plan-owned prospective CMK.
func (p *RotationPlan) Destroy() {
	if p != nil && p.newCMK != nil {
		p.newCMK.Destroy()
	}
}

func (p RotationPlan) String() string {
	return fmt.Sprintf("RotationPlan(<secret>, new_epoch=%d, grants=%d)", p.newEpoch, len(p.grants))
}

func (p RotationPlan) GoString() string { return p.String() }

// PlanRotation returns a complete receiver-sorted successor plan or no partial plan.
func PlanRotation(originatorID, channelID []byte, currentEpoch uint64, newCMK *ChannelMasterKey, receivers []*RotationReceiver, signingKey *OriginatorSigningKey) (*RotationPlan, error) {
	if err := validateGrantIdentity(originatorID); err != nil {
		return nil, err
	}
	channel, err := grantChannelID(channelID)
	if err != nil {
		return nil, err
	}
	if currentEpoch == ^uint64(0) {
		return nil, grantFail(KeyGrantErrEpochExhausted)
	}
	if len(receivers) == 0 {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	ordered := append([]*RotationReceiver(nil), receivers...)
	defer func() {
		for _, receiver := range ordered {
			receiver.Destroy()
		}
	}()
	for _, receiver := range ordered {
		if receiver == nil || receiver.destroyed {
			return nil, grantFail(KeyGrantErrInvalidField)
		}
	}
	sort.Slice(ordered, func(i, j int) bool { return bytes.Compare(ordered[i].receiverID, ordered[j].receiverID) < 0 })
	for i := 1; i < len(ordered); i++ {
		if bytes.Equal(ordered[i-1].receiverID, ordered[i].receiverID) {
			return nil, grantFail(KeyGrantErrInvalidField)
		}
	}
	planCMK, err := newCMK.clone()
	if err != nil {
		return nil, err
	}
	grants := make([]PortableKeyGrant, 0, len(ordered))
	for _, receiver := range ordered {
		fields, err := NewKeyGrantFields(originatorID, receiver.receiverID, channel[:], currentEpoch+1)
		if err != nil {
			planCMK.Destroy()
			return nil, err
		}
		grant, err := SealChannelKeyWithMaterial(fields, newCMK, receiver.publicKey[:], signingKey, receiver.ephemeralPrivateKey[:], receiver.wrappingNonce[:])
		if err != nil {
			planCMK.Destroy()
			return nil, err
		}
		grants = append(grants, grant)
	}
	return &RotationPlan{newEpoch: currentEpoch + 1, newCMK: planCMK, grants: grants}, nil
}

// SecretErasureCapability is the closed D18Q erasure-strength vocabulary.
type SecretErasureCapability string

const (
	SecretErasureGuaranteed     SecretErasureCapability = "guaranteed"
	SecretErasureBestEffort     SecretErasureCapability = "best_effort"
	SecretErasureNotEnforceable SecretErasureCapability = "not_enforceable"
)

// GrantSecretErasureCapability reports Go's honest controlled-destruction strength.
func GrantSecretErasureCapability() SecretErasureCapability { return SecretErasureBestEffort }

// KeyGrantHKDFSalt returns the canonical D18Q HKDF salt for diagnostics.
func KeyGrantHKDFSalt(channelID []byte, keyEpoch uint64) ([]byte, error) {
	channel, err := grantArray16(channelID)
	if err != nil {
		return nil, err
	}
	return grantFrame(channel[:], u64be(keyEpoch)), nil
}

// KeyGrantHKDFInfo returns the canonical receiver-specific HKDF info.
func KeyGrantHKDFInfo(receiverID []byte) ([]byte, error) {
	if len(receiverID) > maxIdentityBytes {
		return nil, grantFail(KeyGrantErrLengthLimitExceeded)
	}
	return grantFrame(keyWrapContext, receiverID), nil
}

// KeyGrantAAD returns the canonical grant AAD for diagnostics.
func KeyGrantAAD(grant PortableKeyGrant) []byte {
	return grantAAD(grant.originatorID, grant.receiverID, grant.channelID[:], grant.keyEpoch, grant.ephemeralPublicKey[:])
}

// KeyGrantSignatureInput returns the canonical signature input for diagnostics.
func KeyGrantSignatureInput(grant PortableKeyGrant) []byte {
	return grantSignatureInput(grant.originatorID, grant.receiverID, grant.channelID[:], grant.keyEpoch, grant.ephemeralPublicKey[:], grant.wrappingNonce[:], grant.wrappedCMK[:])
}

// KeyGrantWrappingKey returns the canonical wrapping key for fixture diagnostics.
func KeyGrantWrappingKey(sharedSecret, channelID []byte, keyEpoch uint64, receiverID []byte) ([]byte, error) {
	return deriveKeyGrantWrappingKey(clone(sharedSecret), clone(channelID), keyEpoch, clone(receiverID))
}

func validateKeyGrantFields(fields KeyGrantFields) error {
	if err := validateGrantIdentity(fields.originatorID); err != nil {
		return err
	}
	if err := validateGrantIdentity(fields.receiverID); err != nil {
		return err
	}
	_, err := grantChannelID(fields.channelID[:])
	return err
}

func validatePortableKeyGrant(grant PortableKeyGrant) error {
	if err := validateGrantIdentity(grant.originatorID); err != nil {
		return err
	}
	if err := validateGrantIdentity(grant.receiverID); err != nil {
		return err
	}
	_, err := grantChannelID(grant.channelID[:])
	return err
}

func validateGrantIdentity(identity []byte) error {
	if len(identity) == 0 {
		return grantFail(KeyGrantErrInvalidField)
	}
	if len(identity) > maxIdentityBytes {
		return grantFail(KeyGrantErrLengthLimitExceeded)
	}
	return nil
}

func grantChannelID(value []byte) ([16]byte, error) {
	result, err := grantArray16(value)
	if err != nil {
		return [16]byte{}, err
	}
	if result[6]>>4 != 7 || result[8]>>6 != 2 {
		return [16]byte{}, grantFail(KeyGrantErrInvalidField)
	}
	return result, nil
}

func deriveKeyGrantWrappingKey(sharedSecret, channelID []byte, keyEpoch uint64, receiverID []byte) ([]byte, error) {
	if len(sharedSecret) != 32 {
		return nil, grantFail(KeyGrantErrInvalidField)
	}
	salt, err := KeyGrantHKDFSalt(channelID, keyEpoch)
	if err != nil {
		return nil, err
	}
	info, err := KeyGrantHKDFInfo(receiverID)
	if err != nil {
		return nil, err
	}
	key, err := hkdf.HKDF(salt, sharedSecret, info, 32, hkdf.SHA256)
	if err != nil || len(key) != 32 {
		return nil, grantFail(KeyGrantErrKeyDerivationFailed)
	}
	return key, nil
}

func grantAAD(originatorID, receiverID, channelID []byte, keyEpoch uint64, ephemeralPublicKey []byte) []byte {
	return grantFrame(keyGrantContext, originatorID, channelID, u64be(keyEpoch), receiverID, ephemeralPublicKey)
}

func grantSignatureInput(originatorID, receiverID, channelID []byte, keyEpoch uint64, ephemeralPublicKey, wrappingNonce, wrappedCMK []byte) []byte {
	return grantFrame(keyGrantContext, originatorID, channelID, u64be(keyEpoch), receiverID, ephemeralPublicKey, wrappingNonce, wrappedCMK)
}

func grantFrame(fields ...[]byte) []byte {
	total := 0
	for _, field := range fields {
		total += 8 + len(field)
	}
	result := make([]byte, 0, total)
	for _, field := range fields {
		result = append(result, u64be(uint64(len(field)))...)
		result = append(result, field...)
	}
	return result
}

func secureRandomBytes(source SecureRandomSource, length int) ([]byte, error) {
	if source == nil || length < 0 {
		return nil, grantFail(KeyGrantErrRandomnessUnavailable)
	}
	value := make([]byte, length)
	count, err := source.Read(value)
	if err != nil || count != length {
		clear(value)
		return nil, grantFail(KeyGrantErrRandomnessUnavailable)
	}
	return value, nil
}

func constantTimeEqual(left, right []byte) bool {
	return len(left) == len(right) && subtle.ConstantTimeCompare(left, right) == 1
}

func grantsEqual(left, right PortableKeyGrant) bool {
	return bytes.Equal(left.originatorID, right.originatorID) &&
		bytes.Equal(left.receiverID, right.receiverID) &&
		left.channelID == right.channelID && left.keyEpoch == right.keyEpoch &&
		left.ephemeralPublicKey == right.ephemeralPublicKey &&
		left.wrappingNonce == right.wrappingNonce &&
		left.wrappedCMK == right.wrappedCMK &&
		left.originatorSignature == right.originatorSignature
}

func grantArray16(value []byte) ([16]byte, error) {
	if len(value) != 16 {
		return [16]byte{}, grantFail(KeyGrantErrInvalidField)
	}
	var result [16]byte
	copy(result[:], value)
	return result, nil
}

func grantArray24(value []byte) ([24]byte, error) {
	if len(value) != 24 {
		return [24]byte{}, grantFail(KeyGrantErrInvalidField)
	}
	var result [24]byte
	copy(result[:], value)
	return result, nil
}

func grantArray32(value []byte) ([32]byte, error) {
	if len(value) != 32 {
		return [32]byte{}, grantFail(KeyGrantErrInvalidField)
	}
	var result [32]byte
	copy(result[:], value)
	return result, nil
}

func grantArray48(value []byte) ([48]byte, error) {
	if len(value) != 48 {
		return [48]byte{}, grantFail(KeyGrantErrInvalidField)
	}
	var result [48]byte
	copy(result[:], value)
	return result, nil
}

func grantArray64(value []byte) ([64]byte, error) {
	if len(value) != 64 {
		return [64]byte{}, grantFail(KeyGrantErrInvalidField)
	}
	var result [64]byte
	copy(result[:], value)
	return result, nil
}

type grantDecoder struct {
	data []byte
	pos  int
}

func (d *grantDecoder) take(length int) ([]byte, error) {
	if length < 0 || length > len(d.data)-d.pos {
		return nil, grantFail(KeyGrantErrTruncatedRecord)
	}
	value := clone(d.data[d.pos : d.pos+length])
	d.pos += length
	return value, nil
}

func (d *grantDecoder) identity() ([]byte, error) {
	lengthBytes, err := d.take(4)
	if err != nil {
		return nil, err
	}
	length := uint64(binary.BigEndian.Uint32(lengthBytes))
	if length > maxIdentityBytes {
		return nil, grantFail(KeyGrantErrLengthLimitExceeded)
	}
	if length > uint64(len(d.data)-d.pos) {
		return nil, grantFail(KeyGrantErrTruncatedRecord)
	}
	return d.take(int(length))
}
