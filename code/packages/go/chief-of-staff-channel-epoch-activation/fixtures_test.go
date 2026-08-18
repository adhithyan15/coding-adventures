package epochactivation

import (
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
	channelstore "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-store"
)

// These tests consume the canonical Rust-authored D18T manifest directly. They
// never regenerate expected bytes locally and never shell out to another
// language -- that is the whole point of a shared fixture. If Go disagrees with
// Rust about a single octet, these fail.

type d18tManifest struct {
	FixtureFormat           string            `json:"fixture_format"`
	Spec                    string            `json:"spec"`
	GeneratorBlobSHA1       string            `json:"generator_blob_sha1"`
	Warning                 string            `json:"warning"`
	Constants               map[string]string `json:"constants"`
	TestOnlySecrets         map[string]string `json:"test_only_secrets"`
	StateMigrations         []stateMigration  `json:"state_migrations"`
	ActivationCase          activationCase    `json:"activation_case"`
	CrashReplayTraces       []namedTrace      `json:"crash_replay_traces"`
	RaceTraces              []namedTrace      `json:"race_traces"`
	StableErrorCodes        []string          `json:"stable_error_codes"`
	NegativeScenarios       []namedTrace      `json:"negative_scenarios"`
	SecretErasureCapability string            `json:"secret_erasure_capability"`
}

type stateMigration struct {
	Name         string `json:"name"`
	D18SV1B64    string `json:"d18s_v1_b64"`
	D18SV2B64    string `json:"d18s_v2_b64"`
	ActiveEpoch  string `json:"active_epoch"`
	NextSequence string `json:"next_sequence"`
}

type activationCase struct {
	Name              string   `json:"name"`
	BaseEpoch         string   `json:"base_epoch"`
	NewEpoch          string   `json:"new_epoch"`
	PlanRecordKey     string   `json:"plan_record_key"`
	PlanContentType   string   `json:"plan_content_type"`
	PlanB64           string   `json:"plan_b64"`
	GrantB64          []string `json:"grant_b64"`
	ReceiverARetains  []string `json:"receiver_a_retains_epochs"`
	ReceiverBRetains  []string `json:"receiver_b_retains_epochs"`
	ReceiverANewGrant *string  `json:"receiver_a_new_grant"`
}

type namedTrace struct {
	Name string `json:"name"`
}

const fixtureChannelIDHex = "018f47a09b6c7def923456789abcdef0"

func manifestPath() string {
	return filepath.Join("..", "..", "..", "fixtures", "chief-of-staff-channel-epoch-activation", "v1", "manifest.json")
}

func loadManifest(t *testing.T) d18tManifest {
	t.Helper()
	raw, err := os.ReadFile(manifestPath())
	if err != nil {
		t.Fatal(err)
	}
	var manifest d18tManifest
	if err := json.Unmarshal(raw, &manifest); err != nil {
		t.Fatal(err)
	}
	return manifest
}

func decodeB64(t *testing.T, value string) []byte {
	t.Helper()
	decoded, err := base64.StdEncoding.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return decoded
}

func decodeHexString(t *testing.T, value string) []byte {
	t.Helper()
	decoded, err := hex.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return decoded
}

func parseUint(t *testing.T, value string) uint64 {
	t.Helper()
	parsed, err := strconv.ParseUint(value, 10, 64)
	if err != nil {
		t.Fatal(err)
	}
	return parsed
}

func fixtureChannelID(t *testing.T) []byte { return decodeHexString(t, fixtureChannelIDHex) }

func TestManifestContractRosterAndSecretBoundary(t *testing.T) {
	manifest := loadManifest(t)
	if manifest.FixtureFormat != "D18T-durable-epoch-activation-fixtures-v1" {
		t.Fatalf("unexpected fixture format %q", manifest.FixtureFormat)
	}
	if manifest.Spec != "code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md" {
		t.Fatalf("unexpected spec path %q", manifest.Spec)
	}
	if !strings.Contains(manifest.Warning, "Never log") {
		t.Fatal("manifest must carry its secret-handling warning")
	}
	expectedConstants := map[string]string{
		"state_magic_ascii":  "D18S",
		"state_version":      "2",
		"plan_magic_ascii":   "D18T",
		"plan_version":       "1",
		"state_content_type": EpochStateContentType,
		"plan_content_type":  ActivationPlanContentType,
		"max_cas_attempts":   strconv.Itoa(MaxEpochCASAttempts),
	}
	if len(manifest.Constants) != len(expectedConstants) {
		t.Fatalf("constant roster drifted: %v", manifest.Constants)
	}
	for key, want := range expectedConstants {
		if manifest.Constants[key] != want {
			t.Fatalf("constant %q = %q, want %q", key, manifest.Constants[key], want)
		}
	}

	// The error roster is closed and ordered. A gate that only checked
	// membership would not notice a reordering, and consumers in six languages
	// index this list.
	if len(manifest.StableErrorCodes) != len(EpochActivationErrorCodes) {
		t.Fatalf("error roster length %d, want %d", len(manifest.StableErrorCodes), len(EpochActivationErrorCodes))
	}
	for index, code := range manifest.StableErrorCodes {
		if ErrorCode(code) != EpochActivationErrorCodes[index] {
			t.Fatalf("error code %d = %q, want %q", index, code, EpochActivationErrorCodes[index])
		}
	}

	expectCrash := []string{"after-custody-selection", "after-plan-write", "after-first-grant", "after-all-grants", "after-activation-cas"}
	if got := traceNames(manifest.CrashReplayTraces); !equalStrings(got, expectCrash) {
		t.Fatalf("crash traces %v, want %v", got, expectCrash)
	}
	if len(manifest.RaceTraces) != 4 {
		t.Fatalf("expected 4 race traces, got %d", len(manifest.RaceTraces))
	}
	if len(manifest.NegativeScenarios) != 6 {
		t.Fatalf("expected 6 negative scenarios, got %d", len(manifest.NegativeScenarios))
	}

	// Rust guarantees erasure; Go honestly cannot. The fixture records Rust's
	// claim, and Go must report its own rather than echo the manifest.
	if manifest.SecretErasureCapability != "guaranteed" {
		t.Fatalf("manifest capability %q", manifest.SecretErasureCapability)
	}
	if got := SecretErasureCapability(); got != "best_effort" {
		t.Fatalf("Go capability %q, want best_effort", got)
	}

	// Every labelled test-only secret must appear exactly once in the whole
	// manifest -- a second occurrence would mean a secret leaked into a
	// summary, a public record, or an expected-error string.
	raw, err := os.ReadFile(manifestPath())
	if err != nil {
		t.Fatal(err)
	}
	for name, secret := range manifest.TestOnlySecrets {
		if count := strings.Count(string(raw), secret); count != 1 {
			t.Fatalf("secret %q appears %d times in the manifest, want exactly 1", name, count)
		}
	}
}

func TestExactV1ToV2StateMigrations(t *testing.T) {
	manifest := loadManifest(t)
	channelID := fixtureChannelID(t)
	if got := traceNamesOfMigrations(manifest.StateMigrations); !equalStrings(got, []string{"no-pending", "pending-d18h"}) {
		t.Fatalf("migration names %v", got)
	}
	for _, vector := range manifest.StateMigrations {
		t.Run(vector.Name, func(t *testing.T) {
			v1, err := channelstore.ChannelStateDeserialize(decodeB64(t, vector.D18SV1B64), channelID)
			if err != nil {
				t.Fatal(err)
			}
			expected := decodeB64(t, vector.D18SV2B64)
			v2, err := EpochStateDeserialize(expected, channelID)
			if err != nil {
				t.Fatal(err)
			}
			if v2.ActiveEpoch() != parseUint(t, vector.ActiveEpoch) {
				t.Fatalf("active epoch %d", v2.ActiveEpoch())
			}
			if v2.NextSequence() != parseUint(t, vector.NextSequence) || v2.NextSequence() != v1.NextSequence {
				t.Fatalf("next sequence %d vs v1 %d", v2.NextSequence(), v1.NextSequence)
			}
			// Migration preserves the pending reservation exactly; it never
			// clears a publish that was already in flight.
			switch {
			case v1.PendingHeader == nil && v2.PendingHeader() != nil:
				t.Fatal("migration invented a pending header")
			case v1.PendingHeader != nil && v2.PendingHeader() == nil:
				t.Fatal("migration dropped a pending header")
			case v1.PendingHeader != nil && !v1.PendingHeader.Equal(*v2.PendingHeader()):
				t.Fatal("migration altered the pending header")
			}
			reencoded, err := EpochStateSerialize(v2)
			if err != nil {
				t.Fatal(err)
			}
			if !bytesEqual(reencoded, expected) {
				t.Fatal("re-encoding the canonical v2 state did not reproduce it byte for byte")
			}
		})
	}
}

func TestConsumesAndReencodesCanonicalActivationPlan(t *testing.T) {
	manifest := loadManifest(t)
	channelID := fixtureChannelID(t)
	activation := manifest.ActivationCase
	expected := decodeB64(t, activation.PlanB64)

	plan, err := ActivationPlanDeserialize(expected)
	if err != nil {
		t.Fatal(err)
	}
	if !bytesEqual(plan.ChannelID(), channelID) {
		t.Fatal("plan channel mismatch")
	}
	if plan.BaseEpoch() != 0 || plan.NewEpoch() != 1 || len(plan.Receivers()) != 1 {
		t.Fatalf("plan shape (%d, %d, %d)", plan.BaseEpoch(), plan.NewEpoch(), len(plan.Receivers()))
	}
	reencoded, err := ActivationPlanSerialize(plan)
	if err != nil {
		t.Fatal(err)
	}
	if !bytesEqual(reencoded, expected) {
		t.Fatal("re-encoding the canonical plan did not reproduce it byte for byte")
	}
	key, err := ActivationPlanRecordKey(channelID, 1)
	if err != nil {
		t.Fatal(err)
	}
	if key != activation.PlanRecordKey {
		t.Fatalf("record key %q, want %q", key, activation.PlanRecordKey)
	}
	if activation.PlanContentType != ActivationPlanContentType {
		t.Fatalf("content type %q", activation.PlanContentType)
	}

	// Prospective revocation, stated as data: A is rotated out at epoch 1, so A
	// gets no new grant and keeps only epoch 0, while B keeps both.
	if len(activation.GrantB64) != 1 {
		t.Fatalf("expected exactly one grant, got %d", len(activation.GrantB64))
	}
	if activation.ReceiverANewGrant != nil {
		t.Fatal("revoked receiver A must receive no epoch-1 grant")
	}
	if !equalStrings(activation.ReceiverARetains, []string{"0"}) {
		t.Fatalf("A retains %v, want [0]", activation.ReceiverARetains)
	}
	if !equalStrings(activation.ReceiverBRetains, []string{"0", "1"}) {
		t.Fatalf("B retains %v, want [0 1]", activation.ReceiverBRetains)
	}
}

// TestReproducesRustAuthoredPlanAndGrantBytes is the strongest fixture test in
// the file. It rebuilds the candidate from the labelled test-only secrets using
// Go's own D18Q and D18T code, and requires the result to equal the bytes Rust
// authored -- plan and grant alike.
func TestReproducesRustAuthoredPlanAndGrantBytes(t *testing.T) {
	manifest := loadManifest(t)
	channelID := fixtureChannelID(t)
	secrets := manifest.TestOnlySecrets

	signer, err := channelcrypto.OriginatorSigningKeyFromSeed(decodeHexString(t, secrets["originator_signing_seed_hex"]))
	if err != nil {
		t.Fatal(err)
	}
	defer signer.Destroy()
	signerPublic, err := signer.PublicKey()
	if err != nil {
		t.Fatal(err)
	}
	receiverAKey, err := channelcrypto.ReceiverKeyPairFromPrivateKey(decodeHexString(t, secrets["receiver_a_private_key_hex"]))
	if err != nil {
		t.Fatal(err)
	}
	defer receiverAKey.Destroy()
	receiverBKey, err := channelcrypto.ReceiverKeyPairFromPrivateKey(decodeHexString(t, secrets["receiver_b_private_key_hex"]))
	if err != nil {
		t.Fatal(err)
	}
	defer receiverBKey.Destroy()
	receiverAPublic, err := receiverAKey.PublicKey()
	if err != nil {
		t.Fatal(err)
	}
	receiverBPublic, err := receiverBKey.PublicKey()
	if err != nil {
		t.Fatal(err)
	}

	receiverA, err := channelstore.NewReceiverIdentity([]byte("receiver-a"), receiverAPublic)
	if err != nil {
		t.Fatal(err)
	}
	receiverB, err := channelstore.NewReceiverIdentity([]byte("receiver-b"), receiverBPublic)
	if err != nil {
		t.Fatal(err)
	}
	originator, err := channelstore.NewOriginatorIdentity([]byte("originator"), signerPublic)
	if err != nil {
		t.Fatal(err)
	}
	definition, err := channelstore.NewChannelDefinition(
		channelID, originator,
		[]channelstore.ReceiverIdentity{receiverA, receiverB},
		1_725_000_000_000_000_000, 0, channelstore.LifecycleActive,
	)
	if err != nil {
		t.Fatal(err)
	}

	nextCMK, err := channelcrypto.ChannelMasterKeyFromBytes(decodeHexString(t, secrets["next_cmk_hex"]))
	if err != nil {
		t.Fatal(err)
	}
	rotationReceiver, err := channelcrypto.NewRotationReceiverWithMaterial(
		receiverB.AgentID(), receiverBPublic,
		decodeHexString(t, secrets["ephemeral_private_key_hex"]),
		decodeHexString(t, secrets["wrapping_nonce_hex"]),
	)
	if err != nil {
		t.Fatal(err)
	}
	rotation, err := channelcrypto.PlanRotation(
		[]byte("originator"), channelID, 0, nextCMK,
		[]*channelcrypto.RotationReceiver{rotationReceiver}, signer,
	)
	if err != nil {
		t.Fatal(err)
	}

	prepared, err := PrepareRotationCandidate(definition, 0, []channelstore.ReceiverIdentity{receiverB}, rotation)
	if err != nil {
		t.Fatal(err)
	}
	defer prepared.Destroy()

	public := prepared.PublicPreparation()
	if !bytesEqual(public.PlanBytes(), decodeB64(t, manifest.ActivationCase.PlanB64)) {
		t.Fatal("Go produced different D18T plan bytes than the canonical Rust manifest")
	}
	grants := public.Grants()
	if len(grants) != len(manifest.ActivationCase.GrantB64) {
		t.Fatalf("grant count %d, want %d", len(grants), len(manifest.ActivationCase.GrantB64))
	}
	for index, encoded := range manifest.ActivationCase.GrantB64 {
		if !bytesEqual(grants[index], decodeB64(t, encoded)) {
			t.Fatalf("Go produced different D18G bytes than the canonical manifest at grant %d", index)
		}
	}
	if prepared.String() != "PreparedEpoch([REDACTED])" {
		t.Fatalf("PreparedEpoch must redact, got %q", prepared.String())
	}
}

func TestRejectsMalformedStateRecords(t *testing.T) {
	manifest := loadManifest(t)
	channelID := fixtureChannelID(t)
	canonical := decodeB64(t, manifest.StateMigrations[0].D18SV2B64)

	for _, testCase := range []struct {
		name   string
		mutate func([]byte) []byte
	}{
		{"truncated", func(b []byte) []byte { return b[:len(b)-1] }},
		{"trailing-byte", func(b []byte) []byte { return append(b, 0x00) }},
		{"wrong-version", func(b []byte) []byte { b[4] = 3; return b }},
		{"unknown-pending-flag", func(b []byte) []byte { b[len(b)-1] = 2; return b }},
		{"wrong-magic", func(b []byte) []byte { b[0] = 'X'; return b }},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			mutated := testCase.mutate(append([]byte(nil), canonical...))
			_, err := EpochStateDeserialize(mutated, channelID)
			if !IsCode(err, ErrCorruptRecord) {
				t.Fatalf("expected corrupt_record, got %v", err)
			}
		})
	}
}

func TestRejectsNonCanonicalPlans(t *testing.T) {
	manifest := loadManifest(t)
	channelID := fixtureChannelID(t)
	canonical := decodeB64(t, manifest.ActivationCase.PlanB64)

	t.Run("trailing-bytes", func(t *testing.T) {
		if _, err := ActivationPlanDeserialize(append(append([]byte(nil), canonical...), 0x00)); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	// A two-receiver plan whose entries are in descending hash order. The
	// decoder must reject it rather than silently canonicalize -- which is
	// exactly what NewActivationPlan alone would have done.
	t.Run("descending-receiver-order", func(t *testing.T) {
		body := append([]byte(nil), canonical[:37]...)
		body = appendU32(body, 2)
		body = append(body, repeatByte(0x04, 32)...) // higher receiver hash first
		body = append(body, repeatByte(0x03, 32)...)
		body = append(body, repeatByte(0x02, 32)...)
		body = append(body, repeatByte(0x01, 32)...)
		if _, err := ActivationPlanDeserialize(body); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	// A 16-octet channel id that is not a real UUID v7 is rejected, matching
	// Rust, Python, Ruby, and Elixir. Accepting it would mean two conforming
	// implementations disagreed about whether the same plan record is valid.
	t.Run("channel-id-is-not-uuid-v7", func(t *testing.T) {
		for _, testCase := range []struct {
			name  string
			index int
			value byte
		}{
			{"wrong-version-nibble", 6, 0x4f},
			{"wrong-variant-bits", 8, 0x1f},
		} {
			t.Run(testCase.name, func(t *testing.T) {
				bad := append([]byte(nil), channelID...)
				bad[testCase.index] = testCase.value
				entry, err := NewActivationPlanEntry(repeatByte(0x01, 32), repeatByte(0x02, 32))
				if err != nil {
					t.Fatal(err)
				}
				if _, err := NewActivationPlan(bad, 0, 1, []ActivationPlanEntry{entry}); !IsCode(err, ErrCorruptRecord) {
					t.Fatalf("expected corrupt_record, got %v", err)
				}
			})
		}

		// The canonical fixture identifier must still be accepted, so the check
		// cannot be satisfied by rejecting everything.
		entry, err := NewActivationPlanEntry(repeatByte(0x01, 32), repeatByte(0x02, 32))
		if err != nil {
			t.Fatal(err)
		}
		if _, err := NewActivationPlan(channelID, 0, 1, []ActivationPlanEntry{entry}); err != nil {
			t.Fatalf("the canonical channel id must remain valid: %v", err)
		}
	})

	t.Run("empty-receiver-set", func(t *testing.T) {
		if _, err := NewActivationPlan(channelID, 0, 1, nil); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	t.Run("non-successor-epoch", func(t *testing.T) {
		entry, err := NewActivationPlanEntry(repeatByte(0x01, 32), repeatByte(0x02, 32))
		if err != nil {
			t.Fatal(err)
		}
		if _, err := NewActivationPlan(channelID, 0, 2, []ActivationPlanEntry{entry}); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	// Two distinct receivers hashing to the same value is a collision, and D18T
	// treats a collision as invalid input rather than as equal authorization.
	t.Run("duplicate-receiver-hash", func(t *testing.T) {
		first, _ := NewActivationPlanEntry(repeatByte(0x01, 32), repeatByte(0x02, 32))
		second, _ := NewActivationPlanEntry(repeatByte(0x01, 32), repeatByte(0x03, 32))
		if _, err := NewActivationPlan(channelID, 0, 1, []ActivationPlanEntry{first, second}); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})

	t.Run("duplicate-grant-hash", func(t *testing.T) {
		first, _ := NewActivationPlanEntry(repeatByte(0x01, 32), repeatByte(0x02, 32))
		second, _ := NewActivationPlanEntry(repeatByte(0x04, 32), repeatByte(0x02, 32))
		if _, err := NewActivationPlan(channelID, 0, 1, []ActivationPlanEntry{first, second}); !IsCode(err, ErrCorruptRecord) {
			t.Fatalf("expected corrupt_record, got %v", err)
		}
	})
}

func repeatByte(value byte, length int) []byte {
	out := make([]byte, length)
	for index := range out {
		out[index] = value
	}
	return out
}

func traceNames(traces []namedTrace) []string {
	out := make([]string, len(traces))
	for index, trace := range traces {
		out[index] = trace.Name
	}
	return out
}

func traceNamesOfMigrations(migrations []stateMigration) []string {
	out := make([]string, len(migrations))
	for index, migration := range migrations {
		out[index] = migration.Name
	}
	return out
}

func equalStrings(left, right []string) bool {
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
