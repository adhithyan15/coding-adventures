/** Bounded generic RFC 5280 Extension decoding. */

import {
  Asn1Cursor,
  Asn1Decoder,
  Asn1Element,
  Asn1Error,
  Asn1ErrorKind,
  type ObjectIdentifier,
  decodeBoolean,
  decodeObjectIdentifier,
  decodeOctetString,
  trustedCursorRead,
  trustedCursorRemainingLength,
  trustedDecoderLimits,
  trustedElementShape,
  trustedSequence,
} from "@coding-adventures/der-asn1";

export const VERSION = "0.1.0";
const BOOLEAN_TAG = 1;

export const X509ExtensionErrorKind = Object.freeze({
  Structure: "structure",
  MissingExtensionId: "missing-extension-id",
  InvalidExtensionId: "invalid-extension-id",
  InvalidCritical: "invalid-critical",
  EncodedDefaultCritical: "encoded-default-critical",
  MissingExtensionValue: "missing-extension-value",
  InvalidExtensionValue: "invalid-extension-value",
  TrailingElement: "trailing-element",
} as const);

export type X509ExtensionErrorKind =
  (typeof X509ExtensionErrorKind)[keyof typeof X509ExtensionErrorKind];

export class X509ExtensionError extends Error {
  readonly kind: X509ExtensionErrorKind;
  readonly offset: number;
  readonly asn1Kind?: Asn1Error["kind"];
  readonly framingKind?: Asn1Error["framingKind"];

  constructor(kind: X509ExtensionErrorKind, offset: number, nested?: Asn1Error) {
    super(`X.509 extension error ${kind} at byte ${offset}`);
    this.name = "X509ExtensionError";
    this.kind = kind;
    this.offset = offset;
    this.asn1Kind = nested?.kind;
    this.framingKind = nested?.framingKind;
  }
}

const VALUE_TOKEN = {};

export class X509Extension {
  readonly #extensionId: ObjectIdentifier;
  readonly #critical: boolean;
  readonly #extensionValue: Uint8Array;

  constructor(token: object, extensionId: ObjectIdentifier, critical: boolean, value: Uint8Array) {
    if (token !== VALUE_TOKEN) throw new TypeError("X509Extension values are created by decodeX509Extension");
    this.#extensionId = extensionId;
    this.#critical = critical;
    this.#extensionValue = Uint8Array.from(value);
  }

  get extensionId(): ObjectIdentifier { return this.#extensionId; }
  get critical(): boolean { return this.#critical; }
  get extensionValue(): Uint8Array { return this.#extensionValue.slice(); }
}

function structure(error: Asn1Error, valueOffset?: number, childOffset?: number): X509ExtensionError {
  let offset = error.offset;
  if (valueOffset !== undefined && childOffset !== undefined) {
    offset += error.kind === Asn1ErrorKind.Framing ? valueOffset : childOffset;
  }
  return new X509ExtensionError(X509ExtensionErrorKind.Structure, offset, error);
}

function semantic(kind: X509ExtensionErrorKind, error: Asn1Error, childOffset: number): X509ExtensionError {
  return new X509ExtensionError(kind, childOffset + error.offset, error);
}

function childOffset(valueOffset: number, valueLength: number, remainingLength: number): number {
  return valueOffset + valueLength - remainingLength;
}

export function decodeX509Extension(decoder: Asn1Decoder, element: Asn1Element): X509Extension {
  const rootShape = trustedElementShape(element);
  const valueOffset = rootShape.headerLength;
  const valueLength = rootShape.valueLength;
  let fields: Asn1Cursor;
  try { fields = trustedSequence(decoder, element); }
  catch (error: unknown) {
    /* v8 ignore else -- dependency documents Asn1Error as its only failure */
    if (error instanceof Asn1Error) throw structure(error);
    /* v8 ignore next -- preserve unexpected dependency failures */
    throw error;
  }

  const read = (offset: number): Asn1Element | undefined => {
    try { return trustedCursorRead(fields, decoder); }
    catch (error: unknown) {
      /* v8 ignore else -- dependency documents Asn1Error as its only failure */
      if (error instanceof Asn1Error) throw structure(error, valueOffset, offset);
      /* v8 ignore next -- preserve unexpected dependency failures */
      throw error;
    }
  };

  const idOffset = childOffset(valueOffset, valueLength, trustedCursorRemainingLength(fields));
  const idElement = read(idOffset);
  if (idElement === undefined) throw new X509ExtensionError(X509ExtensionErrorKind.MissingExtensionId, idOffset);
  let extensionId;
  try { extensionId = decodeObjectIdentifier(idElement, trustedDecoderLimits(decoder)); }
  catch (error: unknown) {
    /* v8 ignore else -- dependency documents Asn1Error as its only failure */
    if (error instanceof Asn1Error) throw semantic(X509ExtensionErrorKind.InvalidExtensionId, error, idOffset);
    /* v8 ignore next -- preserve unexpected dependency failures */
    throw error;
  }

  const secondOffset = childOffset(valueOffset, valueLength, trustedCursorRemainingLength(fields));
  const second = read(secondOffset);
  if (second === undefined) throw new X509ExtensionError(X509ExtensionErrorKind.MissingExtensionValue, secondOffset);

  let critical = false;
  let valueElement = second;
  let valueElementOffset = secondOffset;
  if (trustedElementShape(second).tag.number === BOOLEAN_TAG) {
    try { critical = decodeBoolean(second); }
    catch (error: unknown) {
      /* v8 ignore else -- dependency documents Asn1Error as its only failure */
      if (error instanceof Asn1Error) throw semantic(X509ExtensionErrorKind.InvalidCritical, error, secondOffset);
      /* v8 ignore next -- preserve unexpected dependency failures */
      throw error;
    }
    if (!critical) throw new X509ExtensionError(X509ExtensionErrorKind.EncodedDefaultCritical, secondOffset);
    valueElementOffset = childOffset(valueOffset, valueLength, trustedCursorRemainingLength(fields));
    const third = read(valueElementOffset);
    if (third === undefined) throw new X509ExtensionError(X509ExtensionErrorKind.MissingExtensionValue, valueElementOffset);
    valueElement = third;
  }

  let extensionValue;
  try { extensionValue = decodeOctetString(valueElement); }
  catch (error: unknown) {
    /* v8 ignore else -- dependency documents Asn1Error as its only failure */
    if (error instanceof Asn1Error) throw semantic(X509ExtensionErrorKind.InvalidExtensionValue, error, valueElementOffset);
    /* v8 ignore next -- preserve unexpected dependency failures */
    throw error;
  }

  const trailingOffset = childOffset(valueOffset, valueLength, trustedCursorRemainingLength(fields));
  if (read(trailingOffset) !== undefined) {
    throw new X509ExtensionError(X509ExtensionErrorKind.TrailingElement, trailingOffset);
  }
  return new X509Extension(VALUE_TOKEN, extensionId, critical, extensionValue);
}
