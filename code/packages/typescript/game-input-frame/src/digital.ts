/**
 * Bit-level operations over the reducer's fixed 64-button profile.
 *
 * The reducer validates each slot before calling these helpers. `set_bit` and
 * `clear_bit` preserve every unrelated level in the unsigned 64-bit mask.
 */

/** Test one already-validated button slot in the complete u64 mask. */
export function bit_is_set(mask: bigint, slot: number): boolean {
  return (mask & (1n << BigInt(slot))) !== 0n;
}

/** Set one already-validated button slot without disturbing other levels. */
export function set_bit(mask: bigint, slot: number): bigint {
  return mask | (1n << BigInt(slot));
}

/** Clear one already-validated button slot without disturbing other levels. */
export function clear_bit(mask: bigint, slot: number): bigint {
  return mask & ~(1n << BigInt(slot));
}
