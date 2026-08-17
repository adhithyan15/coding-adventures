package channelcrypto

import "testing"

// preUnwrapOpeningCodes are the D18Q failures that VerifyGrantSignature can
// reach, because they are decided before a receiver secret is ever needed.
//
// Every other opening failure -- invalid_key_agreement and the whole
// authentication_failed family -- lives strictly *after* signature
// verification, in the X25519 agreement or the AEAD open. Those cases are
// therefore expected to VERIFY SUCCESSFULLY here. That is the sharpest
// statement these tests make: the receiver-key-free entry point stops at
// exactly the boundary where the receiver key becomes necessary, no earlier
// and no later.
var preUnwrapOpeningCodes = map[string]bool{
	"unexpected_originator": true,
	"unexpected_receiver":   true,
	"unexpected_channel":    true,
	"invalid_signature":     true,
}

func TestVerifyGrantSignatureAcceptsEveryPositiveFixtureUsingOnlyPublicInputs(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	if len(fixtures.manifest.PositiveCases) == 0 {
		t.Fatal("expected at least one positive D18Q fixture")
	}
	for _, testCase := range fixtures.manifest.PositiveCases {
		t.Run(testCase.Name, func(t *testing.T) {
			grant, err := GrantDeserialize(decodeGrantB64(t, testCase.D18GB64))
			if err != nil {
				t.Fatal(err)
			}
			// Note what is absent: receiver_private_key_hex is never read.
			// An originator holds no receiver secrets, and D18T requires it to
			// verify every receiver's grant regardless.
			if err := VerifyGrantSignature(
				grant,
				decodeGrantB64(t, testCase.OriginatorIDB64),
				decodeGrantB64(t, testCase.ReceiverIDB64),
				decodeHex(t, testCase.ChannelIDHex),
				fixtures.publicKey,
			); err != nil {
				t.Fatalf("expected the canonical grant to verify, got %v", err)
			}
		})
	}
}

func TestVerifyGrantSignatureStopsExactlyAtTheReceiverKeyBoundary(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	seenPreUnwrap, seenPostUnwrap := 0, 0
	for _, testCase := range fixtures.manifest.OpeningNegativeCases {
		t.Run(testCase.Name, func(t *testing.T) {
			grant, err := GrantDeserialize(decodeGrantB64(t, testCase.D18GB64))
			if err != nil {
				t.Fatal(err)
			}
			err = VerifyGrantSignature(
				grant,
				decodeGrantB64(t, testCase.ExpectedOriginatorIDB64),
				decodeGrantB64(t, testCase.ExpectedReceiverIDB64),
				decodeHex(t, testCase.ExpectedChannelIDHex),
				fixtures.publicKey,
			)
			if preUnwrapOpeningCodes[testCase.ExpectedError] {
				seenPreUnwrap++
				// Same grant, same expectation, same stable code as
				// OpenChannelKeyGrant would have produced.
				requireGrantCode(t, err, testCase.ExpectedError)
				return
			}
			seenPostUnwrap++
			if err != nil {
				t.Fatalf("%q fails only during unwrapping, so signature verification must succeed; got %v", testCase.ExpectedError, err)
			}
		})
	}
	// Guard against a manifest that silently loses one side of the split and
	// leaves half this test vacuous.
	if seenPreUnwrap == 0 || seenPostUnwrap == 0 {
		t.Fatalf("expected the manifest to exercise both sides of the boundary, saw %d pre-unwrap and %d post-unwrap", seenPreUnwrap, seenPostUnwrap)
	}
}

func TestVerifyGrantSignatureAndOpenChannelKeyGrantAgreeOnEveryOpeningFixture(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	for _, testCase := range fixtures.manifest.OpeningNegativeCases {
		if !preUnwrapOpeningCodes[testCase.ExpectedError] {
			continue
		}
		t.Run(testCase.Name, func(t *testing.T) {
			grant, err := GrantDeserialize(decodeGrantB64(t, testCase.D18GB64))
			if err != nil {
				t.Fatal(err)
			}
			receiver, err := ReceiverKeyPairFromPrivateKey(decodeHex(t, testCase.ReceiverPrivateKeyHex))
			if err != nil {
				t.Fatal(err)
			}
			defer receiver.Destroy()
			originator := decodeGrantB64(t, testCase.ExpectedOriginatorIDB64)
			receiverID := decodeGrantB64(t, testCase.ExpectedReceiverIDB64)
			channel := decodeHex(t, testCase.ExpectedChannelIDHex)

			verifyErr := VerifyGrantSignature(grant, originator, receiverID, channel, fixtures.publicKey)
			_, openErr := OpenChannelKeyGrant(grant, originator, receiverID, channel, receiver, fixtures.publicKey)

			// Both paths share verifyGrantBindings, so they can never disagree
			// on a pre-unwrap failure. This test is what makes that structural
			// claim observable rather than merely asserted in a comment.
			requireGrantCode(t, verifyErr, testCase.ExpectedError)
			requireGrantCode(t, openErr, testCase.ExpectedError)
		})
	}
}

func TestVerifyGrantSignatureRejectsMalformedPublicInputs(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	positive := fixtures.manifest.PositiveCases[0]
	grant, err := GrantDeserialize(decodeGrantB64(t, positive.D18GB64))
	if err != nil {
		t.Fatal(err)
	}
	originator := decodeGrantB64(t, positive.OriginatorIDB64)
	receiverID := decodeGrantB64(t, positive.ReceiverIDB64)
	channel := decodeHex(t, positive.ChannelIDHex)

	for _, testCase := range []struct {
		name      string
		channel   []byte
		publicKey []byte
	}{
		{"short-channel-id", channel[:15], fixtures.publicKey},
		{"long-channel-id", append(append([]byte{}, channel...), 0), fixtures.publicKey},
		{"empty-channel-id", []byte{}, fixtures.publicKey},
		{"short-public-key", channel, fixtures.publicKey[:31]},
		{"long-public-key", channel, append(append([]byte{}, fixtures.publicKey...), 0)},
		{"empty-public-key", channel, []byte{}},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			if err := VerifyGrantSignature(grant, originator, receiverID, testCase.channel, testCase.publicKey); err == nil {
				t.Fatal("expected malformed public inputs to fail closed")
			}
		})
	}
}

func TestVerifyGrantSignatureRejectsAnotherOriginatorsPublicKey(t *testing.T) {
	fixtures := loadGrantFixtures(t)
	positive := fixtures.manifest.PositiveCases[0]
	grant, err := GrantDeserialize(decodeGrantB64(t, positive.D18GB64))
	if err != nil {
		t.Fatal(err)
	}
	// A well-formed key belonging to somebody else must not verify. Without
	// this, a caller could be fooled by any 32 valid bytes.
	other, err := OriginatorSigningKeyFromSeed(make([]byte, 32))
	if err != nil {
		t.Fatal(err)
	}
	defer other.Destroy()
	otherPublic, err := other.PublicKey()
	if err != nil {
		t.Fatal(err)
	}
	err = VerifyGrantSignature(
		grant,
		decodeGrantB64(t, positive.OriginatorIDB64),
		decodeGrantB64(t, positive.ReceiverIDB64),
		decodeHex(t, positive.ChannelIDHex),
		otherPublic,
	)
	requireGrantCode(t, err, string(KeyGrantErrInvalidSignature))
}
