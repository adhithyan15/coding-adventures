export function bit_is_set(mask: bigint, slot: number): boolean {
  return (mask & (1n << BigInt(slot))) !== 0n;
}

export function set_bit(mask: bigint, slot: number): bigint {
  return mask | (1n << BigInt(slot));
}

export function clear_bit(mask: bigint, slot: number): bigint {
  return mask & ~(1n << BigInt(slot));
}
