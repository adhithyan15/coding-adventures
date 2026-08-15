import { readFileSync } from "node:fs";
import { generateKeypair as x25519PublicKey, x25519 } from "@coding-adventures/x25519";
import { describe, expect, it } from "vitest";
import {
  ChannelMasterKey,
  KEY_GRANT_ERROR_CODES,
  KeyGrantFields,
  KeyGrantProfileError,
  OriginatorSigningKey,
  PortableKeyGrant,
  ReceiverEpochKeys,
  ReceiverKeyPair,
  RotationReceiver,
  grantDeserialize,
  grantSerialize,
  keyGrantAad,
  keyGrantHkdfInfo,
  keyGrantHkdfSalt,
  keyGrantSignatureInput,
  keyGrantWrappingKey,
  openChannelKeyGrant,
  planRotation,
  sealChannelKey,
  sealChannelKeyWithMaterial,
  secretErasureCapability,
  type KeyGrantErrorCode,
  type SecureRandomSource,
} from "../src/index.js";

interface PositiveCase {
  name: string;
  originator_id_b64: string;
  receiver_id_b64: string;
  channel_id_hex: string;
  key_epoch: string;
  cmk_hex: string;
  receiver_private_key_hex: string;
  receiver_public_key_hex: string;
  ephemeral_private_key_hex: string;
  ephemeral_public_key_hex: string;
  shared_secret_hex: string;
  hkdf_salt_b64: string;
  hkdf_info_b64: string;
  wrapping_key_hex: string;
  wrapping_nonce_hex: string;
  grant_aad_b64: string;
  wrapped_cmk_hex: string;
  signature_input_b64: string;
  signature_hex: string;
  d18g_b64: string;
  expected_opened_cmk_hex: string;
}

interface Fixture {
  fixture_format: string;
  spec: string;
  generator_blob_sha1: string;
  warning: string;
  constants: Record<string, string>;
  test_signing_key: { seed_hex: string; public_key_hex: string };
  positive_cases: PositiveCase[];
  structural_negative_cases: Array<{ name: string; d18g_b64: string; expected_error: KeyGrantErrorCode }>;
  truncated_prefix_recipe: { source_case: string; first_length: string; last_length_exclusive: string; expected_error: KeyGrantErrorCode };
  oversize_recipes: Array<{ field: string; length_offset: string; declared_length: string; expected_error: KeyGrantErrorCode }>;
  field_negative_cases: Array<{ name: string; expected_error: KeyGrantErrorCode }>;
  seal_negative_cases: Array<{ name: string; expected_error: KeyGrantErrorCode }>;
  opening_negative_cases: Array<{
    name: string;
    d18g_b64: string;
    expected_originator_id_b64: string;
    expected_receiver_id_b64: string;
    expected_channel_id_hex: string;
    receiver_private_key_hex: string;
    expected_error: KeyGrantErrorCode;
  }>;
  receiver_state_trace: {
    grants: Record<string, string>;
    steps: Array<{
      name: string;
      grant: string;
      expected: string;
      latest_epoch: string;
      retained_epochs: string[];
    }>;
    missing_epoch: string;
    missing_epoch_error: KeyGrantErrorCode;
  };
  rotation_case: {
    name: string;
    current_epoch: string;
    new_epoch: string;
    new_cmk_hex: string;
    authorized_receiver_ids_b64: string[];
    new_grants_b64: string[];
    receiver_a_retains_epochs: string[];
    receiver_b_retains_epochs: string[];
    receiver_a_new_grant: null;
  };
  secret_erasure_capabilities: string[];
  rust_secret_erasure_capability: string;
  stable_error_codes: KeyGrantErrorCode[];
}

const fixture = JSON.parse(readFileSync(
  new URL("../../../../fixtures/chief-of-staff-channel-key-grant/v1/manifest.json", import.meta.url),
  "utf8",
)) as Fixture;
const signingSeed = fromHex(fixture.test_signing_key.seed_hex);
const signer = OriginatorSigningKey.fromSeed(signingSeed);
const originatorPublicKey = fromHex(fixture.test_signing_key.public_key_hex);
const channelId = fromHex(fixture.positive_cases[0]!.channel_id_hex);

describe("D18Q shared grant fixture", () => {
  it("locks provenance, constants, closed rosters, and honest erasure", () => {
    expect(Object.keys(fixture)).toEqual([
      "fixture_format", "spec", "generator_blob_sha1", "warning", "constants",
      "test_signing_key", "positive_cases", "structural_negative_cases",
      "truncated_prefix_recipe", "oversize_recipes", "field_negative_cases",
      "seal_negative_cases", "opening_negative_cases", "receiver_state_trace",
      "rotation_case", "secret_erasure_capabilities", "rust_secret_erasure_capability",
      "stable_error_codes",
    ]);
    expect(fixture.fixture_format).toBe("D18Q-channel-key-grant-fixtures-v1");
    expect(fixture.spec).toBe("code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md");
    expect(fixture.generator_blob_sha1).toMatch(/^[0-9a-f]{40}$/);
    expect(fixture.warning).toContain("test-only");
    expect(fixture.warning).toContain("Never log");
    expect(fixture.constants).toEqual({
      key_grant_context_ascii: "chief-channel-key-grant-v1",
      key_wrap_context_ascii: "chief-channel-key-wrap-v1",
      max_identity_bytes: "4096",
      wire_magic_ascii: "D18G",
      wire_version: "1",
    });
    expect([...KEY_GRANT_ERROR_CODES]).toEqual(fixture.stable_error_codes);
    expect(fixture.secret_erasure_capabilities).toEqual([
      "guaranteed", "best_effort", "not_enforceable",
    ]);
    expect(secretErasureCapability()).toBe("best_effort");
    expect(fixture.rust_secret_erasure_capability).toBe("guaranteed");
    expect(fixture.positive_cases.map(({ name }) => name)).toEqual([
      "epoch-zero-receiver-a", "epoch-zero-receiver-b", "maximum-epoch-receiver-a",
    ]);
    for (const testCase of fixture.positive_cases) {
      expect(Object.keys(testCase)).toEqual([
        "name", "originator_id_b64", "receiver_id_b64", "channel_id_hex", "key_epoch",
        "cmk_hex", "receiver_private_key_hex", "receiver_public_key_hex",
        "ephemeral_private_key_hex", "ephemeral_public_key_hex", "shared_secret_hex",
        "hkdf_salt_b64", "hkdf_info_b64", "wrapping_key_hex", "wrapping_nonce_hex",
        "grant_aad_b64", "wrapped_cmk_hex", "signature_input_b64", "signature_hex",
        "d18g_b64", "expected_opened_cmk_hex",
      ]);
    }
    expectCaseRoster(fixture.structural_negative_cases,
      ["wrong-magic", "unsupported-version", "trailing-byte"],
      ["name", "d18g_b64", "expected_error"]);
    expectCaseRoster(fixture.field_negative_cases,
      ["empty-originator", "empty-receiver", "invalid-uuid-version", "invalid-uuid-variant", "oversized-originator", "oversized-receiver"],
      ["name", "expected_error"]);
    expectCaseRoster(fixture.seal_negative_cases,
      ["low-order-receiver-public-key"], ["name", "expected_error"]);
    expectCaseRoster(fixture.opening_negative_cases, [
      "unexpected-originator", "unexpected-receiver", "unexpected-channel", "invalid-signature",
      "invalid-signature-before-key-agreement", "low-order-ephemeral-public-key",
      "wrong-receiver-private-key", "wrong-wrapping-nonce", "mutated-wrapped-cmk",
      "mutated-tag", "epoch-derivation-binding", "receiver-derivation-binding",
      "channel-aad-binding", "originator-aad-binding",
    ], [
      "name", "d18g_b64", "expected_originator_id_b64", "expected_receiver_id_b64",
      "expected_channel_id_hex", "receiver_private_key_hex", "expected_error",
    ]);
    expect(Object.keys(fixture.truncated_prefix_recipe)).toEqual([
      "source_case", "first_length", "last_length_exclusive", "expected_error",
    ]);
    for (const recipe of fixture.oversize_recipes) {
      expect(Object.keys(recipe)).toEqual(["field", "length_offset", "declared_length", "expected_error"]);
    }
    expect(Object.keys(fixture.receiver_state_trace)).toEqual([
      "grants", "steps", "missing_epoch", "missing_epoch_error",
    ]);
    expect(fixture.receiver_state_trace.steps.map(({ name }) => name)).toEqual([
      "install-epoch-zero", "retry-epoch-zero", "same-epoch-conflict",
      "failed-higher-open", "install-skipped-epoch-three", "decreasing-epoch",
    ]);
    for (const step of fixture.receiver_state_trace.steps) {
      expect(Object.keys(step)).toEqual(["name", "grant", "expected", "latest_epoch", "retained_epochs"]);
    }
    expect(Object.keys(fixture.rotation_case)).toEqual([
      "name", "current_epoch", "new_epoch", "new_cmk_hex", "authorized_receiver_ids_b64",
      "new_grants_b64", "receiver_a_retains_epochs", "receiver_b_retains_epochs",
      "receiver_a_new_grant",
    ]);
  });

  it("reproduces every derivation input and production D18G byte", () => {
    expect(signer.publicKey).toEqual(originatorPublicKey);
    for (const testCase of fixture.positive_cases) {
      const originatorId = fromBase64(testCase.originator_id_b64);
      const receiverId = fromBase64(testCase.receiver_id_b64);
      const testChannelId = fromHex(testCase.channel_id_hex);
      const epoch = BigInt(testCase.key_epoch);
      const receiverPrivateKey = fromHex(testCase.receiver_private_key_hex);
      const receiver = ReceiverKeyPair.fromPrivateKey(receiverPrivateKey);
      expect(receiver.publicKey, `${testCase.name}: receiver public key`)
        .toEqual(fromHex(testCase.receiver_public_key_hex));
      const ephemeralPrivateKey = fromHex(testCase.ephemeral_private_key_hex);
      const ephemeralPublicKey = x25519PublicKey(ephemeralPrivateKey);
      expect(ephemeralPublicKey, `${testCase.name}: ephemeral public key`)
        .toEqual(fromHex(testCase.ephemeral_public_key_hex));
      const sharedSecret = x25519(ephemeralPrivateKey, receiver.publicKey);
      expect(sharedSecret, `${testCase.name}: shared secret`)
        .toEqual(fromHex(testCase.shared_secret_hex));
      expect(keyGrantHkdfSalt(testChannelId, epoch), `${testCase.name}: salt`)
        .toEqual(fromBase64(testCase.hkdf_salt_b64));
      expect(keyGrantHkdfInfo(receiverId), `${testCase.name}: info`)
        .toEqual(fromBase64(testCase.hkdf_info_b64));
      expect(
        keyGrantWrappingKey(sharedSecret, testChannelId, epoch, receiverId),
        `${testCase.name}: wrapping key`,
      ).toEqual(fromHex(testCase.wrapping_key_hex));
      const fields = new KeyGrantFields(originatorId, receiverId, testChannelId, epoch);
      const cmk = ChannelMasterKey.fromBytes(fromHex(testCase.cmk_hex));
      const grant = sealChannelKeyWithMaterial(
        fields,
        cmk,
        receiver.publicKey,
        signer,
        ephemeralPrivateKey,
        fromHex(testCase.wrapping_nonce_hex),
      );
      const record = fromBase64(testCase.d18g_b64);
      expect(grantSerialize(grant), `${testCase.name}: D18G`).toEqual(record);
      expect(grant.wrappedCmk).toEqual(fromHex(testCase.wrapped_cmk_hex));
      expect(grant.originatorSignature).toEqual(fromHex(testCase.signature_hex));
      expect(keyGrantAad(grant)).toEqual(fromBase64(testCase.grant_aad_b64));
      expect(keyGrantSignatureInput(grant)).toEqual(fromBase64(testCase.signature_input_b64));
      const decoded = grantDeserialize(record);
      expect(grantSerialize(decoded), `${testCase.name}: round trip`).toEqual(record);
      expect(openChannelKeyGrant(
        decoded, originatorId, receiverId, testChannelId, receiver, originatorPublicKey,
      ).bytes).toEqual(fromHex(testCase.expected_opened_cmk_hex));

      const mutableOriginator = grant.originatorId;
      mutableOriginator.fill(0);
      expect(grantSerialize(grant)).toEqual(record);
      expect(Object.isFrozen(grant)).toBe(true);
      cmk.destroy();
      receiver.destroy();
      sharedSecret.fill(0);
    }
  });

  it("maps every structural, field, and seal failure to its declared code", () => {
    const base = fromBase64(fixture.positive_cases[0]!.d18g_b64);
    for (const testCase of fixture.structural_negative_cases) {
      expectCode(() => grantDeserialize(fromBase64(testCase.d18g_b64)), testCase.expected_error, testCase.name);
    }
    const last = Number(fixture.truncated_prefix_recipe.last_length_exclusive);
    expect(last).toBe(base.length);
    for (let end = Number(fixture.truncated_prefix_recipe.first_length); end < last; end++) {
      expectCode(
        () => grantDeserialize(base.slice(0, end)),
        fixture.truncated_prefix_recipe.expected_error,
        `truncated prefix ${end}`,
      );
    }
    for (const recipe of fixture.oversize_recipes) {
      const record = base.slice();
      new DataView(record.buffer).setUint32(
        Number(recipe.length_offset), Number(recipe.declared_length), false,
      );
      expectCode(() => grantDeserialize(record), recipe.expected_error, recipe.field);
    }
    for (const testCase of fixture.field_negative_cases) {
      let originatorId = utf8("originator");
      let receiverId = utf8("receiver");
      const invalidChannelId = channelId.slice();
      if (testCase.name === "empty-originator") originatorId = new Uint8Array();
      else if (testCase.name === "empty-receiver") receiverId = new Uint8Array();
      else if (testCase.name === "invalid-uuid-version") invalidChannelId[6] = 0x60;
      else if (testCase.name === "invalid-uuid-variant") invalidChannelId[8] = 0x10;
      else if (testCase.name === "oversized-originator") originatorId = new Uint8Array(4097);
      else if (testCase.name === "oversized-receiver") receiverId = new Uint8Array(4097);
      expectCode(
        () => new KeyGrantFields(originatorId, receiverId, invalidChannelId, 0n),
        testCase.expected_error,
        testCase.name,
      );
    }
    const fields = new KeyGrantFields(utf8("originator"), utf8("receiver"), channelId, 0n);
    expectCode(() => sealChannelKeyWithMaterial(
      fields,
      ChannelMasterKey.fromBytes(new Uint8Array(32).fill(0x22)),
      new Uint8Array(32),
      signer,
      new Uint8Array(32).fill(0x51),
      new Uint8Array(24).fill(0x61),
    ), fixture.seal_negative_cases[0]!.expected_error);
  });

  it("follows the declared opening validation order for every negative case", () => {
    for (const testCase of fixture.opening_negative_cases) {
      const grant = grantDeserialize(fromBase64(testCase.d18g_b64));
      const receiver = ReceiverKeyPair.fromPrivateKey(fromHex(testCase.receiver_private_key_hex));
      expectCode(() => openChannelKeyGrant(
        grant,
        fromBase64(testCase.expected_originator_id_b64),
        fromBase64(testCase.expected_receiver_id_b64),
        fromHex(testCase.expected_channel_id_hex),
        receiver,
        originatorPublicKey,
      ), testCase.expected_error, testCase.name);
      receiver.destroy();
    }
  });

  it("installs grants atomically, monotonically, and with skipped epochs", () => {
    const first = fixture.positive_cases[0]!;
    const state = new ReceiverEpochKeys(
      fromBase64(first.originator_id_b64),
      fromBase64(first.receiver_id_b64),
      channelId,
      ReceiverKeyPair.fromPrivateKey(fromHex(first.receiver_private_key_hex)),
      originatorPublicKey,
    );
    const trace = fixture.receiver_state_trace;
    for (const step of trace.steps) {
      const grant = grantDeserialize(fromBase64(trace.grants[step.grant]!));
      let actual: string;
      try {
        actual = state.installGrant(grant);
      } catch (error) {
        actual = errorCode(error);
      }
      expect(actual, step.name).toBe(step.expected);
      expect(String(state.latestEpoch), `${step.name}: latest`).toBe(step.latest_epoch);
      const retained = [0n, 1n, 2n, 3n].filter((epoch) => {
        try { state.key(epoch).destroy(); return true; } catch { return false; }
      }).map(String);
      expect(retained, `${step.name}: retained`).toEqual(step.retained_epochs);
    }
    expectCode(() => state.key(BigInt(trace.missing_epoch)), trace.missing_epoch_error);
    expectCode(() => state.installGrant(new PortableKeyGrant({
      originatorId: new Uint8Array(),
      receiverId: new Uint8Array(),
      channelId: new Uint8Array(16),
      keyEpoch: state.latestEpoch!,
      ephemeralPublicKey: new Uint8Array(32),
      wrappingNonce: new Uint8Array(24),
      wrappedCmk: new Uint8Array(48),
      originatorSignature: new Uint8Array(64),
    })), "conflicting_grant");
    state.destroy();
  });

  it("reproduces prospective revocation and the ordered rotation plan", () => {
    const [a, b] = fixture.positive_cases;
    const receiverA = ReceiverKeyPair.fromPrivateKey(fromHex(a!.receiver_private_key_hex));
    const receiverB = ReceiverKeyPair.fromPrivateKey(fromHex(b!.receiver_private_key_hex));
    const stateA = new ReceiverEpochKeys(
      fromBase64(a!.originator_id_b64), fromBase64(a!.receiver_id_b64), channelId,
      receiverA, originatorPublicKey,
    );
    const stateB = new ReceiverEpochKeys(
      fromBase64(b!.originator_id_b64), fromBase64(b!.receiver_id_b64), channelId,
      receiverB, originatorPublicKey,
    );
    stateA.installGrant(grantDeserialize(fromBase64(a!.d18g_b64)));
    stateB.installGrant(grantDeserialize(fromBase64(b!.d18g_b64)));
    const rotation = fixture.rotation_case;
    const newCmk = ChannelMasterKey.fromBytes(fromHex(rotation.new_cmk_hex));
    const plan = planRotation(
      fromBase64(a!.originator_id_b64),
      channelId,
      BigInt(rotation.current_epoch),
      newCmk,
      [RotationReceiver.withMaterial(
        fromBase64(b!.receiver_id_b64), receiverB.publicKey,
        new Uint8Array(32).fill(0x71), new Uint8Array(24).fill(0x81),
      )],
      signer,
    );
    expect(plan.newEpoch).toBe(BigInt(rotation.new_epoch));
    expect(plan.grants.map((grant) => toBase64(grantSerialize(grant))))
      .toEqual(rotation.new_grants_b64);
    expect(plan.grants.map((grant) => toBase64(grant.receiverId)))
      .toEqual(rotation.authorized_receiver_ids_b64);
    stateB.installGrant(plan.grants[0]!);
    expect(retainedEpochs(stateA, 1n)).toEqual(rotation.receiver_a_retains_epochs);
    expect(retainedEpochs(stateB, 1n)).toEqual(rotation.receiver_b_retains_epochs);
    expect(rotation.receiver_a_new_grant).toBeNull();
    const plannedCmk = plan.newCmk;
    const installedCmk = stateB.key(1n);
    expect(installedCmk.bytes).toEqual(plannedCmk.bytes);
    installedCmk.destroy();
    plannedCmk.destroy();
    plan.destroy();
    newCmk.destroy();
    stateA.destroy();
    stateB.destroy();
  });

  it("keeps production entropy fail-closed and rotation all-or-nothing", () => {
    const first = fixture.positive_cases[0]!;
    const fields = new KeyGrantFields(
      fromBase64(first.originator_id_b64), fromBase64(first.receiver_id_b64),
      channelId, BigInt(first.key_epoch),
    );
    const cmk = ChannelMasterKey.fromBytes(fromHex(first.cmk_hex));
    const receiverPublicKey = fromHex(first.receiver_public_key_hex);
    const deterministic = queuedRandomSource([
      fromHex(first.ephemeral_private_key_hex), fromHex(first.wrapping_nonce_hex),
    ]);
    expect(grantSerialize(sealChannelKey(fields, cmk, receiverPublicKey, signer, deterministic)))
      .toEqual(fromBase64(first.d18g_b64));

    const generatedCmk = ChannelMasterKey.generate(queuedRandomSource([new Uint8Array(32).fill(9)]));
    expect(generatedCmk.bytes).toEqual(new Uint8Array(32).fill(9));
    const generatedReceiver = ReceiverKeyPair.generate(queuedRandomSource([new Uint8Array(32).fill(10)]));
    expect(generatedReceiver.publicKey).toHaveLength(32);
    const generatedSigner = OriginatorSigningKey.generate(queuedRandomSource([new Uint8Array(32).fill(11)]));
    expect(generatedSigner.publicKey).toHaveLength(32);
    generatedCmk.destroy();
    generatedReceiver.destroy();
    generatedSigner.destroy();

    const shortSource: SecureRandomSource = { randomBytes: (length) => new Uint8Array(length - 1) };
    const throwingSource: SecureRandomSource = { randomBytes: () => { throw new Error("secret detail"); } };
    expectCode(() => ChannelMasterKey.generate(shortSource), "randomness_unavailable");
    expectCode(() => ReceiverKeyPair.generate(throwingSource), "randomness_unavailable");
    expectCode(() => OriginatorSigningKey.generate(shortSource), "randomness_unavailable");
    expectCode(() => sealChannelKey(fields, cmk, receiverPublicKey, signer, shortSource), "randomness_unavailable");
    expectCode(() => RotationReceiver.generate(utf8("receiver"), receiverPublicKey, shortSource), "randomness_unavailable");

    expectCode(() => planRotation(utf8("originator"), channelId, (1n << 64n) - 1n, cmk, [
      RotationReceiver.withMaterial(utf8("receiver"), receiverPublicKey, new Uint8Array(32).fill(3), new Uint8Array(24).fill(4)),
    ], signer), "epoch_exhausted");
    expectCode(() => planRotation(utf8("originator"), channelId, 0n, cmk, [], signer), "invalid_field");
    const duplicateA = RotationReceiver.withMaterial(
      utf8("duplicate"), receiverPublicKey, new Uint8Array(32).fill(5), new Uint8Array(24).fill(6),
    );
    const duplicateB = RotationReceiver.withMaterial(
      utf8("duplicate"), receiverPublicKey, new Uint8Array(32).fill(7), new Uint8Array(24).fill(8),
    );
    expectCode(() => planRotation(
      utf8("originator"), channelId, 0n, cmk, [duplicateB, duplicateA], signer,
    ), "invalid_field");
    const sortedPlan = planRotation(utf8("originator"), channelId, 0n, cmk, [
      RotationReceiver.withMaterial(utf8("receiver-b"), receiverPublicKey, new Uint8Array(32).fill(12), new Uint8Array(24).fill(13)),
      RotationReceiver.withMaterial(utf8("receiver-a"), receiverPublicKey, new Uint8Array(32).fill(14), new Uint8Array(24).fill(15)),
    ], signer);
    expect(sortedPlan.grants.map((grant) => new TextDecoder().decode(grant.receiverId)))
      .toEqual(["receiver-a", "receiver-b"]);
    sortedPlan.destroy();
    duplicateA.destroy();
    duplicateB.destroy();
    cmk.destroy();
    expectCode(() => cmk.bytes, "invalid_field");
  });
});

function retainedEpochs(state: ReceiverEpochKeys, maximum: bigint): string[] {
  const result: string[] = [];
  for (let epoch = 0n; epoch <= maximum; epoch++) {
    try {
      const key = state.key(epoch);
      key.destroy();
      result.push(String(epoch));
    } catch {
      // Missing epochs are intentionally absent.
    }
  }
  return result;
}

function queuedRandomSource(chunks: Uint8Array[]): SecureRandomSource {
  let index = 0;
  return {
    randomBytes(length: number): Uint8Array {
      const chunk = chunks[index++];
      if (chunk === undefined || chunk.length !== length) throw new Error("unexpected request");
      return chunk.slice();
    },
  };
}

function expectCaseRoster(
  cases: Array<{ name: string }>,
  names: string[],
  fields: string[],
): void {
  expect(cases.map(({ name }) => name)).toEqual(names);
  for (const testCase of cases) expect(Object.keys(testCase)).toEqual(fields);
}

function expectCode(operation: () => unknown, code: KeyGrantErrorCode, label?: string): void {
  try {
    operation();
    throw new Error(`${label ?? code}: operation unexpectedly succeeded`);
  } catch (error) {
    expect(errorCode(error), label).toBe(code);
  }
}

function errorCode(error: unknown): string {
  if (error instanceof KeyGrantProfileError) return error.code;
  throw error;
}

function utf8(value: string): Uint8Array {
  return new TextEncoder().encode(value);
}

function fromHex(value: string): Uint8Array {
  return Uint8Array.from(Buffer.from(value, "hex"));
}

function fromBase64(value: string): Uint8Array {
  return Uint8Array.from(Buffer.from(value, "base64"));
}

function toBase64(value: Uint8Array): string {
  return Buffer.from(value).toString("base64");
}
