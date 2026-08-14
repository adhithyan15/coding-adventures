package channelcrypto

import (
	"bytes"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"strconv"
	"testing"

	ed25519 "github.com/example/coding-adventures/code/packages/go/ed25519"
)

type fixtureManifest struct {
	FixtureFormat     string                  `json:"fixture_format"`
	GeneratorBlobSHA1 string                  `json:"generator_blob_sha1"`
	Warning           string                  `json:"warning"`
	Keys              fixtureKeys             `json:"keys"`
	PositiveCases     []positiveFixture       `json:"positive_cases"`
	BinaryNegative    []binaryNegativeFixture `json:"binary_negative_cases"`
	JSONNegative      []jsonNegativeFixture   `json:"json_negative_cases"`
	OversizeRecipes   []oversizeRecipe        `json:"oversize_recipes"`
}

type fixtureKeys struct {
	SigningSeedHex string `json:"originator_signing_seed_hex"`
	PublicKeyHex   string `json:"originator_public_key_hex"`
	EpochKeys      []struct {
		Epoch  string `json:"key_epoch"`
		KeyHex string `json:"key_hex"`
	} `json:"channel_master_keys"`
}

type positiveFixture struct {
	Name                   string `json:"name"`
	PlaintextB64           string `json:"plaintext_b64"`
	AuthenticatedHeaderB64 string `json:"authenticated_header_b64"`
	D18MB64                string `json:"d18m_b64"`
	CanonicalJSONB64       string `json:"canonical_json_b64"`
}

type binaryNegativeFixture struct {
	Name          string `json:"name"`
	Phase         string `json:"phase"`
	D18MB64       string `json:"d18m_b64"`
	ExpectedError string `json:"expected_error"`
}

type jsonNegativeFixture struct {
	Name          string `json:"name"`
	JSONB64       string `json:"json_b64"`
	ExpectedError string `json:"expected_error"`
}

type oversizeRecipe struct {
	Field          string `json:"field"`
	DeclaredLength string `json:"declared_length"`
	ExpectedError  string `json:"expected_error"`
}

type fixtureContext struct {
	manifest  fixtureManifest
	publicKey []byte
	secretKey []byte
	epochKeys map[uint64][]byte
}

func loadFixtures(t *testing.T) fixtureContext {
	t.Helper()
	path := filepath.Join("..", "..", "..", "fixtures", "chief-of-staff-message", "v1", "manifest.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var manifest fixtureManifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		t.Fatal(err)
	}
	seedBytes, _ := hex.DecodeString(manifest.Keys.SigningSeedHex)
	var seed [32]byte
	copy(seed[:], seedBytes)
	public, secret := ed25519.GenerateKeypair(seed)
	epochKeys := make(map[uint64][]byte)
	for _, item := range manifest.Keys.EpochKeys {
		epoch, _ := strconv.ParseUint(item.Epoch, 10, 64)
		key, _ := hex.DecodeString(item.KeyHex)
		epochKeys[epoch] = key
	}
	return fixtureContext{manifest, clone(public[:]), clone(secret[:]), epochKeys}
}

func decodeB64(t *testing.T, value string) []byte {
	t.Helper()
	decoded, err := base64.StdEncoding.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return decoded
}

func requireCode(t *testing.T, err error, expected string) {
	t.Helper()
	if !ErrorIs(err, ErrorCode(expected)) {
		t.Fatalf("error = %v, want code %q", err, expected)
	}
}

func TestFixtureProvenanceAndPublicMaterialAreLocked(t *testing.T) {
	fixtures := loadFixtures(t)
	if fixtures.manifest.FixtureFormat != "D18F-message-fixtures-v1" || len(fixtures.manifest.GeneratorBlobSHA1) != 40 ||
		!bytes.Contains([]byte(fixtures.manifest.Warning), []byte("test-only")) {
		t.Fatal("fixture provenance metadata changed")
	}
	if len(fixtures.manifest.PositiveCases) != 8 || len(fixtures.manifest.BinaryNegative) != 20 || len(fixtures.manifest.JSONNegative) != 11 {
		t.Fatal("fixture case counts changed")
	}
	expected, _ := hex.DecodeString(fixtures.manifest.Keys.PublicKeyHex)
	if !bytes.Equal(fixtures.publicKey, expected) {
		t.Fatal("generated public key differs from fixture")
	}
}

func TestPositiveFixturesAreReproducedByteIdentically(t *testing.T) {
	fixtures := loadFixtures(t)
	for _, testCase := range fixtures.manifest.PositiveCases {
		t.Run(testCase.Name, func(t *testing.T) {
			binaryRecord := decodeB64(t, testCase.D18MB64)
			plaintext := decodeB64(t, testCase.PlaintextB64)
			message, err := MessageDeserialize(binaryRecord)
			if err != nil {
				t.Fatal(err)
			}
			key := fixtures.epochKeys[message.KeyEpoch()]
			serialized, err := MessageSerialize(message)
			if err != nil || !bytes.Equal(serialized, binaryRecord) {
				t.Fatalf("binary round trip failed: %v", err)
			}
			if !bytes.Equal(MessageAuthenticatedHeader(message), decodeB64(t, testCase.AuthenticatedHeaderB64)) {
				t.Fatal("authenticated header differs")
			}
			verified, err := MessageVerifyWithKeyResolver(message, fixtures.publicKey, func(epoch uint64) []byte { return fixtures.epochKeys[epoch] })
			if err != nil || !bytes.Equal(verified, plaintext) {
				t.Fatalf("resolver verify failed: %v", err)
			}
			verified, err = MessageVerify(message, fixtures.publicKey, key)
			if err != nil || !bytes.Equal(verified, plaintext) {
				t.Fatalf("direct verify failed: %v", err)
			}
			canonical := decodeB64(t, testCase.CanonicalJSONB64)
			encoded, err := MessageToJSON(message)
			if err != nil || !bytes.Equal(encoded, canonical) {
				t.Fatalf("canonical JSON differs: %v\n got %s\nwant %s", err, encoded, canonical)
			}
			fromJSON, err := MessageFromJSON(canonical)
			if err != nil {
				t.Fatal(err)
			}
			fromJSONBinary, _ := MessageSerialize(fromJSON)
			if !bytes.Equal(fromJSONBinary, binaryRecord) {
				t.Fatal("JSON round trip differs")
			}
			recreated, err := MessageCreate(message.Fields(), plaintext, fixtures.secretKey, key)
			if err != nil {
				t.Fatal(err)
			}
			recreatedBinary, _ := MessageSerialize(recreated)
			if !bytes.Equal(recreatedBinary, binaryRecord) {
				t.Fatal("recreated bytes differ")
			}
		})
	}
}

func TestBinaryMutationsMapToStableErrors(t *testing.T) {
	fixtures := loadFixtures(t)
	for _, testCase := range fixtures.manifest.BinaryNegative {
		t.Run(testCase.Name, func(t *testing.T) {
			message, err := MessageDeserialize(decodeB64(t, testCase.D18MB64))
			if err == nil && testCase.Phase == "verify" {
				_, err = MessageVerifyWithKeyResolver(message, fixtures.publicKey, func(epoch uint64) []byte { return fixtures.epochKeys[epoch] })
			}
			requireCode(t, err, testCase.ExpectedError)
		})
	}
}

func TestJSONMutationsMapToStableErrors(t *testing.T) {
	fixtures := loadFixtures(t)
	for _, testCase := range fixtures.manifest.JSONNegative {
		t.Run(testCase.Name, func(t *testing.T) {
			_, err := MessageFromJSON(decodeB64(t, testCase.JSONB64))
			requireCode(t, err, testCase.ExpectedError)
		})
	}
}

func TestJSONFieldOrderIsIrrelevantAndOutputCanonical(t *testing.T) {
	fixtures := loadFixtures(t)
	canonical := decodeB64(t, fixtures.manifest.PositiveCases[2].CanonicalJSONB64)
	values, err := decodeJSONObject(canonical)
	if err != nil {
		t.Fatal(err)
	}
	var reordered bytes.Buffer
	reordered.WriteByte('{')
	for i := len(jsonFields) - 1; i >= 0; i-- {
		if i != len(jsonFields)-1 {
			reordered.WriteByte(',')
		}
		reordered.WriteString(strconv.Quote(jsonFields[i]))
		reordered.WriteByte(':')
		reordered.Write(values[jsonFields[i]])
	}
	reordered.WriteByte('}')
	message, err := MessageFromJSON(reordered.Bytes())
	if err != nil {
		t.Fatal(err)
	}
	encoded, err := MessageToJSON(message)
	if err != nil || !bytes.Equal(encoded, canonical) {
		t.Fatalf("canonicalization failed: %v", err)
	}
}

func TestJSONRejectsUnpairedSurrogates(t *testing.T) {
	fixtures := loadFixtures(t)
	canonical := decodeB64(t, fixtures.manifest.PositiveCases[0].CanonicalJSONB64)
	malformed := bytes.Replace(canonical, []byte(`"content_type":"application/octet-stream"`), []byte(`"content_type":"\ud800"`), 1)
	_, err := MessageFromJSON(malformed)
	requireCode(t, err, string(ErrInvalidField))
}

func TestCompactOversizeRecipesAreEnforced(t *testing.T) {
	fixtures := loadFixtures(t)
	baseline := decodeB64(t, fixtures.manifest.PositiveCases[0].D18MB64)
	for _, recipe := range fixtures.manifest.OversizeRecipes {
		t.Run(recipe.Field, func(t *testing.T) {
			if recipe.Field == "json-input" {
				_, err := MessageFromJSON(make([]byte, MaxMessageJSONBytes+1))
				requireCode(t, err, recipe.ExpectedError)
				return
			}
			changed := clone(baseline)
			switch recipe.Field {
			case "originator-id", "content-type":
				length, err := strconv.ParseUint(recipe.DeclaredLength, 10, 32)
				if err != nil || length > uint64(^uint32(0)) {
					t.Fatalf("invalid uint32 fixture length %q", recipe.DeclaredLength)
				}
				offset := 29
				if recipe.Field == "content-type" {
					offset = 83
				}
				binaryPut32(changed[offset:offset+4], uint32(length))
			case "ciphertext":
				length, err := strconv.ParseUint(recipe.DeclaredLength, 10, 64)
				if err != nil {
					t.Fatalf("invalid uint64 fixture length %q", recipe.DeclaredLength)
				}
				binaryPut64(changed[143:151], length)
			}
			_, err := MessageDeserialize(changed)
			requireCode(t, err, recipe.ExpectedError)
		})
	}
}

func binaryPut32(target []byte, value uint32) { copy(target, u32be(value)) }
func binaryPut64(target []byte, value uint64) { copy(target, u64be(value)) }

func TestMessagesCopyMutableInputsAndAccessorResults(t *testing.T) {
	fixtures := loadFixtures(t)
	source, err := MessageDeserialize(decodeB64(t, fixtures.manifest.PositiveCases[1].D18MB64))
	if err != nil {
		t.Fatal(err)
	}
	messageID, originatorID, channelID := source.MessageID(), source.OriginatorID(), source.ChannelID()
	fields, err := NewMessageFields(messageID, source.TimestampNS(), originatorID, channelID, source.Sequence(), source.KeyEpoch(), source.ContentType())
	if err != nil {
		t.Fatal(err)
	}
	message, err := MessageCreate(fields, decodeB64(t, fixtures.manifest.PositiveCases[1].PlaintextB64), fixtures.secretKey, fixtures.epochKeys[source.KeyEpoch()])
	if err != nil {
		t.Fatal(err)
	}
	original, _ := MessageSerialize(message)
	for _, mutable := range [][]byte{messageID, originatorID, channelID, message.MessageID(), message.OriginatorID(), message.ChannelID(), message.PlaintextHash(), message.Ciphertext(), message.AuthenticationTag(), message.OriginatorSignature()} {
		for i := range mutable {
			mutable[i] = 0
		}
	}
	after, _ := MessageSerialize(message)
	if !bytes.Equal(original, after) {
		t.Fatal("caller mutation changed immutable message")
	}
}

type fixedUUIDSource struct{ value []byte }

func (s fixedUUIDSource) NextUUIDv7() ([]byte, error) { return clone(s.value), nil }

type fixedClock struct{ value uint64 }

func (c fixedClock) NowNanoseconds() uint64 { return c.value }

func TestCreationUsesInjectedUUIDAndMonotonicClockSources(t *testing.T) {
	fixtures := loadFixtures(t)
	source, _ := MessageDeserialize(decodeB64(t, fixtures.manifest.PositiveCases[0].D18MB64))
	fields, err := NewSourcedMessageFields(source.OriginatorID(), source.ChannelID(), 123, source.KeyEpoch(), source.ContentType())
	if err != nil {
		t.Fatal(err)
	}
	key := fixtures.epochKeys[source.KeyEpoch()]
	message, err := MessageCreateWithSources(fields, []byte{1, 2, 3}, fixtures.secretKey, key, fixedUUIDSource{source.MessageID()}, fixedClock{456})
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(message.MessageID(), source.MessageID()) || message.TimestampNS() != 456 {
		t.Fatal("injected sources not used")
	}
	plaintext, err := MessageVerify(message, fixtures.publicKey, key)
	if err != nil || !bytes.Equal(plaintext, []byte{1, 2, 3}) {
		t.Fatalf("verification failed: %v", err)
	}
}

func TestUUIDv7GeneratorOrders1000ValuesInOneMillisecond(t *testing.T) {
	var generator MonotonicUUIDv7Generator
	var previous []byte
	for i := 0; i < 1000; i++ {
		current, err := generator.Next(1_725_000_000_000, bytes.Repeat([]byte{0x55}, 10))
		if err != nil {
			t.Fatal(err)
		}
		if current[6]>>4 != 7 || current[8]&0xc0 != 0x80 {
			t.Fatal("UUID bits invalid")
		}
		if previous != nil && bytes.Compare(previous, current) >= 0 {
			t.Fatal("UUIDs are not strictly ordered")
		}
		previous = current
	}
}
