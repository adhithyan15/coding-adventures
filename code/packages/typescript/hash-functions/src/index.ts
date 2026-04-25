export type HashInput = string | Uint8Array;
export type HashValue = number | bigint;

export const FNV32_OFFSET_BASIS = 0x811c9dc5;
export const FNV32_PRIME = 0x01000193;
export const FNV64_OFFSET_BASIS = 0xcbf29ce484222325n;
export const FNV64_PRIME = 0x00000100000001b3n;
export const POLYNOMIAL_ROLLING_DEFAULT_BASE = 31n;
export const POLYNOMIAL_ROLLING_DEFAULT_MODULUS = (1n << 61n) - 1n;

const MASK32 = 0xffffffff;
const MASK64 = 0xffffffffffffffffn;
const MURMUR3_C1 = 0xcc9e2d51;
const MURMUR3_C2 = 0x1b873593;
const textEncoder = new TextEncoder();

function toBytes(data: HashInput): Uint8Array {
  return typeof data === "string" ? textEncoder.encode(data) : data;
}

function rotl32(value: number, count: number): number {
  return ((value << count) | (value >>> (32 - count))) >>> 0;
}

function fmix32(value: number): number {
  let hash = value >>> 0;
  hash ^= hash >>> 16;
  hash = Math.imul(hash, 0x85ebca6b) >>> 0;
  hash ^= hash >>> 13;
  hash = Math.imul(hash, 0xc2b2ae35) >>> 0;
  hash ^= hash >>> 16;
  return hash >>> 0;
}

function asBigInt(value: HashValue): bigint {
  return typeof value === "bigint" ? value : BigInt(value >>> 0);
}

function popcount(value: bigint): number {
  let remaining = value;
  let count = 0;
  while (remaining > 0n) {
    count += Number(remaining & 1n);
    remaining >>= 1n;
  }
  return count;
}

function deterministicBytes(sampleIndex: number): Uint8Array {
  let state = (0x9e3779b9 ^ sampleIndex) >>> 0;
  const bytes = new Uint8Array(8);
  for (let index = 0; index < bytes.length; index += 1) {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    bytes[index] = state & 0xff;
  }
  return bytes;
}

export function fnv1a32(data: HashInput): number {
  let hash = FNV32_OFFSET_BASIS >>> 0;
  for (const byte of toBytes(data)) {
    hash ^= byte;
    hash = Math.imul(hash, FNV32_PRIME) >>> 0;
  }
  return hash;
}

export function fnv1a64(data: HashInput): bigint {
  let hash = FNV64_OFFSET_BASIS;
  for (const byte of toBytes(data)) {
    hash ^= BigInt(byte);
    hash = (hash * FNV64_PRIME) & MASK64;
  }
  return hash;
}

export function djb2(data: HashInput): bigint {
  let hash = 5381n;
  for (const byte of toBytes(data)) {
    hash = (((hash << 5n) + hash) + BigInt(byte)) & MASK64;
  }
  return hash;
}

export function polynomialRolling(
  data: HashInput,
  base: bigint = POLYNOMIAL_ROLLING_DEFAULT_BASE,
  modulus: bigint = POLYNOMIAL_ROLLING_DEFAULT_MODULUS,
): bigint {
  if (modulus <= 0n) {
    throw new RangeError("modulus must be positive");
  }

  let hash = 0n;
  for (const byte of toBytes(data)) {
    hash = (hash * base + BigInt(byte)) % modulus;
  }
  return hash;
}

export function murmur3_32(data: HashInput, seed = 0): number {
  const raw = toBytes(data);
  const length = raw.length;
  let hash = seed >>> 0;
  const blockCount = length >>> 2;

  for (let blockIndex = 0; blockIndex < blockCount; blockIndex += 1) {
    const offset = blockIndex * 4;
    let k =
      raw[offset] |
      (raw[offset + 1] << 8) |
      (raw[offset + 2] << 16) |
      (raw[offset + 3] << 24);

    k = Math.imul(k, MURMUR3_C1) >>> 0;
    k = rotl32(k, 15);
    k = Math.imul(k, MURMUR3_C2) >>> 0;

    hash ^= k;
    hash = rotl32(hash, 13);
    hash = (Math.imul(hash, 5) + 0xe6546b64) >>> 0;
  }

  const tailOffset = blockCount * 4;
  let k = 0;
  switch (length & 3) {
    case 3:
      k ^= raw[tailOffset + 2] << 16;
    // fall through
    case 2:
      k ^= raw[tailOffset + 1] << 8;
    // fall through
    case 1:
      k ^= raw[tailOffset];
      k = Math.imul(k, MURMUR3_C1) >>> 0;
      k = rotl32(k, 15);
      k = Math.imul(k, MURMUR3_C2) >>> 0;
      hash ^= k;
  }

  hash ^= length;
  return fmix32(hash);
}

export function avalancheScore(
  hashFn: (data: Uint8Array) => HashValue,
  outputBits: number,
  sampleSize = 100,
): number {
  if (outputBits <= 0 || outputBits > 64) {
    throw new RangeError("outputBits must be in 1..64");
  }
  if (sampleSize <= 0) {
    throw new RangeError("sampleSize must be positive");
  }

  let totalBitFlips = 0;
  let totalTrials = 0;
  for (let sampleIndex = 0; sampleIndex < sampleSize; sampleIndex += 1) {
    const input = deterministicBytes(sampleIndex);
    const original = asBigInt(hashFn(input));

    for (let bitPosition = 0; bitPosition < input.length * 8; bitPosition += 1) {
      const flipped = new Uint8Array(input);
      flipped[bitPosition >>> 3] ^= 1 << (bitPosition & 7);
      totalBitFlips += popcount(original ^ asBigInt(hashFn(flipped)));
      totalTrials += outputBits;
    }
  }

  return totalBitFlips / totalTrials;
}

export function distributionTest(
  hashFn: (data: Uint8Array) => HashValue,
  inputs: HashInput[],
  numBuckets: number,
): number {
  if (numBuckets <= 0) {
    throw new RangeError("numBuckets must be positive");
  }
  if (inputs.length === 0) {
    throw new RangeError("inputs must not be empty");
  }

  const counts = new Array<number>(numBuckets).fill(0);
  for (const input of inputs) {
    const bucket = Number(asBigInt(hashFn(toBytes(input))) % BigInt(numBuckets));
    counts[bucket] += 1;
  }

  const expected = inputs.length / numBuckets;
  return counts.reduce((sum, observed) => {
    const delta = observed - expected;
    return sum + (delta * delta) / expected;
  }, 0);
}
