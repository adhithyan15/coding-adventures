/**
 * @coding-adventures/ed25519
 *
 * Ed25519 digital signature algorithm (RFC 8032) implemented from scratch.
 *
 * Ed25519 provides:
 * - 128-bit security level
 * - Deterministic signatures (no random nonce)
 * - 32-byte public keys, 64-byte signatures
 * - Fast verification
 *
 * All arithmetic uses JavaScript's native BigInt for clarity.
 */

export { generateKeypair, sign, verify, hexToBytes, bytesToHex } from "./ed25519";
