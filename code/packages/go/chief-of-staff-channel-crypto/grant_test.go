package channelcrypto

import (
	"bytes"
	"encoding/base64"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"testing"

	x25519 "github.com/example/coding-adventures/code/packages/go/x25519"
)

type grantFixtureManifest struct {
	FixtureFormat               string                 `json:"fixture_format"`
	Spec                        string                 `json:"spec"`
	GeneratorBlobSHA1           string                 `json:"generator_blob_sha1"`
	Warning                     string                 `json:"warning"`
	Constants                   map[string]string      `json:"constants"`
	TestSigningKey              grantSigningFixture    `json:"test_signing_key"`
	PositiveCases               []grantPositiveFixture `json:"positive_cases"`
	StructuralNegativeCases     []grantRecordFixture   `json:"structural_negative_cases"`
	TruncatedPrefixRecipe       grantTruncatedRecipe   `json:"truncated_prefix_recipe"`
	OversizeRecipes             []grantOversizeRecipe  `json:"oversize_recipes"`
	FieldNegativeCases          []grantNamedError      `json:"field_negative_cases"`
	SealNegativeCases           []grantNamedError      `json:"seal_negative_cases"`
	OpeningNegativeCases        []grantOpeningFixture  `json:"opening_negative_cases"`
	ReceiverStateTrace          grantReceiverTrace     `json:"receiver_state_trace"`
	RotationCase                grantRotationFixture   `json:"rotation_case"`
	SecretErasureCapabilities   []string               `json:"secret_erasure_capabilities"`
	RustSecretErasureCapability string                 `json:"rust_secret_erasure_capability"`
	StableErrorCodes            []string               `json:"stable_error_codes"`
}

type grantSigningFixture struct {
	SeedHex      string `json:"seed_hex"`
	PublicKeyHex string `json:"public_key_hex"`
}

type grantPositiveFixture struct {
	Name                   string `json:"name"`
	OriginatorIDB64        string `json:"originator_id_b64"`
	ReceiverIDB64          string `json:"receiver_id_b64"`
	ChannelIDHex           string `json:"channel_id_hex"`
	KeyEpoch               string `json:"key_epoch"`
	CMKHex                 string `json:"cmk_hex"`
	ReceiverPrivateKeyHex  string `json:"receiver_private_key_hex"`
	ReceiverPublicKeyHex   string `json:"receiver_public_key_hex"`
	EphemeralPrivateKeyHex string `json:"ephemeral_private_key_hex"`
	EphemeralPublicKeyHex  string `json:"ephemeral_public_key_hex"`
	SharedSecretHex        string `json:"shared_secret_hex"`
	HKDFSaltB64            string `json:"hkdf_salt_b64"`
	HKDFInfoB64            string `json:"hkdf_info_b64"`
	WrappingKeyHex         string `json:"wrapping_key_hex"`
	WrappingNonceHex       string `json:"wrapping_nonce_hex"`
	GrantAADB64            string `json:"grant_aad_b64"`
	WrappedCMKHex          string `json:"wrapped_cmk_hex"`
	SignatureInputB64      string `json:"signature_input_b64"`
	SignatureHex           string `json:"signature_hex"`
	D18GB64                string `json:"d18g_b64"`
	ExpectedOpenedCMKHex   string `json:"expected_opened_cmk_hex"`
}

type grantNamedError struct {
	Name          string `json:"name"`
	ExpectedError string `json:"expected_error"`
}

type grantRecordFixture struct {
	Name          string `json:"name"`
	D18GB64       string `json:"d18g_b64"`
	ExpectedError string `json:"expected_error"`
}

type grantTruncatedRecipe struct {
	SourceCase          string `json:"source_case"`
	FirstLength         string `json:"first_length"`
	LastLengthExclusive string `json:"last_length_exclusive"`
	ExpectedError       string `json:"expected_error"`
}

type grantOversizeRecipe struct {
	Field          string `json:"field"`
	LengthOffset   string `json:"length_offset"`
	DeclaredLength string `json:"declared_length"`
	ExpectedError  string `json:"expected_error"`
}

type grantOpeningFixture struct {
	Name                    string `json:"name"`
	D18GB64                 string `json:"d18g_b64"`
	ExpectedOriginatorIDB64 string `json:"expected_originator_id_b64"`
	ExpectedReceiverIDB64   string `json:"expected_receiver_id_b64"`
	ExpectedChannelIDHex    string `json:"expected_channel_id_hex"`
	ReceiverPrivateKeyHex   string `json:"receiver_private_key_hex"`
	ExpectedError           string `json:"expected_error"`
}

type grantReceiverTrace struct {
	Grants            map[string]string   `json:"grants"`
	Steps             []grantReceiverStep `json:"steps"`
	MissingEpoch      string              `json:"missing_epoch"`
	MissingEpochError string              `json:"missing_epoch_error"`
}

type grantReceiverStep struct {
	Name           string   `json:"name"`
	Grant          string   `json:"grant"`
	Expected       string   `json:"expected"`
	LatestEpoch    string   `json:"latest_epoch"`
	RetainedEpochs []string `json:"retained_epochs"`
}

type grantRotationFixture struct {
	Name                     string   `json:"name"`
	CurrentEpoch             string   `json:"current_epoch"`
	NewEpoch                 string   `json:"new_epoch"`
	NewCMKHex                string   `json:"new_cmk_hex"`
	AuthorizedReceiverIDsB64 []string `json:"authorized_receiver_ids_b64"`
	NewGrantsB64             []string `json:"new_grants_b64"`
	ReceiverARetainsEpochs   []string `json:"receiver_a_retains_epochs"`
	ReceiverBRetainsEpochs   []string `json:"receiver_b_retains_epochs"`
	ReceiverANewGrant        *string  `json:"receiver_a_new_grant"`
}

type grantFixtureContext struct {
	manifest  grantFixtureManifest
	seed      []byte
	publicKey []byte
	signer    *OriginatorSigningKey
	channelID []byte
}

func loadGrantFixtures(t *testing.T) grantFixtureContext {
	t.Helper()
	path := grantFixturePath()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var manifest grantFixtureManifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		t.Fatal(err)
	}
	seed := decodeHex(t, manifest.TestSigningKey.SeedHex)
	signer, err := OriginatorSigningKeyFromSeed(seed)
	if err != nil {
		t.Fatal(err)
	}
	return grantFixtureContext{
		manifest: manifest, seed: seed,
		publicKey: decodeHex(t, manifest.TestSigningKey.PublicKeyHex), signer: signer,
		channelID: decodeHex(t, manifest.PositiveCases[0].ChannelIDHex),
	}
}

func grantFixturePath() string {
	return filepath.Join("..", "..", "..", "fixtures", "chief-of-staff-channel-key-grant", "v1", "manifest.json")
}

func decodeGrantB64(t *testing.T, value string) []byte {
	t.Helper()
	decoded, err := base64.StdEncoding.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return decoded
}

func decodeHex(t *testing.T, value string) []byte {
	t.Helper()
	decoded, err := hex.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return decoded
}

func parseFixtureUint(t *testing.T, value string) uint64 {
	t.Helper()
	parsed, err := strconv.ParseUint(value, 10, 64)
	if err != nil {
		t.Fatal(err)
	}
	return parsed
}

func parseFixtureInt(t *testing.T, value string) int {
	t.Helper()
	parsed, err := strconv.Atoi(value)
	if err != nil {
		t.Fatal(err)
	}
	return parsed
}

func parseFixtureUint32(t *testing.T, value string) uint32 {
	t.Helper()
	parsed, err := strconv.ParseUint(value, 10, 32)
	if err != nil {
		t.Fatal(err)
	}
	return uint32(parsed)
}

func requireGrantCode(t *testing.T, err error, expected string) {
	t.Helper()
	if !KeyGrantErrorIs(err, KeyGrantErrorCode(expected)) {
		t.Fatalf("error = %v, want D18Q code %q", err, expected)
	}
	if err == nil || err.Error() != expected {
		t.Fatalf("error text = %v, want %q", err, expected)
	}
}

func namesOf[T interface{ fixtureName() string }](values []T) []string {
	result := make([]string, len(values))
	for i, value := range values {
		result[i] = value.fixtureName()
	}
	return result
}

func (f grantPositiveFixture) fixtureName() string { return f.Name }
func (f grantRecordFixture) fixtureName() string   { return f.Name }
func (f grantNamedError) fixtureName() string      { return f.Name }
func (f grantOpeningFixture) fixtureName() string  { return f.Name }

func sortedJSONKeys(value map[string]json.RawMessage) []string {
	keys := make([]string, 0, len(value))
	for key := range value {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

func requireJSONKeys(t *testing.T, value map[string]json.RawMessage, expected ...string) {
	t.Helper()
	sort.Strings(expected)
	if !reflect.DeepEqual(sortedJSONKeys(value), expected) {
		t.Fatalf("JSON fields = %v, want %v", sortedJSONKeys(value), expected)
	}
}

func TestGrantManifestFieldRostersAreClosed(t *testing.T) {
	data, err := os.ReadFile(grantFixturePath())
	if err != nil {
		t.Fatal(err)
	}
	var raw map[string]json.RawMessage
	if err := json.Unmarshal(data, &raw); err != nil {
		t.Fatal(err)
	}
	requireJSONKeys(t, raw,
		"fixture_format", "spec", "generator_blob_sha1", "warning", "constants",
		"test_signing_key", "positive_cases", "structural_negative_cases",
		"truncated_prefix_recipe", "oversize_recipes", "field_negative_cases",
		"seal_negative_cases", "opening_negative_cases", "receiver_state_trace",
		"rotation_case", "secret_erasure_capabilities",
		"rust_secret_erasure_capability", "stable_error_codes",
	)
	var signing map[string]json.RawMessage
	if err := json.Unmarshal(raw["test_signing_key"], &signing); err != nil {
		t.Fatal(err)
	}
	requireJSONKeys(t, signing, "seed_hex", "public_key_hex")
	var constants map[string]json.RawMessage
	if err := json.Unmarshal(raw["constants"], &constants); err != nil {
		t.Fatal(err)
	}
	requireJSONKeys(t, constants, "key_grant_context_ascii", "key_wrap_context_ascii", "max_identity_bytes", "wire_magic_ascii", "wire_version")
	arrayRosters := map[string][]string{
		"positive_cases": {
			"name", "originator_id_b64", "receiver_id_b64", "channel_id_hex", "key_epoch",
			"cmk_hex", "receiver_private_key_hex", "receiver_public_key_hex",
			"ephemeral_private_key_hex", "ephemeral_public_key_hex", "shared_secret_hex",
			"hkdf_salt_b64", "hkdf_info_b64", "wrapping_key_hex", "wrapping_nonce_hex",
			"grant_aad_b64", "wrapped_cmk_hex", "signature_input_b64", "signature_hex",
			"d18g_b64", "expected_opened_cmk_hex",
		},
		"structural_negative_cases": {"name", "d18g_b64", "expected_error"},
		"oversize_recipes":          {"field", "length_offset", "declared_length", "expected_error"},
		"field_negative_cases":      {"name", "expected_error"},
		"seal_negative_cases":       {"name", "expected_error"},
		"opening_negative_cases": {
			"name", "d18g_b64", "expected_originator_id_b64", "expected_receiver_id_b64",
			"expected_channel_id_hex", "receiver_private_key_hex", "expected_error",
		},
	}
	for field, expected := range arrayRosters {
		var cases []map[string]json.RawMessage
		if err := json.Unmarshal(raw[field], &cases); err != nil {
			t.Fatal(err)
		}
		for _, testCase := range cases {
			requireJSONKeys(t, testCase, expected...)
		}
	}
	var truncated map[string]json.RawMessage
	if err := json.Unmarshal(raw["truncated_prefix_recipe"], &truncated); err != nil {
		t.Fatal(err)
	}
	requireJSONKeys(t, truncated, "source_case", "first_length", "last_length_exclusive", "expected_error")
	var trace map[string]json.RawMessage
	if err := json.Unmarshal(raw["receiver_state_trace"], &trace); err != nil {
		t.Fatal(err)
	}
	requireJSONKeys(t, trace, "grants", "steps", "missing_epoch", "missing_epoch_error")
	var traceGrants map[string]json.RawMessage
	if err := json.Unmarshal(trace["grants"], &traceGrants); err != nil {
		t.Fatal(err)
	}
	requireJSONKeys(t, traceGrants, "epoch_zero_b64", "same_epoch_conflict_b64", "failed_higher_epoch_b64", "skipped_epoch_three_b64")
	var steps []map[string]json.RawMessage
	if err := json.Unmarshal(trace["steps"], &steps); err != nil {
		t.Fatal(err)
	}
	for _, step := range steps {
		requireJSONKeys(t, step, "name", "grant", "expected", "latest_epoch", "retained_epochs")
	}
	var rotation map[string]json.RawMessage
	if err := json.Unmarshal(raw["rotation_case"], &rotation); err != nil {
		t.Fatal(err)
	}
	requireJSONKeys(t, rotation, "name", "current_epoch", "new_epoch", "new_cmk_hex", "authorized_receiver_ids_b64", "new_grants_b64", "receiver_a_retains_epochs", "receiver_b_retains_epochs", "receiver_a_new_grant")
}

func TestGrantManifestTopologyVocabularyAndErasureAreClosed(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	manifest := fixtures.manifest
	if manifest.FixtureFormat != "D18Q-channel-key-grant-fixtures-v1" ||
		manifest.Spec != "code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md" ||
		len(manifest.GeneratorBlobSHA1) != 40 || !strings.Contains(manifest.Warning, "test-only") ||
		!strings.Contains(manifest.Warning, "Never log") {
		t.Fatal("fixture provenance metadata changed")
	}
	wantConstants := map[string]string{
		"key_grant_context_ascii": "chief-channel-key-grant-v1",
		"key_wrap_context_ascii":  "chief-channel-key-wrap-v1",
		"max_identity_bytes":      "4096", "wire_magic_ascii": "D18G", "wire_version": "1",
	}
	if !reflect.DeepEqual(manifest.Constants, wantConstants) {
		t.Fatalf("constants = %#v", manifest.Constants)
	}
	codes := KeyGrantErrorCodes()
	gotCodes := make([]string, len(codes))
	for i, code := range codes {
		gotCodes[i] = string(code)
	}
	if !reflect.DeepEqual(gotCodes, manifest.StableErrorCodes) {
		t.Fatalf("stable errors = %v", gotCodes)
	}
	if !reflect.DeepEqual(manifest.SecretErasureCapabilities, []string{"guaranteed", "best_effort", "not_enforceable"}) ||
		GrantSecretErasureCapability() != SecretErasureBestEffort || manifest.RustSecretErasureCapability != "guaranteed" {
		t.Fatal("secret-erasure vocabulary changed")
	}
	if !reflect.DeepEqual(namesOf(manifest.PositiveCases), []string{"epoch-zero-receiver-a", "epoch-zero-receiver-b", "maximum-epoch-receiver-a"}) ||
		!reflect.DeepEqual(namesOf(manifest.StructuralNegativeCases), []string{"wrong-magic", "unsupported-version", "trailing-byte"}) ||
		!reflect.DeepEqual(namesOf(manifest.FieldNegativeCases), []string{"empty-originator", "empty-receiver", "invalid-uuid-version", "invalid-uuid-variant", "oversized-originator", "oversized-receiver"}) ||
		!reflect.DeepEqual(namesOf(manifest.SealNegativeCases), []string{"low-order-receiver-public-key"}) ||
		!reflect.DeepEqual(namesOf(manifest.OpeningNegativeCases), []string{"unexpected-originator", "unexpected-receiver", "unexpected-channel", "invalid-signature", "invalid-signature-before-key-agreement", "low-order-ephemeral-public-key", "wrong-receiver-private-key", "wrong-wrapping-nonce", "mutated-wrapped-cmk", "mutated-tag", "epoch-derivation-binding", "receiver-derivation-binding", "channel-aad-binding", "originator-aad-binding"}) {
		t.Fatal("fixture case roster changed")
	}
	public, err := fixtures.signer.PublicKey()
	if err != nil || !bytes.Equal(public, fixtures.publicKey) {
		t.Fatalf("signing public key differs: %v", err)
	}
}

func TestGrantPositiveCasesLockEveryIntermediateAndD18GByte(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	for _, testCase := range fixtures.manifest.PositiveCases {
		t.Run(testCase.Name, func(t *testing.T) {
			originatorID := decodeGrantB64(t, testCase.OriginatorIDB64)
			receiverID := decodeGrantB64(t, testCase.ReceiverIDB64)
			channelID := decodeHex(t, testCase.ChannelIDHex)
			epoch := parseFixtureUint(t, testCase.KeyEpoch)
			fields, err := NewKeyGrantFields(originatorID, receiverID, channelID, epoch)
			if err != nil {
				t.Fatal(err)
			}
			cmk, err := ChannelMasterKeyFromBytes(decodeHex(t, testCase.CMKHex))
			if err != nil {
				t.Fatal(err)
			}
			receiver, err := ReceiverKeyPairFromPrivateKey(decodeHex(t, testCase.ReceiverPrivateKeyHex))
			if err != nil {
				t.Fatal(err)
			}
			public, _ := receiver.PublicKey()
			if !bytes.Equal(public, decodeHex(t, testCase.ReceiverPublicKeyHex)) {
				t.Fatal("receiver public key differs")
			}
			ephemeralPrivate, _ := grantArray32(decodeHex(t, testCase.EphemeralPrivateKeyHex))
			receiverPublic, _ := grantArray32(public)
			ephemeralPublic, err := x25519.GenerateKeypair(ephemeralPrivate)
			if err != nil || !bytes.Equal(ephemeralPublic[:], decodeHex(t, testCase.EphemeralPublicKeyHex)) {
				t.Fatalf("ephemeral public key differs: %v", err)
			}
			shared, err := x25519.X25519(ephemeralPrivate, receiverPublic)
			if err != nil || !bytes.Equal(shared[:], decodeHex(t, testCase.SharedSecretHex)) {
				t.Fatalf("shared secret differs: %v", err)
			}
			salt, _ := KeyGrantHKDFSalt(channelID, epoch)
			info, _ := KeyGrantHKDFInfo(receiverID)
			wrappingKey, err := KeyGrantWrappingKey(shared[:], channelID, epoch, receiverID)
			if err != nil || !bytes.Equal(salt, decodeGrantB64(t, testCase.HKDFSaltB64)) ||
				!bytes.Equal(info, decodeGrantB64(t, testCase.HKDFInfoB64)) ||
				!bytes.Equal(wrappingKey, decodeHex(t, testCase.WrappingKeyHex)) {
				t.Fatalf("HKDF intermediate differs: %v", err)
			}
			grant, err := SealChannelKeyWithMaterial(fields, cmk, public, fixtures.signer, ephemeralPrivate[:], decodeHex(t, testCase.WrappingNonceHex))
			if err != nil {
				t.Fatal(err)
			}
			if !bytes.Equal(grant.OriginatorID(), originatorID) || !bytes.Equal(grant.ReceiverID(), receiverID) ||
				!bytes.Equal(grant.ChannelID(), channelID) || grant.KeyEpoch() != epoch ||
				!bytes.Equal(grant.EphemeralPublicKey(), decodeHex(t, testCase.EphemeralPublicKeyHex)) ||
				!bytes.Equal(grant.WrappingNonce(), decodeHex(t, testCase.WrappingNonceHex)) ||
				!bytes.Equal(grant.WrappedCMK(), decodeHex(t, testCase.WrappedCMKHex)) ||
				!bytes.Equal(grant.OriginatorSignature(), decodeHex(t, testCase.SignatureHex)) {
				t.Fatal("sealed grant field differs")
			}
			if !bytes.Equal(KeyGrantAAD(grant), decodeGrantB64(t, testCase.GrantAADB64)) ||
				!bytes.Equal(KeyGrantSignatureInput(grant), decodeGrantB64(t, testCase.SignatureInputB64)) {
				t.Fatal("authentication intermediate differs")
			}
			record, err := GrantSerialize(grant)
			if err != nil || !bytes.Equal(record, decodeGrantB64(t, testCase.D18GB64)) {
				t.Fatalf("D18G differs: %v", err)
			}
			decoded, err := GrantDeserialize(record)
			if err != nil {
				t.Fatal(err)
			}
			roundTrip, _ := GrantSerialize(decoded)
			if !bytes.Equal(roundTrip, record) {
				t.Fatal("D18G round trip differs")
			}
			opened, err := OpenChannelKeyGrant(decoded, originatorID, receiverID, channelID, receiver, fixtures.publicKey)
			if err != nil {
				t.Fatal(err)
			}
			openedBytes, _ := opened.Bytes()
			if !bytes.Equal(openedBytes, decodeHex(t, testCase.ExpectedOpenedCMKHex)) {
				t.Fatal("opened CMK differs")
			}
			opened.Destroy()
			cmk.Destroy()
			receiver.Destroy()
		})
	}
}

func TestGrantStructuralFieldAndSealFailuresUseDeclaredCodes(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	base := decodeGrantB64(t, fixtures.manifest.PositiveCases[0].D18GB64)
	for _, testCase := range fixtures.manifest.StructuralNegativeCases {
		t.Run(testCase.Name, func(t *testing.T) {
			_, err := GrantDeserialize(decodeGrantB64(t, testCase.D18GB64))
			requireGrantCode(t, err, testCase.ExpectedError)
		})
	}
	recipe := fixtures.manifest.TruncatedPrefixRecipe
	first := parseFixtureInt(t, recipe.FirstLength)
	last := parseFixtureInt(t, recipe.LastLengthExclusive)
	if last != len(base) {
		t.Fatalf("last truncated prefix = %d, record length = %d", last, len(base))
	}
	for end := first; end < last; end++ {
		_, err := GrantDeserialize(base[:end])
		requireGrantCode(t, err, recipe.ExpectedError)
	}
	for _, oversize := range fixtures.manifest.OversizeRecipes {
		changed := clone(base)
		offset := parseFixtureInt(t, oversize.LengthOffset)
		binary.BigEndian.PutUint32(changed[offset:offset+4], parseFixtureUint32(t, oversize.DeclaredLength))
		_, err := GrantDeserialize(changed)
		requireGrantCode(t, err, oversize.ExpectedError)
	}
	for _, testCase := range fixtures.manifest.FieldNegativeCases {
		t.Run(testCase.Name, func(t *testing.T) {
			originatorID, receiverID := []byte("originator"), []byte("receiver")
			channelID := clone(fixtures.channelID)
			switch testCase.Name {
			case "empty-originator":
				originatorID = nil
			case "empty-receiver":
				receiverID = nil
			case "invalid-uuid-version":
				channelID[6] = 0x60
			case "invalid-uuid-variant":
				channelID[8] = 0x10
			case "oversized-originator":
				originatorID = make([]byte, 4097)
			case "oversized-receiver":
				receiverID = make([]byte, 4097)
			}
			_, err := NewKeyGrantFields(originatorID, receiverID, channelID, 0)
			requireGrantCode(t, err, testCase.ExpectedError)
		})
	}
	fields, _ := NewKeyGrantFields([]byte("originator"), []byte("receiver"), fixtures.channelID, 0)
	cmk, _ := ChannelMasterKeyFromBytes(bytes.Repeat([]byte{0x22}, 32))
	_, err := SealChannelKeyWithMaterial(fields, cmk, make([]byte, 32), fixtures.signer, bytes.Repeat([]byte{0x51}, 32), bytes.Repeat([]byte{0x61}, 24))
	requireGrantCode(t, err, fixtures.manifest.SealNegativeCases[0].ExpectedError)
	cmk.Destroy()
}

func TestGrantOpeningFailuresFollowNormativeValidationOrder(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	for _, testCase := range fixtures.manifest.OpeningNegativeCases {
		t.Run(testCase.Name, func(t *testing.T) {
			grant, err := GrantDeserialize(decodeGrantB64(t, testCase.D18GB64))
			if err != nil {
				t.Fatal(err)
			}
			receiver, err := ReceiverKeyPairFromPrivateKey(decodeHex(t, testCase.ReceiverPrivateKeyHex))
			if err != nil {
				t.Fatal(err)
			}
			_, err = OpenChannelKeyGrant(
				grant,
				decodeGrantB64(t, testCase.ExpectedOriginatorIDB64),
				decodeGrantB64(t, testCase.ExpectedReceiverIDB64),
				decodeHex(t, testCase.ExpectedChannelIDHex), receiver, fixtures.publicKey,
			)
			requireGrantCode(t, err, testCase.ExpectedError)
			receiver.Destroy()
		})
	}
}

func retainedGrantEpochs(state *ReceiverEpochKeys, maximum uint64) []string {
	result := []string{}
	for epoch := uint64(0); epoch <= maximum; epoch++ {
		key, err := state.Key(epoch)
		if err != nil {
			continue
		}
		key.Destroy()
		result = append(result, strconv.FormatUint(epoch, 10))
	}
	return result
}

func TestGrantReceiverTraceIsAtomicMonotonicAndAllowsSkippedEpochs(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	first := fixtures.manifest.PositiveCases[0]
	receiver, _ := ReceiverKeyPairFromPrivateKey(decodeHex(t, first.ReceiverPrivateKeyHex))
	state, err := NewReceiverEpochKeys(
		decodeGrantB64(t, first.OriginatorIDB64), decodeGrantB64(t, first.ReceiverIDB64),
		fixtures.channelID, receiver, fixtures.publicKey,
	)
	if err != nil {
		t.Fatal(err)
	}
	trace := fixtures.manifest.ReceiverStateTrace
	for _, step := range trace.Steps {
		grant, err := GrantDeserialize(decodeGrantB64(t, trace.Grants[step.Grant]))
		if err != nil {
			t.Fatal(err)
		}
		outcome, err := state.InstallGrant(grant)
		actual := string(outcome)
		if err != nil {
			var profileErr *KeyGrantProfileError
			if !errors.As(err, &profileErr) {
				t.Fatal(err)
			}
			actual = string(profileErr.Code)
		}
		if actual != step.Expected {
			t.Fatalf("%s outcome = %s, want %s", step.Name, actual, step.Expected)
		}
		latest, ok := state.LatestEpoch()
		if !ok || strconv.FormatUint(latest, 10) != step.LatestEpoch || !reflect.DeepEqual(retainedGrantEpochs(state, 3), step.RetainedEpochs) {
			t.Fatalf("%s state changed incorrectly", step.Name)
		}
	}
	missing := parseFixtureUint(t, trace.MissingEpoch)
	_, err = state.Key(missing)
	requireGrantCode(t, err, trace.MissingEpochError)
	stateDebug := fmt.Sprintf("%+v %#v", *state, *state)
	if !strings.Contains(stateDebug, "latest_epoch=3") || strings.Contains(stateDebug, first.CMKHex) {
		t.Fatal("receiver state debug formatting is incomplete or secret-bearing")
	}
	latest, _ := state.LatestEpoch()
	malformed, _ := newPortableKeyGrant(nil, nil, make([]byte, 16), latest, make([]byte, 32), make([]byte, 24), make([]byte, 48), make([]byte, 64))
	_, err = state.InstallGrant(malformed)
	requireGrantCode(t, err, "conflicting_grant")
	originalPublic, _ := receiver.PublicKey()
	statePublic, _ := state.ReceiverPublicKey()
	if !bytes.Equal(originalPublic, statePublic) {
		t.Fatal("state did not retain independent receiver key pair")
	}
	state.Destroy()
	receiver.Destroy()
}

func TestGrantRotationReproducesProspectiveRevocationFixture(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	first, second := fixtures.manifest.PositiveCases[0], fixtures.manifest.PositiveCases[1]
	receiverA, _ := ReceiverKeyPairFromPrivateKey(decodeHex(t, first.ReceiverPrivateKeyHex))
	receiverB, _ := ReceiverKeyPairFromPrivateKey(decodeHex(t, second.ReceiverPrivateKeyHex))
	stateA, _ := NewReceiverEpochKeys(decodeGrantB64(t, first.OriginatorIDB64), decodeGrantB64(t, first.ReceiverIDB64), fixtures.channelID, receiverA, fixtures.publicKey)
	stateB, _ := NewReceiverEpochKeys(decodeGrantB64(t, second.OriginatorIDB64), decodeGrantB64(t, second.ReceiverIDB64), fixtures.channelID, receiverB, fixtures.publicKey)
	grantA, _ := GrantDeserialize(decodeGrantB64(t, first.D18GB64))
	grantB, _ := GrantDeserialize(decodeGrantB64(t, second.D18GB64))
	if _, err := stateA.InstallGrant(grantA); err != nil {
		t.Fatal(err)
	}
	if _, err := stateB.InstallGrant(grantB); err != nil {
		t.Fatal(err)
	}
	rotation := fixtures.manifest.RotationCase
	newCMK, _ := ChannelMasterKeyFromBytes(decodeHex(t, rotation.NewCMKHex))
	publicB, _ := receiverB.PublicKey()
	rotationB, _ := NewRotationReceiverWithMaterial(decodeGrantB64(t, second.ReceiverIDB64), publicB, bytes.Repeat([]byte{0x71}, 32), bytes.Repeat([]byte{0x81}, 24))
	plan, err := PlanRotation(decodeGrantB64(t, first.OriginatorIDB64), fixtures.channelID, parseFixtureUint(t, rotation.CurrentEpoch), newCMK, []*RotationReceiver{rotationB}, fixtures.signer)
	if err != nil {
		t.Fatal(err)
	}
	if plan.NewEpoch() != parseFixtureUint(t, rotation.NewEpoch) {
		t.Fatalf("new epoch = %d", plan.NewEpoch())
	}
	planDebug := fmt.Sprintf("%+v %#v %+v %#v", *plan, *plan, *rotationB, *rotationB)
	if !strings.Contains(planDebug, "RotationPlan(<secret>") || !strings.Contains(planDebug, "RotationReceiver(<destroyed>") || strings.Contains(planDebug, rotation.NewCMKHex) {
		t.Fatal("rotation debug formatting is incomplete or secret-bearing")
	}
	grants := plan.Grants()
	actualGrantB64 := make([]string, len(grants))
	actualReceiverB64 := make([]string, len(grants))
	for i, grant := range grants {
		record, _ := GrantSerialize(grant)
		actualGrantB64[i] = base64.StdEncoding.EncodeToString(record)
		actualReceiverB64[i] = base64.StdEncoding.EncodeToString(grant.ReceiverID())
	}
	if !reflect.DeepEqual(actualGrantB64, rotation.NewGrantsB64) || !reflect.DeepEqual(actualReceiverB64, rotation.AuthorizedReceiverIDsB64) {
		t.Fatal("rotation grants differ")
	}
	if _, err := stateB.InstallGrant(grants[0]); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(retainedGrantEpochs(stateA, 1), rotation.ReceiverARetainsEpochs) ||
		!reflect.DeepEqual(retainedGrantEpochs(stateB, 1), rotation.ReceiverBRetainsEpochs) || rotation.ReceiverANewGrant != nil {
		t.Fatal("prospective revocation state differs")
	}
	plannedCMK, _ := plan.NewCMK()
	installedCMK, _ := stateB.Key(1)
	plannedBytes, _ := plannedCMK.Bytes()
	installedBytes, _ := installedCMK.Bytes()
	if !bytes.Equal(plannedBytes, installedBytes) {
		t.Fatal("planned and installed CMK differ")
	}
	plannedCMK.Destroy()
	installedCMK.Destroy()
	plan.Destroy()
	newCMK.Destroy()
	stateA.Destroy()
	stateB.Destroy()
	receiverA.Destroy()
	receiverB.Destroy()
}

type queuedGrantRandom struct{ chunks [][]byte }

func (r *queuedGrantRandom) Read(destination []byte) (int, error) {
	if len(r.chunks) == 0 {
		return 0, errors.New("entropy exhausted")
	}
	chunk := r.chunks[0]
	r.chunks = r.chunks[1:]
	return copy(destination, chunk), nil
}

type shortGrantRandom struct{}

func (shortGrantRandom) Read(destination []byte) (int, error) {
	return len(destination) - 1, nil
}

type failingGrantRandom struct{}

func (failingGrantRandom) Read([]byte) (int, error) { return 0, errors.New("unavailable") }

func TestGrantEntropyLifecycleImmutabilityAndRotationEdges(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	first := fixtures.manifest.PositiveCases[0]
	originatorMutable := decodeGrantB64(t, first.OriginatorIDB64)
	fields, _ := NewKeyGrantFields(
		originatorMutable,
		decodeGrantB64(t, first.ReceiverIDB64),
		fixtures.channelID,
		0,
	)
	clear(originatorMutable)
	if bytes.Equal(fields.OriginatorID(), originatorMutable) {
		t.Fatal("fields retained caller-owned bytes")
	}
	cmk, _ := ChannelMasterKeyFromBytes(decodeHex(t, first.CMKHex))
	receiverPublic := decodeHex(t, first.ReceiverPublicKeyHex)
	grant, err := SealChannelKeyWithSource(fields, cmk, receiverPublic, fixtures.signer, &queuedGrantRandom{[][]byte{decodeHex(t, first.EphemeralPrivateKeyHex), decodeHex(t, first.WrappingNonceHex)}})
	if err != nil {
		t.Fatal(err)
	}
	receiverCopy := grant.ReceiverID()
	receiverCopy[0] ^= 0xff
	record, _ := GrantSerialize(grant)
	if !bytes.Equal(record, decodeGrantB64(t, first.D18GB64)) {
		t.Fatal("grant accessor mutated immutable state")
	}
	generatedCMK, err := GenerateChannelMasterKeyWithSource(&queuedGrantRandom{[][]byte{bytes.Repeat([]byte{9}, 32)}})
	if err != nil {
		t.Fatal(err)
	}
	generatedReceiver, err := GenerateReceiverKeyPairWithSource(&queuedGrantRandom{[][]byte{bytes.Repeat([]byte{10}, 32)}})
	if err != nil {
		t.Fatal(err)
	}
	generatedSigner, err := GenerateOriginatorSigningKeyWithSource(&queuedGrantRandom{[][]byte{bytes.Repeat([]byte{11}, 32)}})
	if err != nil {
		t.Fatal(err)
	}
	debugText := fmt.Sprintf(
		"%+v %#v %+v %#v %+v %#v",
		*generatedCMK, *generatedCMK,
		*generatedReceiver, *generatedReceiver,
		*generatedSigner, *generatedSigner,
	)
	if strings.Contains(debugText, "0909") || strings.Contains(debugText, "0a0a") || strings.Contains(debugText, "0b0b") {
		t.Fatal("debug formatting exposed secret material")
	}
	generatedCMK.Destroy()
	generatedReceiver.Destroy()
	generatedSigner.Destroy()
	if _, err := generatedCMK.Bytes(); err == nil {
		t.Fatal("destroyed CMK remained usable")
	}
	if _, err := generatedReceiver.PublicKey(); err == nil {
		t.Fatal("destroyed receiver remained usable")
	}
	if _, err := generatedSigner.PublicKey(); err == nil {
		t.Fatal("destroyed signer remained usable")
	}
	if !allZero(generatedCMK.value[:]) || !allZero(generatedReceiver.privateKey[:]) || !allZero(generatedSigner.secretKey[:]) {
		t.Fatal("controlled destruction did not clear an owned buffer")
	}
	_, err = GenerateChannelMasterKeyWithSource(shortGrantRandom{})
	requireGrantCode(t, err, "randomness_unavailable")
	_, err = GenerateReceiverKeyPairWithSource(failingGrantRandom{})
	requireGrantCode(t, err, "randomness_unavailable")
	_, err = GenerateOriginatorSigningKeyWithSource(shortGrantRandom{})
	requireGrantCode(t, err, "randomness_unavailable")
	_, err = SealChannelKeyWithSource(fields, cmk, receiverPublic, fixtures.signer, shortGrantRandom{})
	requireGrantCode(t, err, "randomness_unavailable")
	_, err = GenerateRotationReceiverWithSource([]byte("receiver"), receiverPublic, shortGrantRandom{})
	requireGrantCode(t, err, "randomness_unavailable")
	validRotation, _ := NewRotationReceiverWithMaterial([]byte("receiver"), receiverPublic, bytes.Repeat([]byte{1}, 32), bytes.Repeat([]byte{2}, 24))
	_, err = PlanRotation([]byte("originator"), fixtures.channelID, math.MaxUint64, cmk, []*RotationReceiver{validRotation}, fixtures.signer)
	requireGrantCode(t, err, "epoch_exhausted")
	_, err = PlanRotation([]byte("originator"), fixtures.channelID, 0, cmk, nil, fixtures.signer)
	requireGrantCode(t, err, "invalid_field")
	left, _ := NewRotationReceiverWithMaterial([]byte("duplicate"), receiverPublic, bytes.Repeat([]byte{3}, 32), bytes.Repeat([]byte{4}, 24))
	right, _ := NewRotationReceiverWithMaterial([]byte("duplicate"), receiverPublic, bytes.Repeat([]byte{5}, 32), bytes.Repeat([]byte{6}, 24))
	_, err = PlanRotation([]byte("originator"), fixtures.channelID, 0, cmk, []*RotationReceiver{left, right}, fixtures.signer)
	requireGrantCode(t, err, "invalid_field")
	if !left.destroyed || !right.destroyed {
		t.Fatal("failed rotation retained one-shot secret material")
	}
	second, _ := NewRotationReceiverWithMaterial([]byte("b"), receiverPublic, bytes.Repeat([]byte{7}, 32), bytes.Repeat([]byte{8}, 24))
	firstReceiver, _ := NewRotationReceiverWithMaterial([]byte("a"), receiverPublic, bytes.Repeat([]byte{9}, 32), bytes.Repeat([]byte{10}, 24))
	plan, err := PlanRotation([]byte("originator"), fixtures.channelID, 4, cmk, []*RotationReceiver{second, firstReceiver}, fixtures.signer)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(plan.Grants()[0].ReceiverID(), []byte("a")) || !firstReceiver.destroyed || !second.destroyed {
		t.Fatal("rotation did not sort or consume receiver material")
	}
	plan.Destroy()
	cmk.Destroy()
}

func TestGrantConstructorsAndHighLevelEncoderFailClosed(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	_, err := ChannelMasterKeyFromBytes(make([]byte, 31))
	requireGrantCode(t, err, "invalid_field")
	_, err = ReceiverKeyPairFromPrivateKey(make([]byte, 31))
	requireGrantCode(t, err, "invalid_field")
	_, err = OriginatorSigningKeyFromSeed(make([]byte, 31))
	requireGrantCode(t, err, "invalid_field")
	structural, err := newPortableKeyGrant(nil, nil, make([]byte, 16), 0, make([]byte, 32), make([]byte, 24), make([]byte, 48), make([]byte, 64))
	if err != nil {
		t.Fatal(err)
	}
	_, err = GrantSerialize(structural)
	requireGrantCode(t, err, "invalid_field")
	_, err = newPortableKeyGrant(nil, nil, make([]byte, 15), 0, make([]byte, 32), make([]byte, 24), make([]byte, 48), make([]byte, 64))
	requireGrantCode(t, err, "invalid_field")
	_, err = KeyGrantHKDFInfo(make([]byte, 4097))
	requireGrantCode(t, err, "length_limit_exceeded")
	_, err = KeyGrantWrappingKey(make([]byte, 31), fixtures.channelID, 0, []byte("receiver"))
	requireGrantCode(t, err, "invalid_field")
}

func allZero(value []byte) bool {
	for _, item := range value {
		if item != 0 {
			return false
		}
	}
	return true
}
