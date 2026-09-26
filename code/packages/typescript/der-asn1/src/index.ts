/** Bounded typed ASN.1 DER values over payload-blind DER framing. */

import {
  DerCursor,
  type DerElement,
  DerError,
  type DerErrorKind,
  type DerLimits,
  type DerTag,
  decodeExact as decodeDerExact,
  defaultLimits as defaultDerLimits,
} from "@coding-adventures/der-tlv";

export const VERSION = "0.1.0";
export const DEFAULT_MAX_DEPTH = 32;
export const DEFAULT_MAX_TOTAL_ELEMENTS = 16_384;
export const DEFAULT_MAX_OID_ARCS = 128;
const U64_MAX = (1n << 64n) - 1n;

export interface Asn1Limits {
  readonly der: DerLimits;
  readonly maxDepth: number;
  readonly maxTotalElements: number;
  readonly maxOidArcs: number;
}

export function defaultAsn1Limits(): Asn1Limits {
  return Object.freeze({
    der: defaultDerLimits(),
    maxDepth: DEFAULT_MAX_DEPTH,
    maxTotalElements: DEFAULT_MAX_TOTAL_ELEMENTS,
    maxOidArcs: DEFAULT_MAX_OID_ARCS,
  });
}

function checkedInteger(name: string, value: number): number {
  if (!Number.isSafeInteger(value) || value < 0) throw new RangeError(`${name} must be a non-negative safe integer`);
  return value;
}

function normalizeLimits(limits: Asn1Limits): Asn1Limits {
  const der = Object.freeze({ ...limits.der });
  // DER-TLV performs the complete type and bound validation. Constructing an
  // empty cursor validates the snapshot without adding public raw helpers.
  new DerCursor(new Uint8Array(), der);
  return Object.freeze({
    der,
    maxDepth: checkedInteger("maxDepth", limits.maxDepth),
    maxTotalElements: checkedInteger("maxTotalElements", limits.maxTotalElements),
    maxOidArcs: checkedInteger("maxOidArcs", limits.maxOidArcs),
  });
}

function limitsEqual(left: Asn1Limits, right: Asn1Limits): boolean {
  return left.maxDepth === right.maxDepth
    && left.maxTotalElements === right.maxTotalElements
    && left.maxOidArcs === right.maxOidArcs
    && left.der.maxInputLen === right.der.maxInputLen
    && left.der.maxValueLen === right.der.maxValueLen
    && left.der.maxElements === right.der.maxElements
    && left.der.maxTagNumber === right.der.maxTagNumber;
}

export const Asn1ErrorKind = Object.freeze({
  Framing: "framing",
  UnexpectedTag: "unexpected-tag",
  DecoderLimitMismatch: "decoder-limit-mismatch",
  DepthLimitExceeded: "depth-limit-exceeded",
  ElementLimitExceeded: "element-limit-exceeded",
  InvalidBooleanLength: "invalid-boolean-length",
  InvalidBooleanValue: "invalid-boolean-value",
  EmptyInteger: "empty-integer",
  NonMinimalInteger: "non-minimal-integer",
  NegativeInteger: "negative-integer",
  IntegerOverflow: "integer-overflow",
  MissingUnusedBitCount: "missing-unused-bit-count",
  InvalidUnusedBitCount: "invalid-unused-bit-count",
  NonZeroBitPadding: "non-zero-bit-padding",
  BitLengthOverflow: "bit-length-overflow",
  NonEmptyNull: "non-empty-null",
  NonAsciiIa5String: "non-ascii-ia5-string",
  EmptyObjectIdentifier: "empty-object-identifier",
  UnterminatedObjectIdentifier: "unterminated-object-identifier",
  NonMinimalObjectIdentifier: "non-minimal-object-identifier",
  ObjectIdentifierOverflow: "object-identifier-overflow",
  OidArcLimitExceeded: "oid-arc-limit-exceeded",
} as const);

export type Asn1ErrorKind = (typeof Asn1ErrorKind)[keyof typeof Asn1ErrorKind];

export class Asn1Error extends Error {
  readonly kind: Asn1ErrorKind;
  readonly offset: number;
  readonly framingKind?: DerErrorKind;

  constructor(kind: Asn1ErrorKind, offset: number, framingKind?: DerErrorKind) {
    super(`ASN.1 DER value error ${kind} at byte ${offset}`);
    this.name = "Asn1Error";
    this.kind = kind;
    this.offset = offset;
    this.framingKind = framingKind;
  }
}

function fail(kind: Asn1ErrorKind, offset: number): never {
  throw new Asn1Error(kind, offset);
}

function framing(error: DerError): Asn1Error {
  return new Asn1Error(Asn1ErrorKind.Framing, error.offset, error.kind);
}

const ELEMENT_TOKEN = {};

interface ElementState {
  readonly tag: DerTag;
  readonly depth: number;
  readonly header: Uint8Array;
  readonly value: Uint8Array;
  readonly encoded: Uint8Array;
  readonly valueOffset: number;
}

let inspectElement: (element: Asn1Element) => ElementState;

export class Asn1Element {
  readonly #tag: DerTag;
  readonly #depth: number;
  readonly #header: Uint8Array;
  readonly #value: Uint8Array;
  readonly #encoded: Uint8Array;

  constructor(token: object, element: DerElement, depth: number) {
    if (token !== ELEMENT_TOKEN) throw new TypeError("Asn1Element values are created by Asn1Decoder");
    this.#tag = Object.freeze({ ...element.tag });
    this.#depth = depth;
    this.#header = Uint8Array.from(element.header);
    this.#value = Uint8Array.from(element.value);
    this.#encoded = Uint8Array.from(element.encoded);
  }

  get tag(): DerTag { return this.#tag; }
  get depth(): number { return this.#depth; }
  get header(): Uint8Array { return this.#header.slice(); }
  get value(): Uint8Array { return this.#value.slice(); }
  get encoded(): Uint8Array { return this.#encoded.slice(); }
  get valueOffset(): number { return this.#header.length; }

  static {
    inspectElement = (element: Asn1Element): ElementState => {
      if (typeof element !== "object" || element === null || !(#value in element)) {
        fail(Asn1ErrorKind.UnexpectedTag, 0);
      }
      return {
        tag: element.#tag,
        depth: element.#depth,
        header: element.#header,
        value: element.#value,
        encoded: element.#encoded,
        valueOffset: element.#header.length,
      };
    };
  }
}

function elementOf(element: DerElement, depth: number): Asn1Element {
  return new Asn1Element(ELEMENT_TOKEN, element, depth);
}

export class Asn1Decoder {
  readonly limits: Asn1Limits;
  #elementsRead = 0;

  constructor(limits: Asn1Limits = defaultAsn1Limits()) {
    this.limits = normalizeLimits(limits);
  }

  get elementsRead(): number { return this.#elementsRead; }

  decodeExact(input: Uint8Array): Asn1Element {
    if (this.limits.maxDepth === 0) fail(Asn1ErrorKind.DepthLimitExceeded, 0);
    this.requireCapacity(0);
    try {
      const element = decodeDerExact(input, this.limits.der);
      this.#elementsRead += 1;
      return elementOf(element, 0);
    } catch (error: unknown) {
      if (error instanceof DerError) throw framing(error);
      /* v8 ignore next -- preserve unexpected dependency failures */
      throw error;
    }
  }

  sequence(element: Asn1Element): Asn1Cursor { return this.constructed(element, "universal", 16); }
  set(element: Asn1Element): Asn1Cursor { return this.constructed(element, "universal", 17); }

  explicit(element: Asn1Element, tagNumber: number): Asn1Element {
    const state = expectTag(element, "context-specific", true, tagNumber);
    const depth = this.childDepth(state);
    this.requireCapacity(state.valueOffset);
    try {
      const child = decodeDerExact(state.value, this.limits.der);
      this.#elementsRead += 1;
      return elementOf(child, depth);
    } catch (error: unknown) {
      if (error instanceof DerError) throw framing(error);
      /* v8 ignore next -- preserve unexpected dependency failures */
      throw error;
    }
  }

  private constructed(element: Asn1Element, tagClass: DerTag["class"], number: number): Asn1Cursor {
    const state = expectTag(element, tagClass, true, number);
    const depth = this.childDepth(state);
    try { return new Asn1Cursor(ELEMENT_TOKEN, new DerCursor(state.value, this.limits.der), depth, this.limits); }
    catch (error: unknown) {
      /* v8 ignore next 2 -- DerCursor documents DerError as its only failure */
      if (error instanceof DerError) throw framing(error);
      throw error;
    }
  }

  private childDepth(element: ElementState): number {
    const depth = element.depth + 1;
    if (depth >= this.limits.maxDepth) fail(Asn1ErrorKind.DepthLimitExceeded, 0);
    return depth;
  }

  private requireCapacity(offset: number): void {
    if (this.#elementsRead >= this.limits.maxTotalElements) fail(Asn1ErrorKind.ElementLimitExceeded, offset);
  }

  static readCursor(
    token: object,
    raw: DerCursor,
    childDepth: number,
    limits: Asn1Limits,
    decoder: Asn1Decoder,
  ): Asn1Element | undefined {
    if (token !== ELEMENT_TOKEN) throw new TypeError("cursor authority is internal");
    if (raw.remaining.length === 0) return undefined;
    if (!limitsEqual(decoder.limits, limits)) fail(Asn1ErrorKind.DecoderLimitMismatch, 0);
    decoder.requireCapacity(0);
    try {
      const element = raw.read();
      if (element === undefined) return undefined;
      decoder.#elementsRead += 1;
      return elementOf(element, childDepth);
    } catch (error: unknown) {
      if (error instanceof DerError) throw framing(error);
      /* v8 ignore next -- preserve unexpected dependency failures */
      throw error;
    }
  }
}

export class Asn1Cursor {
  readonly #raw: DerCursor;
  readonly #childDepth: number;
  readonly #limits: Asn1Limits;

  constructor(token: object, cursor: DerCursor, childDepth: number, limits: Asn1Limits) {
    if (token !== ELEMENT_TOKEN) throw new TypeError("Asn1Cursor values are created by Asn1Decoder");
    this.#raw = cursor;
    this.#childDepth = childDepth;
    this.#limits = limits;
  }

  get remaining(): Uint8Array { return this.#raw.remaining.slice(); }
  read(decoder: Asn1Decoder): Asn1Element | undefined {
    return Asn1Decoder.readCursor(ELEMENT_TOKEN, this.#raw, this.#childDepth, this.#limits, decoder);
  }
  finish(): void {
    try { this.#raw.finish(); }
    catch (error: unknown) {
      /* v8 ignore next 2 -- DerCursor documents DerError as its only failure */
      if (error instanceof DerError) throw framing(error);
      throw error;
    }
  }
}

const INTEGER_TOKEN = {};
export class DerInteger {
  readonly #signedBytes: Uint8Array;
  readonly #valueOffset: number;

  constructor(token: object, signedBytes: Uint8Array, valueOffset: number) {
    if (token !== INTEGER_TOKEN) throw new TypeError("DerInteger values are created by decodeInteger");
    this.#signedBytes = Uint8Array.from(signedBytes);
    this.#valueOffset = valueOffset;
  }

  get signedBytes(): Uint8Array { return this.#signedBytes.slice(); }
  get isNegative(): boolean { return (this.#signedBytes[0]! & 0x80) !== 0; }
  toU64(): bigint {
    if (this.isNegative) fail(Asn1ErrorKind.NegativeInteger, this.#valueOffset);
    const start = this.#signedBytes[0] === 0 ? 1 : 0;
    if (this.#signedBytes.length - start > 8) fail(Asn1ErrorKind.IntegerOverflow, this.#valueOffset);
    let value = 0n;
    for (let index = start; index < this.#signedBytes.length; index += 1) value = value * 256n + BigInt(this.#signedBytes[index]!);
    return value;
  }
}

const BIT_STRING_TOKEN = {};
export class DerBitString {
  readonly #bytes: Uint8Array;
  readonly #unusedBits: number;
  readonly #bitLength: number;

  constructor(token: object, bytes: Uint8Array, unusedBits: number, bitLength: number) {
    if (token !== BIT_STRING_TOKEN) throw new TypeError("DerBitString values are created by decodeBitString");
    this.#bytes = Uint8Array.from(bytes);
    this.#unusedBits = unusedBits;
    this.#bitLength = bitLength;
  }

  get bytes(): Uint8Array { return this.#bytes.slice(); }
  get unusedBits(): number { return this.#unusedBits; }
  get bitLength(): number { return this.#bitLength; }
}

const OID_TOKEN = {};
export class ObjectIdentifier {
  readonly #encoded: Uint8Array;
  readonly #arcs: readonly bigint[];

  constructor(token: object, encoded: Uint8Array, arcs: readonly bigint[]) {
    if (token !== OID_TOKEN) throw new TypeError("ObjectIdentifier values are created by decodeObjectIdentifier");
    this.#encoded = Uint8Array.from(encoded);
    this.#arcs = Object.freeze([...arcs]);
  }

  get encoded(): Uint8Array { return this.#encoded.slice(); }
  get arcs(): readonly bigint[] { return this.#arcs; }
  get arcCount(): number { return this.#arcs.length; }
  equalsArcs(expected: readonly bigint[]): boolean {
    return this.#arcs.length === expected.length && this.#arcs.every((arc, index) => arc === expected[index]);
  }
}

export function decodeBoolean(element: Asn1Element): boolean {
  const state = expectUniversalPrimitive(element, 1);
  if (state.value.length !== 1) fail(Asn1ErrorKind.InvalidBooleanLength, state.valueOffset);
  if (state.value[0] === 0) return false;
  if (state.value[0] === 0xff) return true;
  fail(Asn1ErrorKind.InvalidBooleanValue, state.valueOffset);
}

export function decodeInteger(element: Asn1Element): DerInteger {
  const state = expectUniversalPrimitive(element, 2);
  const value = state.value;
  if (value.length === 0) fail(Asn1ErrorKind.EmptyInteger, state.valueOffset);
  if (value.length > 1 && ((value[0] === 0 && (value[1]! & 0x80) === 0) || (value[0] === 0xff && (value[1]! & 0x80) !== 0))) {
    fail(Asn1ErrorKind.NonMinimalInteger, state.valueOffset);
  }
  return new DerInteger(INTEGER_TOKEN, value, state.valueOffset);
}

export function decodeBitString(element: Asn1Element): DerBitString {
  const state = expectUniversalPrimitive(element, 3);
  const value = state.value;
  if (value.length === 0) fail(Asn1ErrorKind.MissingUnusedBitCount, state.valueOffset);
  const unused = value[0]!;
  if (unused > 7 || (value.length === 1 && unused !== 0)) fail(Asn1ErrorKind.InvalidUnusedBitCount, state.valueOffset);
  const bytes = value.subarray(1);
  if (unused !== 0 && (bytes[bytes.length - 1]! & ((1 << unused) - 1)) !== 0) {
    fail(Asn1ErrorKind.NonZeroBitPadding, state.valueOffset + value.length - 1);
  }
  const bitLength = bytes.length * 8 - unused;
  if (!Number.isSafeInteger(bitLength)) fail(Asn1ErrorKind.BitLengthOverflow, state.valueOffset);
  return new DerBitString(BIT_STRING_TOKEN, bytes, unused, bitLength);
}

export function decodeOctetString(element: Asn1Element): Uint8Array {
  return expectUniversalPrimitive(element, 4).value.slice();
}

export function decodeImplicitOctetString(element: Asn1Element, tagNumber: number): Uint8Array {
  return expectContextPrimitive(element, tagNumber).value.slice();
}

function decodeAscii(element: ElementState): string {
  const value = element.value;
  for (let index = 0; index < value.length; index += 1) {
    if (value[index]! > 0x7f) fail(Asn1ErrorKind.NonAsciiIa5String, element.valueOffset + index);
  }
  let output = "";
  for (let index = 0; index < value.length; index += 1) output += String.fromCharCode(value[index]!);
  return output;
}

export function decodeIA5String(element: Asn1Element): string {
  return decodeAscii(expectUniversalPrimitive(element, 22));
}

export function decodeImplicitIA5String(element: Asn1Element, tagNumber: number): string {
  return decodeAscii(expectContextPrimitive(element, tagNumber));
}

export function decodeNull(element: Asn1Element): void {
  const state = expectUniversalPrimitive(element, 5);
  if (state.value.length !== 0) fail(Asn1ErrorKind.NonEmptyNull, state.valueOffset);
}

export function decodeObjectIdentifier(element: Asn1Element, limits: Asn1Limits = defaultAsn1Limits()): ObjectIdentifier {
  return decodeOidContents(expectUniversalPrimitive(element, 6), normalizeLimits(limits));
}

export function decodeImplicitObjectIdentifier(
  element: Asn1Element,
  tagNumber: number,
  limits: Asn1Limits = defaultAsn1Limits(),
): ObjectIdentifier {
  return decodeOidContents(expectContextPrimitive(element, tagNumber), normalizeLimits(limits));
}

function decodeOidContents(element: ElementState, limits: Asn1Limits): ObjectIdentifier {
  const encoded = element.value;
  if (encoded.length === 0) fail(Asn1ErrorKind.EmptyObjectIdentifier, element.valueOffset);
  const first = parseBase128(encoded, 0, element.valueOffset);
  const arcs: bigint[] = first.value < 40n ? [0n, first.value] : first.value < 80n ? [1n, first.value - 40n] : [2n, first.value - 80n];
  if (arcs.length > limits.maxOidArcs) fail(Asn1ErrorKind.OidArcLimitExceeded, element.valueOffset);
  let offset = first.nextOffset;
  while (offset < encoded.length) {
    const start = offset;
    const parsed = parseBase128(encoded, offset, element.valueOffset);
    arcs.push(parsed.value);
    if (arcs.length > limits.maxOidArcs) fail(Asn1ErrorKind.OidArcLimitExceeded, element.valueOffset + start);
    offset = parsed.nextOffset;
  }
  return new ObjectIdentifier(OID_TOKEN, encoded, arcs);
}

function parseBase128(encoded: Uint8Array, start: number, valueOffset: number): { value: bigint; nextOffset: number } {
  if (encoded[start] === 0x80) fail(Asn1ErrorKind.NonMinimalObjectIdentifier, valueOffset + start);
  let value = 0n;
  for (let offset = start; ; offset += 1) {
    if (offset >= encoded.length) fail(Asn1ErrorKind.UnterminatedObjectIdentifier, valueOffset + offset);
    const octet = encoded[offset]!;
    const candidate = value * 128n + BigInt(octet & 0x7f);
    if (candidate > U64_MAX) fail(Asn1ErrorKind.ObjectIdentifierOverflow, valueOffset + offset);
    value = candidate;
    if ((octet & 0x80) === 0) return { value, nextOffset: offset + 1 };
  }
}

function expectUniversalPrimitive(element: Asn1Element, number: number): ElementState {
  return expectTag(element, "universal", false, number);
}

function expectContextPrimitive(element: Asn1Element, number: number): ElementState {
  return expectTag(element, "context-specific", false, number);
}

function expectTag(
  element: Asn1Element,
  tagClass: DerTag["class"],
  constructed: boolean,
  number: number,
): ElementState {
  const state = inspectElement(element);
  const tag = state.tag;
  if (tag.class !== tagClass || tag.constructed !== constructed || tag.number !== number) fail(Asn1ErrorKind.UnexpectedTag, 0);
  return state;
}
