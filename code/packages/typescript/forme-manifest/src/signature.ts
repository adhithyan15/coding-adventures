/**
 * signature.ts — Ed25519 sign/verify for plugin manifests (FM02 §3.2).
 *
 * The signed payload is the manifest hash (manifest-hash.ts), NOT the
 * raw manifest text.  This indirection means:
 *
 *   1. The signature is short (always 64 bytes) regardless of manifest
 *      size.
 *   2. Verification recomputes the hash from the current manifest +
 *      entry bytes; if either has been tampered with, the hash
 *      changes and verification fails.
 *   3. The wire transcript of a sign/verify never carries the entire
 *      manifest text, only its 32-byte digest.
 *
 * Public key, signature, and (when persisted) `signedAt` are stored in
 * the manifest's `[signature]` block.  The block is EXCLUDED from the
 * canonical manifest used for hashing (it's the document being signed,
 * not part of it).
 *
 * @module signature
 */

import { sign as ed25519Sign, verify as ed25519Verify } from "@coding-adventures/ed25519";
import { computeManifestHash } from "./manifest-hash.js";
import { ManifestError } from "./errors.js";
import type { Manifest, SignatureBlock } from "./manifest-types.js";

/**
 * Sign a manifest.  Returns a `SignatureBlock` ready to drop into
 * the manifest's `signature` field.
 *
 * `secretSeed` is the 32-byte Ed25519 secret seed (NOT the expanded
 * private key — generation in `@coding-adventures/ed25519` is from
 * a seed).  `publicKey` must be the matching 32-byte Ed25519
 * public-key bytes; callers compute it via
 * `generateKeypair(seed).publicKey`.
 *
 * `signedAt` defaults to the current ISO timestamp; pass an explicit
 * one for reproducible-build scenarios.  This function does NOT
 * call `Date.now()` directly when `signedAt` is supplied — the
 * caller controls determinism.
 */
export function signManifest(
  manifest: Manifest,
  entryFileBytes: Uint8Array,
  keys: { readonly secretSeed: Uint8Array; readonly publicKey: Uint8Array },
  signedAt?: string,
): SignatureBlock {
  if (!(keys.secretSeed instanceof Uint8Array) || keys.secretSeed.length !== 32) {
    throw new TypeError("signManifest: secretSeed must be a 32-byte Uint8Array");
  }
  if (!(keys.publicKey instanceof Uint8Array) || keys.publicKey.length !== 32) {
    throw new TypeError("signManifest: publicKey must be a 32-byte Uint8Array");
  }
  const hash = computeManifestHash(manifest, entryFileBytes);
  const hashBytes = new TextEncoder().encode(hash);
  // ed25519.sign expects a 64-byte secretKey = seed || publicKey
  // (per the package's `generateKeypair` shape).  Construct it here
  // so callers can pass the small {seed, publicKey} pair they
  // already manage.
  const secretKey = new Uint8Array(64);
  secretKey.set(keys.secretSeed, 0);
  secretKey.set(keys.publicKey, 32);
  const sig = ed25519Sign(hashBytes, secretKey);

  return {
    algorithm: "ed25519",
    publicKey: bytesToBase64(keys.publicKey),
    signature: bytesToBase64(sig),
    signedAt:  signedAt ?? new Date().toISOString(),
  };
}

/**
 * Verify a manifest's embedded signature.  Returns `true` if valid,
 * `false` otherwise (including: no signature, wrong algorithm,
 * malformed base64, signature doesn't verify against the manifest
 * + entry bytes).
 *
 * Does NOT throw on "no signature" or "verification failed" —
 * routine outcomes the host's load loop branches on.  Throws only on
 * malformed inputs that indicate a programming bug (non-Uint8Array
 * entry, etc.).
 */
export function verifyManifest(
  manifest: Manifest,
  entryFileBytes: Uint8Array,
): boolean {
  if (!(entryFileBytes instanceof Uint8Array)) {
    throw new TypeError("verifyManifest: entryFileBytes must be a Uint8Array");
  }
  const block = manifest.signature;
  if (!block) return false;
  if (block.algorithm !== "ed25519") return false;

  let publicKey: Uint8Array;
  let signature: Uint8Array;
  try {
    publicKey = base64ToBytes(block.publicKey);
    signature = base64ToBytes(block.signature);
  } catch {
    return false;
  }
  if (publicKey.length !== 32 || signature.length !== 64) return false;

  const hash = computeManifestHash(manifest, entryFileBytes);
  const hashBytes = new TextEncoder().encode(hash);
  try {
    return ed25519Verify(hashBytes, signature, publicKey);
  } catch {
    return false;
  }
}

/**
 * `verifyManifest` variant that throws a structured `ManifestError`
 * on failure instead of returning `false`.  Useful at install time
 * when a specific failure reason should surface in the UI.
 */
export function assertManifestSigned(
  manifest: Manifest,
  entryFileBytes: Uint8Array,
): void {
  const block = manifest.signature;
  if (!block) {
    throw new ManifestError({
      code: "SIGNATURE_FIELD_MISSING",
      message: "manifest has no [signature] block",
      path: "signature",
    });
  }
  if (block.algorithm !== "ed25519") {
    throw new ManifestError({
      code: "SIGNATURE_ALGORITHM_INVALID",
      message: `signature.algorithm "${block.algorithm}" is not supported`,
      path: "signature.algorithm",
    });
  }
  const ok = verifyManifest(manifest, entryFileBytes);
  if (!ok) {
    throw new ManifestError({
      code: "SIGNATURE_FIELD_MISSING",
      message: "signature does not verify against the manifest + entry bytes",
      path: "signature.signature",
    });
  }
}

// ─── Base64 helpers ─────────────────────────────────────────────────
//
// Bun, Deno, and modern Node all expose `Buffer` / `atob` / `btoa`;
// we use the standard `atob`/`btoa` for portability and add a small
// validation pass on decode.

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]!);
  // btoa is available in all modern JS runtimes including Node 18+.
  return btoa(binary);
}

function base64ToBytes(s: string): Uint8Array {
  if (typeof s !== "string" || s.length === 0) {
    throw new TypeError("base64ToBytes: input must be a non-empty string");
  }
  // Reject any non-base64 characters early; atob is permissive
  // about whitespace which would hide tampering.
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(s)) {
    throw new TypeError("base64ToBytes: input contains invalid base64 characters");
  }
  const binary = atob(s);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i);
  return out;
}
