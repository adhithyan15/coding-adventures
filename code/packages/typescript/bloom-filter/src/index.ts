const DEFAULT_EXPECTED_ITEMS = 1_000;
const DEFAULT_FALSE_POSITIVE_RATE = 0.01;
const MASK32 = 0xffff_ffff;
const textEncoder = new TextEncoder();

export interface BloomFilterOptions {
  expectedItems?: number;
  falsePositiveRate?: number;
}

export class BloomFilter {
  private readonly bits: Uint8Array;
  private readonly expectedItems: number;
  private readonly hashes: number;
  private readonly bitTotal: number;
  private setBits = 0;
  private addedItems = 0;

  constructor(options: BloomFilterOptions = {}) {
    const expectedItems = options.expectedItems ?? DEFAULT_EXPECTED_ITEMS;
    const falsePositiveRate = options.falsePositiveRate ?? DEFAULT_FALSE_POSITIVE_RATE;
    BloomFilter.validateExpectedItems(expectedItems);
    BloomFilter.validateFalsePositiveRate(falsePositiveRate);

    const bitCount = BloomFilter.optimalM(expectedItems, falsePositiveRate);
    const hashCount = BloomFilter.optimalK(bitCount, expectedItems);
    this.bitTotal = bitCount;
    this.hashes = hashCount;
    this.expectedItems = expectedItems;
    this.bits = new Uint8Array(Math.ceil(bitCount / 8));
  }

  static fromParams(bitCount: number, hashCount: number): BloomFilter {
    BloomFilter.validateBitCount(bitCount);
    BloomFilter.validateHashCount(hashCount);

    const filter = Object.create(BloomFilter.prototype) as MutableBloomFilter;
    filter.bitTotal = bitCount;
    filter.hashes = hashCount;
    filter.expectedItems = 0;
    filter.bits = new Uint8Array(Math.ceil(bitCount / 8));
    filter.setBits = 0;
    filter.addedItems = 0;
    return filter as unknown as BloomFilter;
  }

  add(element: unknown): void {
    for (const idx of this.hashIndices(element)) {
      const byteIndex = Math.floor(idx / 8);
      const bitMask = 1 << (idx % 8);
      if ((this.bits[byteIndex]! & bitMask) === 0) {
        this.bits[byteIndex]! |= bitMask;
        this.setBits += 1;
      }
    }
    this.addedItems += 1;
  }

  contains(element: unknown): boolean {
    for (const idx of this.hashIndices(element)) {
      const byteIndex = Math.floor(idx / 8);
      const bitMask = 1 << (idx % 8);
      if ((this.bits[byteIndex]! & bitMask) === 0) {
        return false;
      }
    }
    return true;
  }

  get bitCount(): number {
    return this.bitTotal;
  }

  get hashCount(): number {
    return this.hashes;
  }

  get bitsSet(): number {
    return this.setBits;
  }

  get fillRatio(): number {
    return this.bitTotal === 0 ? 0 : this.setBits / this.bitTotal;
  }

  get estimatedFalsePositiveRate(): number {
    return this.setBits === 0 ? 0 : this.fillRatio ** this.hashes;
  }

  isOverCapacity(): boolean {
    return this.expectedItems > 0 && this.addedItems > this.expectedItems;
  }

  sizeBytes(): number {
    return this.bits.length;
  }

  toString(): string {
    const pctSet = (this.fillRatio * 100).toFixed(2);
    const estFp = (this.estimatedFalsePositiveRate * 100).toFixed(4);
    return `BloomFilter(m=${this.bitTotal}, k=${this.hashes}, bits_set=${this.setBits}/${this.bitTotal} (${pctSet}%), ~fp=${estFp}%)`;
  }

  static optimalM(expectedItems: number, falsePositiveRate: number): number {
    return Math.ceil((-expectedItems * Math.log(falsePositiveRate)) / Math.log(2) ** 2);
  }

  static optimalK(bitCount: number, expectedItems: number): number {
    return Math.max(1, Math.round((bitCount / expectedItems) * Math.log(2)));
  }

  static capacityForMemory(memoryBytes: number, falsePositiveRate: number): number {
    const bitCount = memoryBytes * 8;
    return Math.floor((-bitCount * Math.log(2) ** 2) / Math.log(falsePositiveRate));
  }

  private hashIndices(element: unknown): number[] {
    const raw = elementBytes(element);
    const h1 = fmix32(fnv1a32(raw));
    let h2 = fmix32(djb2(raw));
    h2 |= 1;

    return Array.from({ length: this.hashes }, (_, i) => {
      const idx = (h1 + i * h2) % this.bitTotal;
      return idx < 0 ? idx + this.bitTotal : idx;
    });
  }

  private static validateExpectedItems(expectedItems: number): void {
    if (!Number.isInteger(expectedItems) || expectedItems <= 0) {
      throw new RangeError(`expectedItems must be a positive integer, got ${expectedItems}`);
    }
  }

  private static validateFalsePositiveRate(falsePositiveRate: number): void {
    if (!Number.isFinite(falsePositiveRate) || falsePositiveRate <= 0 || falsePositiveRate >= 1) {
      throw new RangeError(
        `falsePositiveRate must be in the open interval (0, 1), got ${falsePositiveRate}`,
      );
    }
  }

  private static validateBitCount(bitCount: number): void {
    if (!Number.isInteger(bitCount) || bitCount <= 0) {
      throw new RangeError(`bitCount must be a positive integer, got ${bitCount}`);
    }
  }

  private static validateHashCount(hashCount: number): void {
    if (!Number.isInteger(hashCount) || hashCount <= 0) {
      throw new RangeError(`hashCount must be a positive integer, got ${hashCount}`);
    }
  }
}

type MutableBloomFilter = {
  bits: Uint8Array;
  expectedItems: number;
  hashes: number;
  bitTotal: number;
  setBits: number;
  addedItems: number;
};

function elementBytes(element: unknown): Uint8Array {
  if (typeof element === "string") {
    return textEncoder.encode(element);
  }
  try {
    return textEncoder.encode(JSON.stringify(element));
  } catch {
    return textEncoder.encode(String(element));
  }
}

function fnv1a32(bytes: Uint8Array): number {
  let hash = 0x811c_9dc5;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x0100_0193) >>> 0;
  }
  return hash >>> 0;
}

function djb2(bytes: Uint8Array): number {
  let hash = 5_381;
  for (const byte of bytes) {
    hash = (Math.imul(hash, 33) + byte) >>> 0;
  }
  return hash >>> 0;
}

function fmix32(hash: number): number {
  hash >>>= 0;
  hash ^= hash >>> 16;
  hash = Math.imul(hash, 0x85eb_ca6b) & MASK32;
  hash ^= hash >>> 13;
  hash = Math.imul(hash, 0xc2b2_ae35) & MASK32;
  hash ^= hash >>> 16;
  return hash >>> 0;
}
