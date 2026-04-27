// ============================================================================
// index.ts — Public API for @coding-adventures/x25519
// ============================================================================
//
// This module re-exports the three public functions of the X25519 package:
//
//   x25519(scalar, u)       — generic scalar multiplication on Curve25519
//   x25519Base(scalar)      — multiply scalar by the base point (u=9)
//   generateKeypair(privKey) — alias for x25519Base (generate public key)
//
// All inputs and outputs are 32-byte Uint8Arrays in little-endian encoding.
// ============================================================================

export { x25519, x25519Base, generateKeypair } from "./x25519.js";
