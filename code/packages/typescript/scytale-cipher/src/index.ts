/**
 * @module @coding-adventures/scytale-cipher
 *
 * Scytale cipher — ancient Spartan transposition cipher.
 *
 * The Scytale (pronounced "SKIT-ah-lee") is one of the earliest known
 * transposition ciphers, used by the Spartans around 700 BCE. Unlike
 * substitution ciphers which replace characters, it rearranges their
 * positions using a columnar transposition.
 */

export { encrypt, decrypt, bruteForce } from "./cipher.js";
export type { BruteForceResult } from "./cipher.js";
