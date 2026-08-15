/** Portable D18Q channel-key grants for Chief of Staff channels. */

import {
  xchacha20Poly1305Decrypt,
  xchacha20Poly1305Encrypt,
} from "@coding-adventures/chacha20-poly1305";
import {
  generateKeypair as generateEd25519Keypair,
  sign,
  verify,
} from "@coding-adventures/ed25519";
import { hkdf } from "@coding-adventures/hkdf";
import { generateKeypair as x25519PublicKey, x25519 } from "@coding-adventures/x25519";

const GRANT_MAGIC = Uint8Array.of(0x44, 0x31, 0x38, 0x47);
const WIRE_VERSION = 1;
const MAX_IDENTITY_BYTES = 4096;
const MAX_U64 = (1n << 64n) - 1n;
const KEY_GRANT_CONTEXT = new TextEncoder().encode("chief-channel-key-grant-v1");
const KEY_WRAP_CONTEXT = new TextEncoder().encode("chief-channel-key-wrap-v1");

/** Stable D18Q error codes shared by every portable implementation. */
export const KEY_GRANT_ERROR_CODES = [
  "invalid_magic",
  "unsupported_version",
  "truncated_record",
  "trailing_bytes",
  "length_limit_exceeded",
  "invalid_field",
  "randomness_unavailable",
  "invalid_key_agreement",
  "key_derivation_failed",
  "invalid_signature",
  "unexpected_originator",
  "unexpected_receiver",
  "unexpected_channel",
  "authentication_failed",
  "invalid_wrapped_key",
  "conflicting_grant",
  "decreasing_epoch",
  "epoch_exhausted",
  "missing_epoch_key",
] as const;

export type KeyGrantErrorCode = (typeof KEY_GRANT_ERROR_CODES)[number];

/** One fail-closed D18Q operation error. */
export class KeyGrantProfileError extends Error {
  readonly code: KeyGrantErrorCode;

  constructor(code: KeyGrantErrorCode) {
    super(code);
    this.name = "KeyGrantProfileError";
    this.code = code;
  }
}

/** Cryptographically secure byte source used by production convenience APIs. */
export interface SecureRandomSource {
  randomBytes(length: number): Uint8Array;
}

/** Browser, Deno, and modern Node secure-random source. */
export const systemSecureRandomSource: SecureRandomSource = Object.freeze({
  randomBytes(length: number): Uint8Array {
    if (!Number.isSafeInteger(length) || length < 0) fail("randomness_unavailable");
    const cryptoApi = globalThis.crypto;
    if (cryptoApi === undefined || typeof cryptoApi.getRandomValues !== "function") {
      fail("randomness_unavailable");
    }
    try {
      const bytes = new Uint8Array(length);
      cryptoApi.getRandomValues(bytes);
      return bytes;
    } catch {
      fail("randomness_unavailable");
    }
  },
});

/** Mutable secret container with explicit best-effort destruction. */
export class ChannelMasterKey {
  readonly #bytes: Uint8Array;
  #destroyed = false;

  private constructor(bytes: Uint8Array) {
    requireLength(bytes, 32);
    this.#bytes = bytes.slice();
    Object.freeze(this);
  }

  static fromBytes(bytes: Uint8Array): ChannelMasterKey {
    return new ChannelMasterKey(bytes);
  }

  static generate(source: SecureRandomSource = systemSecureRandomSource): ChannelMasterKey {
    const bytes = secureRandomBytes(source, 32);
    try {
      return new ChannelMasterKey(bytes);
    } finally {
      wipe(bytes);
    }
  }

  get bytes(): Uint8Array {
    this.#requireAlive();
    return this.#bytes.slice();
  }

  clone(): ChannelMasterKey {
    this.#requireAlive();
    return new ChannelMasterKey(this.#bytes);
  }

  destroy(): void {
    wipe(this.#bytes);
    this.#destroyed = true;
  }

  #requireAlive(): void {
    if (this.#destroyed) fail("invalid_field");
  }
}

/** Receiver X25519 key pair with an owned, explicitly destructible private key. */
export class ReceiverKeyPair {
  readonly #privateKey: Uint8Array;
  readonly #publicKey: Uint8Array;
  #destroyed = false;

  private constructor(privateKey: Uint8Array, publicKey: Uint8Array) {
    this.#privateKey = privateKey.slice();
    this.#publicKey = publicKey.slice();
    Object.freeze(this);
  }

  static fromPrivateKey(privateKey: Uint8Array): ReceiverKeyPair {
    requireLength(privateKey, 32);
    let publicKey: Uint8Array;
    try {
      publicKey = x25519PublicKey(privateKey);
    } catch {
      fail("invalid_key_agreement");
    }
    return new ReceiverKeyPair(privateKey, publicKey);
  }

  static generate(source: SecureRandomSource = systemSecureRandomSource): ReceiverKeyPair {
    const privateKey = secureRandomBytes(source, 32);
    try {
      return ReceiverKeyPair.fromPrivateKey(privateKey);
    } finally {
      wipe(privateKey);
    }
  }

  get publicKey(): Uint8Array {
    this.#requireAlive();
    return this.#publicKey.slice();
  }

  clone(): ReceiverKeyPair {
    this.#requireAlive();
    return new ReceiverKeyPair(this.#privateKey, this.#publicKey);
  }

  /** Derive one shared secret without exposing this object's private key. */
  agree(peerPublicKey: Uint8Array): Uint8Array {
    this.#requireAlive();
    requireLength(peerPublicKey, 32);
    try {
      return x25519(this.#privateKey, peerPublicKey);
    } catch {
      fail("invalid_key_agreement");
    }
  }

  destroy(): void {
    wipe(this.#privateKey);
    this.#destroyed = true;
  }

  #requireAlive(): void {
    if (this.#destroyed) fail("invalid_field");
  }
}

/** Originator Ed25519 signing identity with an owned secret key. */
export class OriginatorSigningKey {
  readonly #secretKey: Uint8Array;
  readonly #publicKey: Uint8Array;
  #destroyed = false;

  private constructor(secretKey: Uint8Array, publicKey: Uint8Array) {
    this.#secretKey = secretKey.slice();
    this.#publicKey = publicKey.slice();
    Object.freeze(this);
  }

  static fromSeed(seed: Uint8Array): OriginatorSigningKey {
    requireLength(seed, 32);
    let pair: { publicKey: Uint8Array; secretKey: Uint8Array };
    try {
      pair = generateEd25519Keypair(seed);
    } catch {
      fail("invalid_field");
    }
    try {
      return new OriginatorSigningKey(pair.secretKey, pair.publicKey);
    } finally {
      wipe(pair.secretKey);
    }
  }

  static generate(source: SecureRandomSource = systemSecureRandomSource): OriginatorSigningKey {
    const seed = secureRandomBytes(source, 32);
    try {
      return OriginatorSigningKey.fromSeed(seed);
    } finally {
      wipe(seed);
    }
  }

  get publicKey(): Uint8Array {
    this.#requireAlive();
    return this.#publicKey.slice();
  }

  sign(message: Uint8Array): Uint8Array {
    this.#requireAlive();
    return sign(message, this.#secretKey);
  }

  destroy(): void {
    wipe(this.#secretKey);
    this.#destroyed = true;
  }

  #requireAlive(): void {
    if (this.#destroyed) fail("invalid_field");
  }
}

/** Immutable, validated logical fields for one receiver-bound grant. */
export class KeyGrantFields {
  readonly #originatorId: Uint8Array;
  readonly #receiverId: Uint8Array;
  readonly #channelId: Uint8Array;
  readonly keyEpoch: bigint;

  constructor(originatorId: Uint8Array, receiverId: Uint8Array, channelId: Uint8Array, keyEpoch: bigint) {
    validateIdentity(originatorId);
    validateIdentity(receiverId);
    validateChannelId(channelId);
    requireU64(keyEpoch);
    this.#originatorId = originatorId.slice();
    this.#receiverId = receiverId.slice();
    this.#channelId = channelId.slice();
    this.keyEpoch = keyEpoch;
    Object.freeze(this);
  }

  get originatorId(): Uint8Array { return this.#originatorId.slice(); }
  get receiverId(): Uint8Array { return this.#receiverId.slice(); }
  get channelId(): Uint8Array { return this.#channelId.slice(); }
}

/** Complete public fields in one structurally decoded D18G record. */
export interface KeyGrantParts {
  readonly originatorId: Uint8Array;
  readonly receiverId: Uint8Array;
  readonly channelId: Uint8Array;
  readonly keyEpoch: bigint;
  readonly ephemeralPublicKey: Uint8Array;
  readonly wrappingNonce: Uint8Array;
  readonly wrappedCmk: Uint8Array;
  readonly originatorSignature: Uint8Array;
}

/** Immutable public grant. Successful structural decode does not imply trust. */
export class PortableKeyGrant {
  readonly #originatorId: Uint8Array;
  readonly #receiverId: Uint8Array;
  readonly #channelId: Uint8Array;
  readonly keyEpoch: bigint;
  readonly #ephemeralPublicKey: Uint8Array;
  readonly #wrappingNonce: Uint8Array;
  readonly #wrappedCmk: Uint8Array;
  readonly #originatorSignature: Uint8Array;

  constructor(parts: KeyGrantParts) {
    if (!(parts.originatorId instanceof Uint8Array) || !(parts.receiverId instanceof Uint8Array)) {
      fail("invalid_field");
    }
    if (parts.originatorId.length > MAX_IDENTITY_BYTES || parts.receiverId.length > MAX_IDENTITY_BYTES) {
      fail("length_limit_exceeded");
    }
    requireLength(parts.channelId, 16);
    requireU64(parts.keyEpoch);
    requireLength(parts.ephemeralPublicKey, 32);
    requireLength(parts.wrappingNonce, 24);
    requireLength(parts.wrappedCmk, 48);
    requireLength(parts.originatorSignature, 64);
    this.#originatorId = parts.originatorId.slice();
    this.#receiverId = parts.receiverId.slice();
    this.#channelId = parts.channelId.slice();
    this.keyEpoch = parts.keyEpoch;
    this.#ephemeralPublicKey = parts.ephemeralPublicKey.slice();
    this.#wrappingNonce = parts.wrappingNonce.slice();
    this.#wrappedCmk = parts.wrappedCmk.slice();
    this.#originatorSignature = parts.originatorSignature.slice();
    Object.freeze(this);
  }

  get originatorId(): Uint8Array { return this.#originatorId.slice(); }
  get receiverId(): Uint8Array { return this.#receiverId.slice(); }
  get channelId(): Uint8Array { return this.#channelId.slice(); }
  get ephemeralPublicKey(): Uint8Array { return this.#ephemeralPublicKey.slice(); }
  get wrappingNonce(): Uint8Array { return this.#wrappingNonce.slice(); }
  get wrappedCmk(): Uint8Array { return this.#wrappedCmk.slice(); }
  get originatorSignature(): Uint8Array { return this.#originatorSignature.slice(); }
}

/** Structurally decode one complete bounded D18G v1 record. */
export function grantDeserialize(bytes: Uint8Array): PortableKeyGrant {
  const decoder = new Decoder(bytes);
  if (!equalBytes(decoder.take(4), GRANT_MAGIC)) fail("invalid_magic");
  if (decoder.u8() !== WIRE_VERSION) fail("unsupported_version");
  const originatorId = decoder.lengthPrefixedIdentity();
  const receiverId = decoder.lengthPrefixedIdentity();
  const grant = new PortableKeyGrant({
    originatorId,
    receiverId,
    channelId: decoder.take(16),
    keyEpoch: decoder.u64be(),
    ephemeralPublicKey: decoder.take(32),
    wrappingNonce: decoder.take(24),
    wrappedCmk: decoder.take(48),
    originatorSignature: decoder.take(64),
  });
  if (!decoder.done) fail("trailing_bytes");
  return grant;
}

/** Validate and encode one grant as exact D18G v1 bytes. */
export function grantSerialize(grant: PortableKeyGrant): Uint8Array {
  validateGrant(grant);
  const originatorId = grant.originatorId;
  const receiverId = grant.receiverId;
  return concat(
    GRANT_MAGIC,
    Uint8Array.of(WIRE_VERSION),
    u32be(originatorId.length), originatorId,
    u32be(receiverId.length), receiverId,
    grant.channelId,
    u64be(grant.keyEpoch),
    grant.ephemeralPublicKey,
    grant.wrappingNonce,
    grant.wrappedCmk,
    grant.originatorSignature,
  );
}

/** Seal with independently generated ephemeral private-key and nonce material. */
export function sealChannelKey(
  fields: KeyGrantFields,
  cmk: ChannelMasterKey,
  receiverPublicKey: Uint8Array,
  signingKey: OriginatorSigningKey,
  source: SecureRandomSource = systemSecureRandomSource,
): PortableKeyGrant {
  const ephemeralPrivateKey = secureRandomBytes(source, 32);
  let wrappingNonce: Uint8Array | undefined;
  try {
    wrappingNonce = secureRandomBytes(source, 24);
    return sealChannelKeyWithMaterial(
      fields, cmk, receiverPublicKey, signingKey, ephemeralPrivateKey, wrappingNonce,
    );
  } finally {
    wipe(ephemeralPrivateKey);
    if (wrappingNonce !== undefined) wipe(wrappingNonce);
  }
}

/** Seal with deterministic explicit material through the production primitive path. */
export function sealChannelKeyWithMaterial(
  fields: KeyGrantFields,
  cmk: ChannelMasterKey,
  receiverPublicKey: Uint8Array,
  signingKey: OriginatorSigningKey,
  ephemeralPrivateKey: Uint8Array,
  wrappingNonce: Uint8Array,
): PortableKeyGrant {
  requireLength(receiverPublicKey, 32);
  requireLength(ephemeralPrivateKey, 32);
  requireLength(wrappingNonce, 24);
  const originatorId = fields.originatorId;
  const receiverId = fields.receiverId;
  const channelId = fields.channelId;
  let sharedSecret: Uint8Array | undefined;
  let wrappingKey: Uint8Array | undefined;
  let cmkBytes: Uint8Array | undefined;
  try {
    let ephemeralPublicKey: Uint8Array;
    try {
      ephemeralPublicKey = x25519PublicKey(ephemeralPrivateKey);
      sharedSecret = x25519(ephemeralPrivateKey, receiverPublicKey);
    } catch {
      fail("invalid_key_agreement");
    }
    wrappingKey = deriveWrappingKey(sharedSecret, channelId, fields.keyEpoch, receiverId);
    const aad = grantAad(originatorId, receiverId, channelId, fields.keyEpoch, ephemeralPublicKey);
    cmkBytes = cmk.bytes;
    let ciphertext: Uint8Array;
    let tag: Uint8Array;
    try {
      [ciphertext, tag] = xchacha20Poly1305Encrypt(cmkBytes, wrappingKey, wrappingNonce, aad);
    } catch {
      fail("authentication_failed");
    }
    const wrappedCmk = concat(ciphertext, tag);
    const signatureInput = grantSignatureInput(
      originatorId, receiverId, channelId, fields.keyEpoch,
      ephemeralPublicKey, wrappingNonce, wrappedCmk,
    );
    return new PortableKeyGrant({
      originatorId,
      receiverId,
      channelId,
      keyEpoch: fields.keyEpoch,
      ephemeralPublicKey,
      wrappingNonce,
      wrappedCmk,
      originatorSignature: signingKey.sign(signatureInput),
    });
  } finally {
    if (sharedSecret !== undefined) wipe(sharedSecret);
    if (wrappingKey !== undefined) wipe(wrappingKey);
    if (cmkBytes !== undefined) wipe(cmkBytes);
  }
}

/** Verify all expected bindings, then unwrap and return one receiver CMK. */
export function openChannelKeyGrant(
  grant: PortableKeyGrant,
  expectedOriginatorId: Uint8Array,
  expectedReceiverId: Uint8Array,
  expectedChannelId: Uint8Array,
  receiverKeyPair: ReceiverKeyPair,
  originatorPublicKey: Uint8Array,
): ChannelMasterKey {
  validateGrant(grant);
  requireLength(expectedChannelId, 16);
  requireLength(originatorPublicKey, 32);
  if (!equalBytes(grant.originatorId, expectedOriginatorId)) fail("unexpected_originator");
  if (!equalBytes(grant.receiverId, expectedReceiverId)) fail("unexpected_receiver");
  if (!equalBytes(grant.channelId, expectedChannelId)) fail("unexpected_channel");
  const signatureInput = grantSignatureInput(
    grant.originatorId, grant.receiverId, grant.channelId, grant.keyEpoch,
    grant.ephemeralPublicKey, grant.wrappingNonce, grant.wrappedCmk,
  );
  if (!verify(signatureInput, grant.originatorSignature, originatorPublicKey)) {
    fail("invalid_signature");
  }
  let sharedSecret: Uint8Array | undefined;
  let wrappingKey: Uint8Array | undefined;
  let plaintext: Uint8Array | undefined;
  try {
    sharedSecret = receiverKeyPair.agree(grant.ephemeralPublicKey);
    wrappingKey = deriveWrappingKey(sharedSecret, grant.channelId, grant.keyEpoch, grant.receiverId);
    const wrapped = grant.wrappedCmk;
    const aad = grantAad(
      grant.originatorId, grant.receiverId, grant.channelId, grant.keyEpoch, grant.ephemeralPublicKey,
    );
    try {
      plaintext = xchacha20Poly1305Decrypt(
        wrapped.slice(0, 32), wrappingKey, grant.wrappingNonce, aad, wrapped.slice(32),
      );
    } catch {
      fail("authentication_failed");
    }
    if (plaintext.length !== 32) fail("invalid_wrapped_key");
    return ChannelMasterKey.fromBytes(plaintext);
  } finally {
    if (sharedSecret !== undefined) wipe(sharedSecret);
    if (wrappingKey !== undefined) wipe(wrappingKey);
    if (plaintext !== undefined) wipe(plaintext);
  }
}

export type GrantInstallOutcome = "installed" | "idempotent";

/** Receiver-local monotonic grant state for exactly one identity/channel tuple. */
export class ReceiverEpochKeys {
  readonly #originatorId: Uint8Array;
  readonly #receiverId: Uint8Array;
  readonly #channelId: Uint8Array;
  readonly #receiverKeyPair: ReceiverKeyPair;
  readonly #originatorPublicKey: Uint8Array;
  readonly #epochKeys = new Map<bigint, ChannelMasterKey>();
  #latestGrant: PortableKeyGrant | undefined;

  constructor(
    originatorId: Uint8Array,
    receiverId: Uint8Array,
    channelId: Uint8Array,
    receiverKeyPair: ReceiverKeyPair,
    originatorPublicKey: Uint8Array,
  ) {
    validateIdentity(originatorId);
    validateIdentity(receiverId);
    validateChannelId(channelId);
    requireLength(originatorPublicKey, 32);
    this.#originatorId = originatorId.slice();
    this.#receiverId = receiverId.slice();
    this.#channelId = channelId.slice();
    this.#receiverKeyPair = receiverKeyPair.clone();
    this.#originatorPublicKey = originatorPublicKey.slice();
  }

  get receiverPublicKey(): Uint8Array { return this.#receiverKeyPair.publicKey; }
  get latestEpoch(): bigint | undefined { return this.#latestGrant?.keyEpoch; }

  installGrant(grant: PortableKeyGrant): GrantInstallOutcome {
    const latest = this.#latestGrant;
    if (latest !== undefined) {
      if (grant.keyEpoch < latest.keyEpoch) fail("decreasing_epoch");
      if (grant.keyEpoch === latest.keyEpoch) {
        if (grantsEqual(grant, latest)) return "idempotent";
        fail("conflicting_grant");
      }
    }
    validateGrant(grant);
    const key = openChannelKeyGrant(
      grant,
      this.#originatorId,
      this.#receiverId,
      this.#channelId,
      this.#receiverKeyPair,
      this.#originatorPublicKey,
    );
    this.#epochKeys.set(grant.keyEpoch, key);
    this.#latestGrant = grant;
    return "installed";
  }

  key(epoch: bigint): ChannelMasterKey {
    requireU64(epoch);
    const key = this.#epochKeys.get(epoch);
    if (key === undefined) fail("missing_epoch_key");
    return key.clone();
  }

  destroy(): void {
    for (const key of this.#epochKeys.values()) key.destroy();
    this.#epochKeys.clear();
    this.#receiverKeyPair.destroy();
    this.#latestGrant = undefined;
  }
}

/** One authorized rotation recipient and its independent seal material. */
export class RotationReceiver {
  readonly #receiverId: Uint8Array;
  readonly #publicKey: Uint8Array;
  readonly #ephemeralPrivateKey: Uint8Array;
  readonly #wrappingNonce: Uint8Array;
  #destroyed = false;

  private constructor(
    receiverId: Uint8Array,
    publicKey: Uint8Array,
    ephemeralPrivateKey: Uint8Array,
    wrappingNonce: Uint8Array,
  ) {
    this.#receiverId = receiverId.slice();
    this.#publicKey = publicKey.slice();
    this.#ephemeralPrivateKey = ephemeralPrivateKey.slice();
    this.#wrappingNonce = wrappingNonce.slice();
    Object.freeze(this);
  }

  static withMaterial(
    receiverId: Uint8Array,
    publicKey: Uint8Array,
    ephemeralPrivateKey: Uint8Array,
    wrappingNonce: Uint8Array,
  ): RotationReceiver {
    validateIdentity(receiverId);
    requireLength(publicKey, 32);
    requireLength(ephemeralPrivateKey, 32);
    requireLength(wrappingNonce, 24);
    return new RotationReceiver(receiverId, publicKey, ephemeralPrivateKey, wrappingNonce);
  }

  static generate(
    receiverId: Uint8Array,
    publicKey: Uint8Array,
    source: SecureRandomSource = systemSecureRandomSource,
  ): RotationReceiver {
    const privateKey = secureRandomBytes(source, 32);
    let nonce: Uint8Array | undefined;
    try {
      nonce = secureRandomBytes(source, 24);
      return RotationReceiver.withMaterial(receiverId, publicKey, privateKey, nonce);
    } finally {
      wipe(privateKey);
      if (nonce !== undefined) wipe(nonce);
    }
  }

  get receiverId(): Uint8Array { return this.#receiverId.slice(); }

  seal(fields: KeyGrantFields, cmk: ChannelMasterKey, signingKey: OriginatorSigningKey): PortableKeyGrant {
    if (this.#destroyed) fail("invalid_field");
    return sealChannelKeyWithMaterial(
      fields, cmk, this.#publicKey, signingKey, this.#ephemeralPrivateKey, this.#wrappingNonce,
    );
  }

  destroy(): void {
    wipe(this.#ephemeralPrivateKey);
    this.#destroyed = true;
  }
}

/** Pure rotation result. Durable activation remains a D18P integration concern. */
export class RotationPlan {
  readonly newEpoch: bigint;
  readonly #newCmk: ChannelMasterKey;
  readonly #grants: readonly PortableKeyGrant[];

  constructor(newEpoch: bigint, newCmk: ChannelMasterKey, grants: readonly PortableKeyGrant[]) {
    this.newEpoch = newEpoch;
    this.#newCmk = newCmk.clone();
    this.#grants = Object.freeze([...grants]);
    Object.freeze(this);
  }

  get newCmk(): ChannelMasterKey { return this.#newCmk.clone(); }
  get grants(): readonly PortableKeyGrant[] { return this.#grants; }
  destroy(): void { this.#newCmk.destroy(); }
}

/** Create a complete receiver-sorted next-epoch plan or no plan at all. */
export function planRotation(
  originatorId: Uint8Array,
  channelId: Uint8Array,
  currentEpoch: bigint,
  newCmk: ChannelMasterKey,
  receivers: readonly RotationReceiver[],
  signingKey: OriginatorSigningKey,
): RotationPlan {
  validateIdentity(originatorId);
  validateChannelId(channelId);
  requireU64(currentEpoch);
  if (currentEpoch === MAX_U64) fail("epoch_exhausted");
  if (receivers.length === 0) fail("invalid_field");
  const ordered = [...receivers].sort((left, right) => compareBytes(left.receiverId, right.receiverId));
  for (let index = 1; index < ordered.length; index++) {
    if (equalBytes(ordered[index - 1]!.receiverId, ordered[index]!.receiverId)) {
      for (const receiver of ordered) receiver.destroy();
      fail("invalid_field");
    }
  }
  const grants: PortableKeyGrant[] = [];
  try {
    for (const receiver of ordered) {
      const receiverId = receiver.receiverId;
      const fields = new KeyGrantFields(originatorId, receiverId, channelId, currentEpoch + 1n);
      grants.push(receiver.seal(fields, newCmk, signingKey));
    }
    return new RotationPlan(currentEpoch + 1n, newCmk, grants);
  } finally {
    for (const receiver of ordered) receiver.destroy();
  }
}

export type SecretErasureCapability = "guaranteed" | "best_effort" | "not_enforceable";

/** TypeScript overwrites owned mutable buffers but cannot control GC copies. */
export function secretErasureCapability(): SecretErasureCapability {
  return "best_effort";
}

/** Canonical D18Q HKDF salt, exported for conformance diagnostics. */
export function keyGrantHkdfSalt(channelId: Uint8Array, keyEpoch: bigint): Uint8Array {
  requireLength(channelId, 16);
  requireU64(keyEpoch);
  return frame(channelId, u64be(keyEpoch));
}

/** Canonical D18Q HKDF info, exported for conformance diagnostics. */
export function keyGrantHkdfInfo(receiverId: Uint8Array): Uint8Array {
  if (receiverId.length > MAX_IDENTITY_BYTES) fail("length_limit_exceeded");
  return frame(KEY_WRAP_CONTEXT, receiverId);
}

/** Canonical D18Q grant AAD, exported for conformance diagnostics. */
export function keyGrantAad(grant: PortableKeyGrant): Uint8Array {
  return grantAad(
    grant.originatorId, grant.receiverId, grant.channelId,
    grant.keyEpoch, grant.ephemeralPublicKey,
  );
}

/** Canonical D18Q signature input, exported for conformance diagnostics. */
export function keyGrantSignatureInput(grant: PortableKeyGrant): Uint8Array {
  return grantSignatureInput(
    grant.originatorId, grant.receiverId, grant.channelId, grant.keyEpoch,
    grant.ephemeralPublicKey, grant.wrappingNonce, grant.wrappedCmk,
  );
}

/** Canonical receiver-specific wrapping key, exported for fixture conformance. */
export function keyGrantWrappingKey(
  sharedSecret: Uint8Array,
  channelId: Uint8Array,
  keyEpoch: bigint,
  receiverId: Uint8Array,
): Uint8Array {
  return deriveWrappingKey(sharedSecret, channelId, keyEpoch, receiverId);
}

function validateGrant(grant: PortableKeyGrant): void {
  validateIdentity(grant.originatorId);
  validateIdentity(grant.receiverId);
  validateChannelId(grant.channelId);
  requireU64(grant.keyEpoch);
}

function grantsEqual(left: PortableKeyGrant, right: PortableKeyGrant): boolean {
  return left.keyEpoch === right.keyEpoch
    && equalBytes(left.originatorId, right.originatorId)
    && equalBytes(left.receiverId, right.receiverId)
    && equalBytes(left.channelId, right.channelId)
    && equalBytes(left.ephemeralPublicKey, right.ephemeralPublicKey)
    && equalBytes(left.wrappingNonce, right.wrappingNonce)
    && equalBytes(left.wrappedCmk, right.wrappedCmk)
    && equalBytes(left.originatorSignature, right.originatorSignature);
}

function validateIdentity(identity: Uint8Array): void {
  if (!(identity instanceof Uint8Array) || identity.length === 0) fail("invalid_field");
  if (identity.length > MAX_IDENTITY_BYTES) fail("length_limit_exceeded");
}

function validateChannelId(channelId: Uint8Array): void {
  requireLength(channelId, 16);
  if ((channelId[6]! >> 4) !== 7 || (channelId[8]! >> 6) !== 2) fail("invalid_field");
}

function deriveWrappingKey(
  sharedSecret: Uint8Array,
  channelId: Uint8Array,
  keyEpoch: bigint,
  receiverId: Uint8Array,
): Uint8Array {
  requireLength(sharedSecret, 32);
  try {
    const key = hkdf(
      keyGrantHkdfSalt(channelId, keyEpoch),
      sharedSecret,
      keyGrantHkdfInfo(receiverId),
      32,
      "sha256",
    );
    if (key.length !== 32) fail("key_derivation_failed");
    return key;
  } catch (error) {
    if (error instanceof KeyGrantProfileError) throw error;
    fail("key_derivation_failed");
  }
}

function grantAad(
  originatorId: Uint8Array,
  receiverId: Uint8Array,
  channelId: Uint8Array,
  keyEpoch: bigint,
  ephemeralPublicKey: Uint8Array,
): Uint8Array {
  return frame(
    KEY_GRANT_CONTEXT, originatorId, channelId, u64be(keyEpoch), receiverId, ephemeralPublicKey,
  );
}

function grantSignatureInput(
  originatorId: Uint8Array,
  receiverId: Uint8Array,
  channelId: Uint8Array,
  keyEpoch: bigint,
  ephemeralPublicKey: Uint8Array,
  wrappingNonce: Uint8Array,
  wrappedCmk: Uint8Array,
): Uint8Array {
  return frame(
    KEY_GRANT_CONTEXT, originatorId, channelId, u64be(keyEpoch), receiverId,
    ephemeralPublicKey, wrappingNonce, wrappedCmk,
  );
}

function frame(...fields: readonly Uint8Array[]): Uint8Array {
  return concat(...fields.flatMap((field) => [u64be(BigInt(field.length)), field]));
}

function secureRandomBytes(source: SecureRandomSource, length: number): Uint8Array {
  try {
    const bytes = source.randomBytes(length);
    if (!(bytes instanceof Uint8Array) || bytes.length !== length) {
      if (bytes instanceof Uint8Array) wipe(bytes);
      fail("randomness_unavailable");
    }
    const owned = bytes.slice();
    wipe(bytes);
    return owned;
  } catch (error) {
    if (error instanceof KeyGrantProfileError && error.code === "randomness_unavailable") throw error;
    fail("randomness_unavailable");
  }
}

function requireLength(bytes: Uint8Array, length: number): void {
  if (!(bytes instanceof Uint8Array) || bytes.length !== length) fail("invalid_field");
}

function requireU64(value: bigint): void {
  if (typeof value !== "bigint" || value < 0n || value > MAX_U64) fail("invalid_field");
}

function u32be(value: number): Uint8Array {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, value, false);
  return bytes;
}

function u64be(value: bigint): Uint8Array {
  requireU64(value);
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setBigUint64(0, value, false);
  return bytes;
}

function concat(...parts: readonly Uint8Array[]): Uint8Array {
  const length = parts.reduce((sum, part) => sum + part.length, 0);
  const output = new Uint8Array(length);
  let offset = 0;
  for (const part of parts) {
    output.set(part, offset);
    offset += part.length;
  }
  return output;
}

function equalBytes(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) return false;
  let difference = 0;
  for (let index = 0; index < left.length; index++) difference |= left[index]! ^ right[index]!;
  return difference === 0;
}

function compareBytes(left: Uint8Array, right: Uint8Array): number {
  const length = Math.min(left.length, right.length);
  for (let index = 0; index < length; index++) {
    const difference = left[index]! - right[index]!;
    if (difference !== 0) return difference;
  }
  return left.length - right.length;
}

function wipe(bytes: Uint8Array): void {
  bytes.fill(0);
}

function fail(code: KeyGrantErrorCode): never {
  throw new KeyGrantProfileError(code);
}

class Decoder {
  readonly #bytes: Uint8Array;
  #offset = 0;

  constructor(bytes: Uint8Array) {
    this.#bytes = bytes;
  }

  get done(): boolean { return this.#offset === this.#bytes.length; }

  take(length: number): Uint8Array {
    if (length < 0 || this.#offset + length > this.#bytes.length) fail("truncated_record");
    const value = this.#bytes.slice(this.#offset, this.#offset + length);
    this.#offset += length;
    return value;
  }

  u8(): number { return this.take(1)[0]!; }

  u32be(): number {
    const bytes = this.take(4);
    return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(0, false);
  }

  u64be(): bigint {
    const bytes = this.take(8);
    return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getBigUint64(0, false);
  }

  lengthPrefixedIdentity(): Uint8Array {
    const length = this.u32be();
    if (length > MAX_IDENTITY_BYTES) fail("length_limit_exceeded");
    return this.take(length);
  }
}
