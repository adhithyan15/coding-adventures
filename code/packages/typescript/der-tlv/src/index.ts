/** Bounded, payload-blind DER tag-length-value framing. */

export const VERSION = "0.1.0";
export const DEFAULT_MAX_INPUT_LEN = 1024 * 1024;
export const DEFAULT_MAX_VALUE_LEN = 1024n * 1024n;
export const DEFAULT_MAX_ELEMENTS = 4096;
const U32_MAX = 0xffff_ffffn;
const HOST_MAX = BigInt(Number.MAX_SAFE_INTEGER);
const TYPED_ARRAY_PROTOTYPE = Object.getPrototypeOf(Uint8Array.prototype) as object;

function typedArrayGetter(name: "buffer" | "byteLength" | "byteOffset"): (this: Uint8Array) => unknown {
  const getter = Object.getOwnPropertyDescriptor(TYPED_ARRAY_PROTOTYPE, name)?.get;
  if (getter === undefined) throw new Error(`missing typed-array ${name} intrinsic`);
  return getter as (this: Uint8Array) => unknown;
}

const GET_TYPED_ARRAY_BUFFER = typedArrayGetter("buffer");
const GET_TYPED_ARRAY_BYTE_LENGTH = typedArrayGetter("byteLength");
const GET_TYPED_ARRAY_BYTE_OFFSET = typedArrayGetter("byteOffset");

export type TagClass = "universal" | "application" | "context-specific" | "private";

export const DerErrorKind = {
  EmptyInput: "empty-input",
  TruncatedHighTag: "truncated-high-tag",
  TruncatedLength: "truncated-length",
  TruncatedValue: "truncated-value",
  EndOfContents: "end-of-contents",
  NonMinimalTag: "non-minimal-tag",
  TagOverflow: "tag-overflow",
  IndefiniteLength: "indefinite-length",
  ReservedLength: "reserved-length",
  NonMinimalLength: "non-minimal-length",
  LengthTooWide: "length-too-wide",
  LengthHostOverflow: "length-host-overflow",
  InputLimitExceeded: "input-limit-exceeded",
  ValueLimitExceeded: "value-limit-exceeded",
  ElementLimitExceeded: "element-limit-exceeded",
  TagLimitExceeded: "tag-limit-exceeded",
  TrailingData: "trailing-data",
} as const;

export type DerErrorKind = (typeof DerErrorKind)[keyof typeof DerErrorKind];

export class DerError extends Error {
  readonly kind: DerErrorKind;
  readonly offset: number;

  constructor(kind: DerErrorKind, offset: number) {
    super(`DER framing error ${kind} at byte ${offset}`);
    this.name = "DerError";
    this.kind = kind;
    this.offset = offset;
  }
}

export interface DerLimits {
  readonly maxInputLen: number;
  readonly maxValueLen: bigint;
  readonly maxElements: number;
  readonly maxTagNumber: number;
}

export function defaultLimits(): DerLimits {
  return Object.freeze({
    maxInputLen: DEFAULT_MAX_INPUT_LEN,
    maxValueLen: DEFAULT_MAX_VALUE_LEN,
    maxElements: DEFAULT_MAX_ELEMENTS,
    maxTagNumber: 0xffff_ffff,
  });
}

function normalizeLimits(limits: DerLimits): DerLimits {
  const { maxInputLen, maxValueLen, maxElements, maxTagNumber } = limits;
  const numericLimits: readonly [keyof DerLimits, number, number][] = [
    ["maxInputLen", maxInputLen, Number.MAX_SAFE_INTEGER],
    ["maxElements", maxElements, Number.MAX_SAFE_INTEGER],
    ["maxTagNumber", maxTagNumber, Number(U32_MAX)],
  ];
  for (const [name, value, maximum] of numericLimits) {
    if (!Number.isSafeInteger(value) || value < 0 || value > maximum) {
      throw new RangeError(`${name} must be a non-negative safe integer no greater than ${maximum}`);
    }
  }
  if (typeof maxValueLen !== "bigint" || maxValueLen < 0n) {
    throw new RangeError("maxValueLen must be a non-negative bigint");
  }
  return Object.freeze({
    maxInputLen,
    maxValueLen,
    maxElements,
    maxTagNumber,
  });
}

function fixedInputView(input: Uint8Array): Uint8Array {
  const buffer = Reflect.apply(GET_TYPED_ARRAY_BUFFER, input, []) as ArrayBufferLike;
  const byteLength = Reflect.apply(GET_TYPED_ARRAY_BYTE_LENGTH, input, []) as number;
  const byteOffset = Reflect.apply(GET_TYPED_ARRAY_BYTE_OFFSET, input, []) as number;
  return new Uint8Array(buffer, byteOffset, byteLength);
}

export interface DerTag {
  readonly class: TagClass;
  readonly constructed: boolean;
  readonly number: number;
}

export class DerElement {
  readonly tag: DerTag;
  readonly #input: Uint8Array;
  readonly #start: number;
  readonly #headerLen: number;
  readonly #encodedLen: number;

  constructor(tag: DerTag, input: Uint8Array, start: number, headerLen: number, encodedLen: number) {
    this.tag = tag;
    this.#input = input;
    this.#start = start;
    this.#headerLen = headerLen;
    this.#encodedLen = encodedLen;
  }

  get header(): Uint8Array {
    return this.#input.subarray(this.#start, this.#start + this.#headerLen);
  }

  get value(): Uint8Array {
    return this.#input.subarray(this.#start + this.#headerLen, this.#start + this.#encodedLen);
  }

  get encoded(): Uint8Array {
    return this.#input.subarray(this.#start, this.#start + this.#encodedLen);
  }
}

interface Decoded {
  readonly element: DerElement;
  readonly nextOffset: number;
}

function decodeAt(input: Uint8Array, start: number, available: number, limits: DerLimits): Decoded {
  if (available > limits.maxInputLen) throw new DerError(DerErrorKind.InputLimitExceeded, start);
  if (available === 0) throw new DerError(DerErrorKind.EmptyInput, start);

  const first = input[start]!;
  const classes: readonly TagClass[] = ["universal", "application", "context-specific", "private"];
  const tagClass = classes[first >> 6]!;
  const constructed = (first & 0x20) !== 0;
  const low = first & 0x1f;
  let number: bigint;
  let identifierLen = 1;
  if (low !== 0x1f) {
    number = BigInt(low);
    if (number > BigInt(limits.maxTagNumber)) throw new DerError(DerErrorKind.TagLimitExceeded, start);
  } else {
    number = 0n;
    let index = 1;
    for (;;) {
      if (index >= available) throw new DerError(DerErrorKind.TruncatedHighTag, start + index);
      const octet = input[start + index]!;
      const payload = BigInt(octet & 0x7f);
      if (index === 1 && payload === 0n) throw new DerError(DerErrorKind.NonMinimalTag, start + index);
      const candidate = number * 128n + payload;
      if (candidate > U32_MAX) throw new DerError(DerErrorKind.TagOverflow, start + index);
      number = candidate;
      if (number > BigInt(limits.maxTagNumber)) throw new DerError(DerErrorKind.TagLimitExceeded, start + index);
      index += 1;
      if ((octet & 0x80) === 0) break;
    }
    if (number < 31n) throw new DerError(DerErrorKind.NonMinimalTag, start);
    identifierLen = index;
  }

  if (tagClass === "universal" && number === 0n) throw new DerError(DerErrorKind.EndOfContents, start);
  const lengthOffset = start + identifierLen;
  if (identifierLen >= available) throw new DerError(DerErrorKind.TruncatedLength, lengthOffset);
  const firstLength = input[lengthOffset]!;
  let valueLen: bigint;
  let lengthLen = 1;
  if (firstLength < 0x80) {
    valueLen = BigInt(firstLength);
  } else {
    if (firstLength === 0x80) throw new DerError(DerErrorKind.IndefiniteLength, lengthOffset);
    if (firstLength === 0xff) throw new DerError(DerErrorKind.ReservedLength, lengthOffset);
    const count = firstLength & 0x7f;
    if (count > 8) throw new DerError(DerErrorKind.LengthTooWide, lengthOffset);
    const valueStart = identifierLen + 1;
    const valueEnd = valueStart + count;
    if (valueEnd > available) throw new DerError(DerErrorKind.TruncatedLength, start + available);
    if (input[start + valueStart] === 0) throw new DerError(DerErrorKind.NonMinimalLength, start + valueStart);
    valueLen = 0n;
    for (let index = valueStart; index < valueEnd; index += 1) {
      valueLen = valueLen * 256n + BigInt(input[start + index]!);
    }
    if (valueLen < 128n) throw new DerError(DerErrorKind.NonMinimalLength, lengthOffset);
    lengthLen = 1 + count;
  }

  if (valueLen > HOST_MAX) throw new DerError(DerErrorKind.LengthHostOverflow, lengthOffset);
  if (valueLen > limits.maxValueLen) throw new DerError(DerErrorKind.ValueLimitExceeded, lengthOffset);
  const headerLen = identifierLen + lengthLen;
  if (valueLen > HOST_MAX - BigInt(headerLen)) throw new DerError(DerErrorKind.LengthHostOverflow, lengthOffset);
  const encodedLen = headerLen + Number(valueLen);
  if (encodedLen > available) throw new DerError(DerErrorKind.TruncatedValue, start + available);
  const element = new DerElement(
    { class: tagClass, constructed, number: Number(number) },
    input,
    start,
    headerLen,
    encodedLen,
  );
  return { element, nextOffset: start + encodedLen };
}

export function decodeOne(input: Uint8Array, limits: DerLimits = defaultLimits()): [DerElement, Uint8Array] {
  const fixedInput = fixedInputView(input);
  const decoded = decodeAt(fixedInput, 0, fixedInput.length, normalizeLimits(limits));
  return [decoded.element, fixedInput.subarray(decoded.nextOffset)];
}

export function decodeExact(input: Uint8Array, limits: DerLimits = defaultLimits()): DerElement {
  const fixedInput = fixedInputView(input);
  const decoded = decodeAt(fixedInput, 0, fixedInput.length, normalizeLimits(limits));
  if (decoded.nextOffset !== fixedInput.length) throw new DerError(DerErrorKind.TrailingData, decoded.nextOffset);
  return decoded.element;
}

export class DerCursor {
  readonly #input: Uint8Array;
  readonly #inputLen: number;
  readonly #limits: DerLimits;
  #offset = 0;
  #elementsRead = 0;

  constructor(input: Uint8Array, limits: DerLimits = defaultLimits()) {
    const normalizedLimits = normalizeLimits(limits);
    const fixedInput = fixedInputView(input);
    const inputLen = fixedInput.length;
    if (inputLen > normalizedLimits.maxInputLen) throw new DerError(DerErrorKind.InputLimitExceeded, 0);
    this.#input = fixedInput;
    this.#inputLen = inputLen;
    this.#limits = normalizedLimits;
  }

  get elementsRead(): number { return this.#elementsRead; }
  get remaining(): Uint8Array { return this.#input.subarray(this.#offset, this.#inputLen); }

  read(): DerElement | undefined {
    if (this.#offset === this.#inputLen) return undefined;
    if (this.#elementsRead >= this.#limits.maxElements) throw new DerError(DerErrorKind.ElementLimitExceeded, this.#offset);
    const decoded = decodeAt(this.#input, this.#offset, this.#inputLen - this.#offset, this.#limits);
    this.#offset = decoded.nextOffset;
    this.#elementsRead += 1;
    return decoded.element;
  }

  finish(): void {
    if (this.#offset !== this.#inputLen) throw new DerError(DerErrorKind.TrailingData, this.#offset);
  }
}
