const DEFAULT_PRECISION = 14;
const MIN_PRECISION = 4;
const MAX_PRECISION = 16;
const FNV_OFFSET_BASIS = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;

export class HyperLogLogError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "HyperLogLogError";
  }
}

export class HyperLogLog {
  private readonly registers: Uint8Array;
  private readonly precisionBits: number;

  constructor(precision = DEFAULT_PRECISION) {
    if (precision < MIN_PRECISION || precision > MAX_PRECISION) {
      throw new HyperLogLogError(
        `precision must be between ${MIN_PRECISION} and ${MAX_PRECISION}, got ${precision}`,
      );
    }
    this.precisionBits = precision;
    this.registers = new Uint8Array(1 << precision);
  }

  static withPrecision(precision: number): HyperLogLog {
    return new HyperLogLog(precision);
  }

  static tryWithPrecision(precision: number): HyperLogLog | null {
    try {
      return new HyperLogLog(precision);
    } catch {
      return null;
    }
  }

  add(element: unknown): this {
    this.addBytes(valueToBytes(element));
    return this;
  }

  addBytes(bytes: Uint8Array): void {
    let hash = fnv1a64(bytes);
    hash = fmix64(hash);

    const precision = BigInt(this.precisionBits);
    const bucket = Number(hash >> (64n - precision));
    const remainingBits = 64 - this.precisionBits;
    const mask = (1n << BigInt(remainingBits)) - 1n;
    const remaining = hash & mask;
    const rho = countLeadingZeros(remaining, remainingBits) + 1;

    if (rho > this.registers[bucket]) {
      this.registers[bucket] = rho;
    }
  }

  clone(): HyperLogLog {
    const clone = new HyperLogLog(this.precisionBits);
    clone.registers.set(this.registers);
    return clone;
  }

  equals(other: HyperLogLog): boolean {
    if (this.precisionBits !== other.precision()) {
      return false;
    }
    for (let i = 0; i < this.registers.length; i += 1) {
      if (this.registers[i] !== other.registers[i]) {
        return false;
      }
    }
    return true;
  }

  count(): number {
    const m = this.numRegisters();
    const zSum = this.registers.reduce((sum, register) => {
      return sum + 2 ** -register;
    }, 0);
    const alpha = alphaForRegisters(m);
    let estimate = alpha * m * m / zSum;

    if (estimate <= 2.5 * m) {
      const zeros = this.registers.reduce((count, register) => count + (register === 0 ? 1 : 0), 0);
      if (zeros > 0) {
        estimate = m * Math.log(m / zeros);
      }
    }

    const two32 = 2 ** 32;
    if (estimate > two32 / 30) {
      const ratio = 1 - estimate / two32;
      if (ratio > 0) {
        estimate = -two32 * Math.log(ratio);
      }
    }

    return Math.max(0, Math.round(estimate));
  }

  merge(other: HyperLogLog): HyperLogLog {
    const merged = this.tryMerge(other);
    if (merged === null) {
      throw new HyperLogLogError(
        `precision mismatch: ${this.precisionBits} vs ${other.precision()}`,
      );
    }
    return merged;
  }

  tryMerge(other: HyperLogLog): HyperLogLog | null {
    if (this.precisionBits !== other.precision()) {
      return null;
    }
    const merged = new HyperLogLog(this.precisionBits);
    for (let i = 0; i < this.registers.length; i += 1) {
      merged.registers[i] = Math.max(this.registers[i], other.registers[i]);
    }
    return merged;
  }

  len(): number {
    return this.count();
  }

  precision(): number {
    return this.precisionBits;
  }

  numRegisters(): number {
    return this.registers.length;
  }

  errorRate(): number {
    return HyperLogLog.errorRateForPrecision(this.precisionBits);
  }

  static errorRateForPrecision(precision: number): number {
    return 1.04 / Math.sqrt(1 << precision);
  }

  static memoryBytes(precision: number): number {
    return ((1 << precision) * 6) / 8;
  }

  static optimalPrecision(desiredError: number): number {
    const minM = (1.04 / desiredError) ** 2;
    const precision = Math.ceil(Math.log2(minM));
    return clamp(precision, MIN_PRECISION, MAX_PRECISION);
  }

  toString(): string {
    return `HyperLogLog(precision=${this.precisionBits}, registers=${this.numRegisters()}, error_rate=${(
      this.errorRate() * 100
    ).toFixed(2)}%)`;
  }
}

function valueToBytes(value: unknown): Uint8Array {
  if (value instanceof Uint8Array) {
    return new Uint8Array(value);
  }
  if (typeof value === "string") {
    return new TextEncoder().encode(value);
  }
  if (
    typeof value === "number" ||
    typeof value === "bigint" ||
    typeof value === "boolean"
  ) {
    return new TextEncoder().encode(String(value));
  }
  if (value === null || value === undefined) {
    return new TextEncoder().encode(String(value));
  }
  return new TextEncoder().encode(JSON.stringify(value) ?? String(value));
}

function fnv1a64(bytes: Uint8Array): bigint {
  let hash = FNV_OFFSET_BASIS;
  for (const byte of bytes) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * FNV_PRIME);
  }
  return BigInt.asUintN(64, hash);
}

function fmix64(k: bigint): bigint {
  let value = BigInt.asUintN(64, k);
  value ^= value >> 33n;
  value = BigInt.asUintN(64, value * 0xff51afd7ed558ccdn);
  value ^= value >> 33n;
  value = BigInt.asUintN(64, value * 0xc4ceb9fe1a85ec53n);
  value ^= value >> 33n;
  return BigInt.asUintN(64, value);
}

function countLeadingZeros(value: bigint, bitWidth: number): number {
  if (value === 0n) {
    return bitWidth;
  }
  const binary = value.toString(2);
  return bitWidth - binary.length;
}

function alphaForRegisters(registers: number): number {
  switch (registers) {
    case 16:
      return 0.673;
    case 32:
      return 0.697;
    case 64:
      return 0.709;
    default:
      return 0.7213 / (1 + 1.079 / registers);
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
