export type ByteInput = Uint8Array | ArrayBuffer | ReadonlyArray<number>;

const U64_MAX = (1n << 64n) - 1n;

export function ctEq(left: ByteInput, right: ByteInput): boolean {
  const leftBytes = toBytes(left);
  const rightBytes = toBytes(right);
  if (leftBytes.length !== rightBytes.length) {
    return false;
  }

  let accumulator = 0;
  for (let index = 0; index < leftBytes.length; index += 1) {
    accumulator |= leftBytes[index] ^ rightBytes[index];
  }
  return accumulator === 0;
}

export function ctEqFixed(left: ByteInput, right: ByteInput): boolean {
  return ctEq(left, right);
}

export function ctSelectBytes(left: ByteInput, right: ByteInput, choice: boolean): Uint8Array {
  const leftBytes = toBytes(left);
  const rightBytes = toBytes(right);
  if (leftBytes.length !== rightBytes.length) {
    throw new RangeError("ctSelectBytes requires equal-length byte sequences");
  }

  const mask = choice ? 0xff : 0x00;
  const output = new Uint8Array(leftBytes.length);
  for (let index = 0; index < leftBytes.length; index += 1) {
    output[index] = rightBytes[index] ^ ((leftBytes[index] ^ rightBytes[index]) & mask);
  }
  return output;
}

export function ctEqU64(left: bigint | number, right: bigint | number): boolean {
  const leftValue = toU64(left, "left");
  const rightValue = toU64(right, "right");
  const diff = BigInt.asUintN(64, leftValue ^ rightValue);
  const folded = (diff | BigInt.asUintN(64, -diff)) >> 63n;
  return folded === 0n;
}

function toBytes(value: ByteInput): Uint8Array {
  if (value instanceof Uint8Array) {
    return value;
  }
  if (value instanceof ArrayBuffer) {
    return new Uint8Array(value);
  }
  return Uint8Array.from(value);
}

function toU64(value: bigint | number, name: string): bigint {
  const converted = typeof value === "bigint" ? value : numberToBigInt(value, name);
  if (converted < 0n || converted > U64_MAX) {
    throw new RangeError(`${name} must be an unsigned 64-bit integer`);
  }
  return converted;
}

function numberToBigInt(value: number, name: string): bigint {
  if (!Number.isSafeInteger(value)) {
    throw new RangeError(`${name} must be a safe integer or bigint`);
  }
  return BigInt(value);
}
