package chacha20poly1305

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"os"
	"testing"
)

type sharedHChaCha20Case struct {
	ID        string `json:"id"`
	Key       string `json:"key_hex"`
	Nonce     string `json:"nonce_hex"`
	SubkeyHex string `json:"subkey_hex"`
}

type sharedXChaCha20Case struct {
	ID        string `json:"id"`
	Counter   uint32 `json:"counter"`
	Key       string `json:"key_hex"`
	Nonce     string `json:"nonce_hex"`
	InputHex  string `json:"input_hex"`
	OutputHex string `json:"output_hex"`
}

type sharedAEADCase struct {
	ID         string `json:"id"`
	Key        string `json:"key_hex"`
	Nonce      string `json:"nonce_hex"`
	AAD        string `json:"aad_hex"`
	Plaintext  string `json:"plaintext_hex"`
	Ciphertext string `json:"ciphertext_hex"`
	Tag        string `json:"tag_hex"`
}

type sharedMutation struct {
	SourceCase  string `json:"source_case"`
	Target      string `json:"target"`
	ByteIndices []int  `json:"byte_indices"`
	XORHex      string `json:"xor_hex"`
}

type sharedSE04Fixture struct {
	SchemaVersion         int                   `json:"schema_version"`
	Profile               string                `json:"profile"`
	AuthenticationFailure string                `json:"authentication_failure"`
	HChaCha20Cases        []sharedHChaCha20Case `json:"hchacha20_cases"`
	XChaCha20Cases        []sharedXChaCha20Case `json:"xchacha20_cases"`
	AEADCases             []sharedAEADCase      `json:"aead_cases"`
	Mutations             []sharedMutation      `json:"mutations"`
}

func sharedFixture(t *testing.T) sharedSE04Fixture {
	t.Helper()
	data, err := os.ReadFile("../../../specs/fixtures/se04-xchacha20-poly1305-v1/cases.json")
	if err != nil {
		t.Fatal(err)
	}
	var fixture sharedSE04Fixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatal(err)
	}
	return fixture
}

func sharedHex(t *testing.T, value string) []byte {
	t.Helper()
	decoded, err := hex.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return decoded
}

func TestSE04SharedFixture(t *testing.T) {
	fixture := sharedFixture(t)
	if fixture.SchemaVersion != 1 || fixture.Profile != "se04-xchacha20-poly1305-v1" {
		t.Fatalf("unexpected fixture metadata: %+v", fixture)
	}
	if fixture.AuthenticationFailure != "authentication_failed" {
		t.Fatalf("unexpected authentication failure: %s", fixture.AuthenticationFailure)
	}
	if len(fixture.HChaCha20Cases) != 1 || len(fixture.XChaCha20Cases) != 2 ||
		len(fixture.AEADCases) != 3 || len(fixture.Mutations) != 5 {
		t.Fatal("closed fixture case counts changed")
	}

	for _, tc := range fixture.HChaCha20Cases {
		t.Run("hchacha20/"+tc.ID, func(t *testing.T) {
			subkey, err := HChaCha20Subkey(sharedHex(t, tc.Key), sharedHex(t, tc.Nonce))
			if err != nil {
				t.Fatal(err)
			}
			if !bytes.Equal(subkey, sharedHex(t, tc.SubkeyHex)) {
				t.Fatalf("subkey mismatch: %x", subkey)
			}
		})
	}

	for _, tc := range fixture.XChaCha20Cases {
		t.Run("xchacha20/"+tc.ID, func(t *testing.T) {
			input := sharedHex(t, tc.InputHex)
			key := sharedHex(t, tc.Key)
			nonce := sharedHex(t, tc.Nonce)
			output, err := XChaCha20Encrypt(input, key, nonce, tc.Counter)
			if err != nil {
				t.Fatal(err)
			}
			if !bytes.Equal(output, sharedHex(t, tc.OutputHex)) {
				t.Fatalf("output mismatch: %x", output)
			}
			recovered, err := XChaCha20Encrypt(output, key, nonce, tc.Counter)
			if err != nil || !bytes.Equal(recovered, input) {
				t.Fatalf("raw round trip failed: %v", err)
			}
		})
	}

	aeadByID := make(map[string]sharedAEADCase, len(fixture.AEADCases))
	for _, tc := range fixture.AEADCases {
		aeadByID[tc.ID] = tc
		t.Run("aead/"+tc.ID, func(t *testing.T) {
			key := sharedHex(t, tc.Key)
			nonce := sharedHex(t, tc.Nonce)
			aad := sharedHex(t, tc.AAD)
			plaintext := sharedHex(t, tc.Plaintext)
			expectedCiphertext := sharedHex(t, tc.Ciphertext)
			expectedTag := sharedHex(t, tc.Tag)
			ciphertext, tag, err := XChaCha20Poly1305AEADEncrypt(plaintext, key, nonce, aad)
			if err != nil {
				t.Fatal(err)
			}
			if !bytes.Equal(ciphertext, expectedCiphertext) || !bytes.Equal(tag, expectedTag) {
				t.Fatalf("AEAD output mismatch: ciphertext=%x tag=%x", ciphertext, tag)
			}
			recovered, err := XChaCha20Poly1305AEADDecrypt(expectedCiphertext, key, nonce, aad, expectedTag)
			if err != nil || !bytes.Equal(recovered, plaintext) {
				t.Fatalf("AEAD decrypt failed: %v", err)
			}
		})
	}

	for _, mutation := range fixture.Mutations {
		tc, ok := aeadByID[mutation.SourceCase]
		if !ok {
			t.Fatalf("unknown mutation source %s", mutation.SourceCase)
		}
		xorBytes := sharedHex(t, mutation.XORHex)
		for _, byteIndex := range mutation.ByteIndices {
			t.Run("reject/"+mutation.Target+"/"+mutation.SourceCase, func(t *testing.T) {
				values := map[string][]byte{
					"ciphertext": sharedHex(t, tc.Ciphertext),
					"key":        sharedHex(t, tc.Key),
					"nonce":      sharedHex(t, tc.Nonce),
					"aad":        sharedHex(t, tc.AAD),
					"tag":        sharedHex(t, tc.Tag),
				}
				values[mutation.Target][byteIndex] ^= xorBytes[0]
				plaintext, err := XChaCha20Poly1305AEADDecrypt(
					values["ciphertext"], values["key"], values["nonce"], values["aad"], values["tag"],
				)
				if err != ErrAuthFailed || plaintext != nil {
					t.Fatalf("expected only ErrAuthFailed with no plaintext, got %x, %v", plaintext, err)
				}
			})
		}
	}
}
