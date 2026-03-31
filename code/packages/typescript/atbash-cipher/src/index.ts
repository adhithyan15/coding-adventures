/**
 * @coding-adventures/atbash-cipher
 *
 * The Atbash cipher is one of the oldest known substitution ciphers,
 * originally used with the Hebrew alphabet. It reverses the alphabet:
 * A maps to Z, B maps to Y, C maps to X, and so on.
 *
 * The formula is simple: encrypted_position = 25 - original_position
 *
 * A remarkable property is that the cipher is self-inverse: applying it
 * twice returns the original text. This means encrypt and decrypt are
 * the same operation.
 *
 * This package is part of the coding-adventures monorepo.
 */

export const VERSION = "0.1.0";

export { encrypt, decrypt } from "./cipher.js";
