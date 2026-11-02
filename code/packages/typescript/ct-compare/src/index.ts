/**
 * # Constant-time comparison and selection (CT01, TypeScript lane)
 *
 * These helpers compare and select *secret* bytes without letting the time
 * they take depend on those bytes. The specification is
 * `code/specs/CT01-constant-time-compare.md`; read it for the full contract.
 *
 * ## Why validation comes first
 *
 * A constant-time comparison is only as strong as the bytes it compares. The
 * first version of this lane accepted any array and converted it with
 * `Uint8Array.from`, which *silently rewrites* bad input:
 *
 * | Input element | `Uint8Array.from` stores |
 * |---------------|--------------------------|
 * | `256`         | `0` (wraps modulo 256)   |
 * | `-1`          | `255`                    |
 * | `NaN`, `"a"`  | `0`                      |
 * | `1.9`         | `1` (truncates)          |
 *
 * Two *different* inputs could therefore become the *same* bytes and compare
 * equal, which defeats the whole purpose of a MAC or token check. So every
 * input is now validated, and anything that is not already a byte sequence is
 * rejected with a `TypeError` or `RangeError` instead of being coerced. The
 * checks look only at the *type* and *range* of each element, never at
 * whether two inputs match, so they reveal nothing about secret contents for
 * valid inputs.
 *
 * ## Why the selection mask is computed arithmetically
 *
 * `ctSelectBytes` chooses `left` or `right` with a mask that is all ones or all
 * zeros. CT01 forbids a branch on the secret `choice` bit, so the mask is
 * computed as `-Number(choice) & 0xff` instead of `choice ? 0xff : 0x00`:
 *
 * | `choice` | `Number(choice)` | negated | `& 0xff` |
 * |----------|------------------|---------|----------|
 * | `false`  | `0`              | `-0`    | `0x00`   |
 * | `true`   | `1`              | `-1`    | `0xff`   |
 *
 * The JavaScript engine may still compile this however it likes; CT01 does
 * not promise that a JIT preserves the source's shape, and neither does this
 * lane. What the source guarantees is that it contains no secret branch.
 */

/** The byte-sequence shapes this lane accepts. Anything else is rejected. */
export type ByteInput = Uint8Array | ArrayBuffer | ReadonlyArray<number>;

const U64_MAX = (1n << 64n) - 1n;

/**
 * Compare two byte sequences. Lengths are public: a length mismatch returns
 * `false` at once. For equal lengths every byte is folded into an XOR
 * accumulator that is inspected only after the loop.
 */
export function ctEq(left: ByteInput, right: ByteInput): boolean {
  const leftBytes = toBytes(left, "left");
  const rightBytes = toBytes(right, "right");
  if (leftBytes.length !== rightBytes.length) {
    return false;
  }

  let accumulator = 0;
  for (let index = 0; index < leftBytes.length; index += 1) {
    accumulator |= leftBytes[index] ^ rightBytes[index];
  }
  return accumulator === 0;
}

/** Fixed-width equality. JavaScript has no fixed-size arrays, so this is `ctEq`. */
export function ctEqFixed(left: ByteInput, right: ByteInput): boolean {
  return ctEq(left, right);
}

/**
 * Return a fresh copy of `left` when `choice` is true, otherwise of `right`,
 * computed as `right ^ ((left ^ right) & mask)` for every byte.
 */
export function ctSelectBytes(left: ByteInput, right: ByteInput, choice: boolean): Uint8Array {
  const leftBytes = toBytes(left, "left");
  const rightBytes = toBytes(right, "right");
  if (leftBytes.length !== rightBytes.length) {
    throw new RangeError("ctSelectBytes requires equal-length byte sequences");
  }
  // Truthiness would make "0", "false" and [] select `left`. Only a real
  // boolean is a valid choice; checking its *type* reveals nothing about it.
  if (typeof choice !== "boolean") {
    throw new TypeError("ctSelectBytes requires a boolean choice");
  }

  const mask = -Number(choice) & 0xff;
  const output = new Uint8Array(leftBytes.length);
  for (let index = 0; index < leftBytes.length; index += 1) {
    output[index] = rightBytes[index] ^ ((leftBytes[index] ^ rightBytes[index]) & mask);
  }
  return output;
}

/**
 * Compare two unsigned 64-bit values by folding their difference:
 * `(d | -d) >> 63` is 0 exactly when `d` is 0, with no comparison on the value.
 */
export function ctEqU64(left: bigint | number, right: bigint | number): boolean {
  const leftValue = toU64(left, "left");
  const rightValue = toU64(right, "right");
  const diff = BigInt.asUintN(64, leftValue ^ rightValue);
  const folded = (diff | BigInt.asUintN(64, -diff)) >> 63n;
  return folded === 0n;
}

/*
 * ## Recognising real byte arrays
 *
 * `value instanceof Uint8Array` only checks the prototype chain, which any
 * object can fake (`Object.setPrototypeOf({ length: 1 }, Uint8Array.prototype)`),
 * and a subclass or `Proxy` passes it while serving whatever `length` and
 * elements it likes. So the checks below use the engine's own *intrinsic*
 * getters, captured once when the module loads, which work only on genuine
 * objects:
 *
 * - `%TypedArray%.prototype[Symbol.toStringTag]` returns `"Uint8Array"` only
 *   for a real `Uint8Array` (including Node's `Buffer` and arrays from another
 *   realm), and `undefined` for fakes and proxies;
 * - `%TypedArray%.prototype.buffer/byteOffset/byteLength` read the internal
 *   slots, so a subclass's overridden `length` getter is never consulted;
 * - `ArrayBuffer.prototype.byteLength` throws for anything that is not a real
 *   `ArrayBuffer`.
 *
 * Every accepted input is then read through a *fresh* view built from those
 * internal slots.
 */
const TypedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype) as object;
// Every conforming engine defines these accessors, so the descriptors exist.
const intrinsicGetter = (target: object, key: PropertyKey): ((this: unknown) => unknown) =>
  Object.getOwnPropertyDescriptor(target, key)!.get!;
const typedArrayTag = intrinsicGetter(TypedArrayPrototype, Symbol.toStringTag);
const typedArrayBuffer = intrinsicGetter(TypedArrayPrototype, "buffer");
const typedArrayByteOffset = intrinsicGetter(TypedArrayPrototype, "byteOffset");
const typedArrayByteLength = intrinsicGetter(TypedArrayPrototype, "byteLength");
const arrayBufferByteLength = intrinsicGetter(ArrayBuffer.prototype, "byteLength");

function isRealArrayBuffer(value: unknown): value is ArrayBuffer {
  try {
    arrayBufferByteLength.call(value);
    return true;
  } catch {
    return false;
  }
}

/**
 * Turn an accepted input into bytes, rejecting anything that would need
 * coercion. A genuine `Uint8Array` (including Node's `Buffer`) and a genuine
 * `ArrayBuffer` are bytes already; a plain array must hold integers 0 to 255
 * and nothing else. Strings, other typed arrays, fakes, proxies over typed
 * arrays, and arbitrary iterables are rejected.
 *
 * Note that a whole `ArrayBuffer` is used: `Buffer.from("a").buffer` is Node's
 * shared allocation pool, not the one byte you meant. Pass the `Buffer` itself.
 */
function toBytes(value: ByteInput, name: string): Uint8Array {
  if (typedArrayTag.call(value) === "Uint8Array") {
    return new Uint8Array(
      typedArrayBuffer.call(value) as ArrayBuffer,
      typedArrayByteOffset.call(value) as number,
      typedArrayByteLength.call(value) as number,
    );
  }
  if (isRealArrayBuffer(value)) {
    return new Uint8Array(value);
  }
  if (!Array.isArray(value)) {
    throw new TypeError(`${name} must be a Uint8Array, an ArrayBuffer, or an array of byte values`);
  }
  // Read the length exactly once. A getter on an element could otherwise
  // shrink or grow the array while it is being copied.
  const length: number = value.length;
  const bytes = new Uint8Array(length);
  for (let index = 0; index < length; index += 1) {
    const element: unknown = value[index];
    if (typeof element !== "number" || !Number.isInteger(element) || element < 0 || element > 255) {
      throw new RangeError(`${name}[${index}] must be an integer from 0 to 255`);
    }
    bytes[index] = element;
  }
  return bytes;
}

function toU64(value: bigint | number, name: string): bigint {
  const converted = typeof value === "bigint" ? value : numberToBigInt(value, name);
  if (converted < 0n || converted > U64_MAX) {
    throw new RangeError(`${name} must be an unsigned 64-bit integer`);
  }
  return converted;
}

function numberToBigInt(value: number, name: string): bigint {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    throw new RangeError(`${name} must be a safe integer or bigint`);
  }
  return BigInt(value);
}
