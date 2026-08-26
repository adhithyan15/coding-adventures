/** CBR01's maximum accepted container/tag nesting. */
export const MAX_NESTING_DEPTH = 128;

/** CBR01's encoder-only cap for one complete item. */
export const MAX_ENCODED_BYTES = 1_048_576;

const U64_MAX = (1n << 64n) - 1n;

export type CborErrorId =
  | "unexpected-eof"
  | "trailing-bytes"
  | "reserved"
  | "indefinite"
  | "non-minimal-integer"
  | "invalid-utf8"
  | "non-canonical-map-order"
  | "unsupported-simple"
  | "float-not-supported"
  | "too-deep"
  | "length-too-large"
  | "duplicate-map-key"
  | "encode-too-deep"
  | "encode-too-large";

const ERROR_MESSAGES: Readonly<Record<CborErrorId, string>> = {
  "unexpected-eof": "canonical-cbor: unexpected end of input",
  "trailing-bytes": "canonical-cbor: trailing bytes after decoded item",
  reserved: "canonical-cbor: reserved additional-info value",
  indefinite: "canonical-cbor: indefinite item rejected",
  "non-minimal-integer": "canonical-cbor: argument is not in smallest form",
  "invalid-utf8": "canonical-cbor: text is not valid UTF-8",
  "non-canonical-map-order": "canonical-cbor: map key order is not canonical",
  "unsupported-simple": "canonical-cbor: unsupported simple value",
  "float-not-supported": "canonical-cbor: floats are not supported",
  "too-deep": "canonical-cbor: decoded nesting is too deep",
  "length-too-large": "canonical-cbor: declared length is too large",
  "duplicate-map-key": "canonical-cbor: duplicate canonical map key",
  "encode-too-deep": "canonical-cbor: encoded nesting is too deep",
  "encode-too-large": "canonical-cbor: encoded item is too large",
};

/** A stable, payload-blind CBR01 conformance error. */
export class CborError extends Error {
  readonly id: CborErrorId;

  constructor(id: CborErrorId) {
    super(ERROR_MESSAGES[id]);
    this.name = "CborError";
    this.id = id;
  }
}

export abstract class CborValue {
  abstract readonly kind: string;
}

export class CborUnsigned extends CborValue {
  readonly kind = "unsigned";
  readonly #value: bigint;

  constructor(value: bigint) {
    super();
    this.#value = requireU64(value, "unsigned value");
    Object.freeze(this);
  }

  get value(): bigint {
    return this.#value;
  }
}

/** Major type 1, represented by the unsigned argument in `-1 - n`. */
export class CborNegative extends CborValue {
  readonly kind = "negative";
  readonly #value: bigint;

  constructor(value: bigint) {
    super();
    this.#value = requireU64(value, "negative argument");
    Object.freeze(this);
  }

  get value(): bigint {
    return this.#value;
  }
}

export class CborByteString extends CborValue {
  readonly kind = "bytes";
  readonly #value: Uint8Array;

  constructor(value: Uint8Array) {
    super();
    if (!(value instanceof Uint8Array)) {
      throw new TypeError("canonical-cbor: byte string value must be Uint8Array");
    }
    this.#value = new Uint8Array(value);
    Object.freeze(this);
  }

  get value(): Uint8Array {
    return new Uint8Array(this.#value);
  }

  get byteLength(): number {
    return this.#value.length;
  }
}

export class CborText extends CborValue {
  readonly kind = "text";
  readonly #value: string;

  constructor(value: string) {
    super();
    if (typeof value !== "string") {
      throw new TypeError("canonical-cbor: text value must be a string");
    }
    validateScalarText(value);
    this.#value = value;
    Object.freeze(this);
  }

  get value(): string {
    return this.#value;
  }
}

export class CborArray extends CborValue {
  readonly kind = "array";
  readonly #values: CborValue[];

  constructor(values: readonly CborValue[]) {
    super();
    if (!Array.isArray(values) || values.some((value) => !(value instanceof CborValue))) {
      throw new TypeError("canonical-cbor: array items must be CborValue instances");
    }
    this.#values = [...values];
    Object.freeze(this);
  }

  get values(): CborValue[] {
    return [...this.#values];
  }

  get length(): number {
    return this.#values.length;
  }
}

export class CborMapEntry {
  readonly #key: CborValue;
  readonly #value: CborValue;

  constructor(key: CborValue, value: CborValue) {
    if (!(key instanceof CborValue) || !(value instanceof CborValue)) {
      throw new TypeError("canonical-cbor: map entries require CborValue key and value");
    }
    this.#key = key;
    this.#value = value;
    Object.freeze(this);
  }

  get key(): CborValue {
    return this.#key;
  }

  get value(): CborValue {
    return this.#value;
  }
}

export class CborMap extends CborValue {
  readonly kind = "map";
  readonly #entries: CborMapEntry[];

  constructor(entries: readonly CborMapEntry[]) {
    super();
    if (!Array.isArray(entries) || entries.some((entry) => !(entry instanceof CborMapEntry))) {
      throw new TypeError("canonical-cbor: map entries must be CborMapEntry instances");
    }
    this.#entries = [...entries];
    Object.freeze(this);
  }

  get entries(): CborMapEntry[] {
    return [...this.#entries];
  }

  get size(): number {
    return this.#entries.length;
  }
}

export class CborTag extends CborValue {
  readonly kind = "tag";
  readonly #number: bigint;
  readonly #value: CborValue;

  constructor(number: bigint, value: CborValue) {
    super();
    if (!(value instanceof CborValue)) {
      throw new TypeError("canonical-cbor: tag value must be a CborValue instance");
    }
    this.#number = requireU64(number, "tag number");
    this.#value = value;
    Object.freeze(this);
  }

  get number(): bigint {
    return this.#number;
  }

  get value(): CborValue {
    return this.#value;
  }
}

export class CborBoolean extends CborValue {
  readonly kind = "boolean";
  readonly #value: boolean;

  constructor(value: boolean) {
    super();
    if (typeof value !== "boolean") {
      throw new TypeError("canonical-cbor: boolean value must be a boolean");
    }
    this.#value = value;
    Object.freeze(this);
  }

  get value(): boolean {
    return this.#value;
  }
}

export class CborNull extends CborValue {
  readonly kind = "null";
  static readonly #instance = new CborNull();

  static get instance(): CborNull {
    return CborNull.#instance;
  }

  static {
    Object.freeze(this);
  }

  private constructor() {
    super();
    Object.freeze(this);
  }
}

/** Encode one value without exposing partial output on failure. */
export function encodeChecked(value: CborValue): Uint8Array {
  const encoder = new Encoder();
  encoder.writeValue(value, 0);
  return encoder.bytes();
}

/** Atomically append one complete encoding to an ordinary mutable byte array. */
export function encodeIntoChecked(value: CborValue, destination: number[]): void {
  if (!Array.isArray(destination)) {
    throw new TypeError("canonical-cbor: destination must be a mutable byte array");
  }
  const encoded = encodeChecked(value);
  if (destination.length > 0xffff_ffff - encoded.length) {
    throw new RangeError("destination cannot hold the complete CBOR encoding");
  }
  for (const byte of encoded) destination.push(byte);
}

/** Decode exactly one canonical item from a defensive copy of the input. */
export function decode(input: Uint8Array): CborValue {
  if (!(input instanceof Uint8Array)) {
    throw new TypeError("canonical-cbor: decode input must be Uint8Array");
  }
  const cursor = new Cursor(new Uint8Array(input));
  const value = cursor.readValue(0);
  if (cursor.remaining !== 0) {
    fail("trailing-bytes");
  }
  return value;
}

class Encoder {
  readonly #output: number[] = [];

  bytes(): Uint8Array {
    return Uint8Array.from(this.#output);
  }

  writeValue(value: CborValue, depth: number): void {
    if (depth > MAX_NESTING_DEPTH) fail("encode-too-deep");

    if (value instanceof CborUnsigned) {
      this.writeArgument(0, requireU64(value.value, "unsigned value"));
    } else if (value instanceof CborNegative) {
      this.writeArgument(1, requireU64(value.value, "negative argument"));
    } else if (value instanceof CborByteString) {
      const declaredLength = requireCollectionLength(value.byteLength, "byte string length");
      this.ensureFits(argumentSize(BigInt(declaredLength)) + declaredLength);
      const payload = value.value;
      if (!(payload instanceof Uint8Array) || payload.length !== declaredLength) {
        throw new TypeError("canonical-cbor: invalid byte string value");
      }
      this.writeArgument(2, BigInt(payload.length));
      this.writeBytes(payload);
    } else if (value instanceof CborText) {
      const text = value.value;
      if (typeof text !== "string") {
        throw new TypeError("canonical-cbor: text value must be a string");
      }
      validateScalarText(text);
      const length = utf8Length(text);
      this.ensureFits(argumentSize(BigInt(length)) + length);
      const payload = new TextEncoder().encode(text);
      this.writeArgument(3, BigInt(payload.length));
      this.writeBytes(payload);
    } else if (value instanceof CborArray) {
      const declaredLength = requireCollectionLength(value.length, "array length");
      this.ensureFits(argumentSize(BigInt(declaredLength)) + declaredLength);
      const values = value.values;
      if (!Array.isArray(values) || values.length !== declaredLength) {
        throw new TypeError("canonical-cbor: invalid array value");
      }
      this.writeArgument(4, BigInt(values.length));
      for (const item of values) this.writeValue(item, depth + 1);
    } else if (value instanceof CborMap) {
      this.writeMap(value, depth);
    } else if (value instanceof CborTag) {
      this.writeArgument(6, requireU64(value.number, "tag number"));
      this.writeValue(value.value, depth + 1);
    } else if (value instanceof CborBoolean) {
      const boolean = value.value;
      if (typeof boolean !== "boolean") {
        throw new TypeError("canonical-cbor: boolean value must be a boolean");
      }
      this.writeByte(boolean ? 0xf5 : 0xf4);
    } else if (value instanceof CborNull) {
      this.writeByte(0xf6);
    } else {
      throw new TypeError("unknown CborValue implementation");
    }
  }

  private writeMap(map: CborMap, depth: number): void {
    const declaredSize = requireCollectionLength(map.size, "map size");
    this.ensureFits(argumentSize(BigInt(declaredSize)) + declaredSize * 2);
    const sourceEntries = map.entries;
    if (!Array.isArray(sourceEntries) || sourceEntries.length !== declaredSize) {
      throw new TypeError("canonical-cbor: invalid map value");
    }
    const entries: { key: Uint8Array; value: CborValue }[] = [];
    let retainedKeyBytes = 0;

    for (const entry of sourceEntries) {
      const keyEncoder = new Encoder();
      keyEncoder.writeValue(entry.key, depth + 1);
      const key = keyEncoder.bytes();
      retainedKeyBytes += key.length;
      this.ensureFits(
        argumentSize(BigInt(sourceEntries.length)) + sourceEntries.length + retainedKeyBytes,
      );
      entries.push({ key, value: entry.value });
    }

    entries.sort((left, right) => compareLengthFirst(left.key, right.key));
    for (let index = 1; index < entries.length; index += 1) {
      if (equalBytes(entries[index - 1].key, entries[index].key)) fail("duplicate-map-key");
    }

    this.writeArgument(5, BigInt(entries.length));
    for (const entry of entries) {
      this.writeBytes(entry.key);
      this.writeValue(entry.value, depth + 1);
    }
  }

  private writeArgument(major: number, argument: bigint): void {
    const prefix = major << 5;
    if (argument <= 23n) {
      this.writeByte(prefix | Number(argument));
    } else if (argument <= 0xffn) {
      this.writeByte(prefix | 24);
      this.writeByte(Number(argument));
    } else if (argument <= 0xffffn) {
      this.writeByte(prefix | 25);
      this.writeUnsigned(argument, 2);
    } else if (argument <= 0xffff_ffffn) {
      this.writeByte(prefix | 26);
      this.writeUnsigned(argument, 4);
    } else {
      this.writeByte(prefix | 27);
      this.writeUnsigned(argument, 8);
    }
  }

  private writeUnsigned(value: bigint, width: number): void {
    for (let index = width - 1; index >= 0; index -= 1) {
      this.writeByte(Number((value >> BigInt(index * 8)) & 0xffn));
    }
  }

  private ensureFits(additionalBytes: number): void {
    if (additionalBytes > MAX_ENCODED_BYTES - this.#output.length) fail("encode-too-large");
  }

  private writeByte(value: number): void {
    if (this.#output.length >= MAX_ENCODED_BYTES) fail("encode-too-large");
    this.#output.push(value & 0xff);
  }

  private writeBytes(bytes: Uint8Array): void {
    this.ensureFits(bytes.length);
    for (const byte of bytes) this.#output.push(byte);
  }
}

type Header = { major: number; info: number; argument: bigint };

class Cursor {
  readonly #bytes: Uint8Array;
  #position = 0;

  constructor(bytes: Uint8Array) {
    this.#bytes = bytes;
  }

  get remaining(): number {
    return this.#bytes.length - this.#position;
  }

  readValue(depth: number): CborValue {
    if (depth > MAX_NESTING_DEPTH) fail("too-deep");
    const header = this.readHeader();
    if (header.major === 0) return new CborUnsigned(header.argument);
    if (header.major === 1) return new CborNegative(header.argument);
    if (header.major === 2) {
      return new CborByteString(this.readBytes(this.checkedLength(header.argument, 1)));
    }
    if (header.major === 3) return new CborText(this.readText(this.checkedLength(header.argument, 1)));
    if (header.major === 4) return this.readArray(this.checkedLength(header.argument, 1), depth);
    if (header.major === 5) return this.readMap(this.checkedLength(header.argument, 2), depth);
    if (header.major === 6) return new CborTag(header.argument, this.readValue(depth + 1));
    return this.readSimple(header.info);
  }

  private readByte(): number {
    if (this.#position >= this.#bytes.length) fail("unexpected-eof");
    return this.#bytes[this.#position++];
  }

  private readBytes(length: number): Uint8Array {
    if (length > this.remaining) fail("unexpected-eof");
    const result = this.#bytes.slice(this.#position, this.#position + length);
    this.#position += length;
    return result;
  }

  private readHeader(): Header {
    const initial = this.readByte();
    const major = initial >> 5;
    const info = initial & 0x1f;
    const enforceMinimal = major !== 7;
    let argument: bigint;
    if (info <= 23) {
      argument = BigInt(info);
    } else if (info === 24) {
      argument = BigInt(this.readByte());
      this.ensureMinimal(argument, 23n, enforceMinimal);
    } else if (info === 25) {
      argument = this.readUnsigned(2);
      this.ensureMinimal(argument, 0xffn, enforceMinimal);
    } else if (info === 26) {
      argument = this.readUnsigned(4);
      this.ensureMinimal(argument, 0xffffn, enforceMinimal);
    } else if (info === 27) {
      argument = this.readUnsigned(8);
      this.ensureMinimal(argument, 0xffff_ffffn, enforceMinimal);
    } else if (info <= 30) {
      fail("reserved");
    } else {
      fail("indefinite");
    }
    return { major, info, argument };
  }

  private readUnsigned(width: number): bigint {
    let value = 0n;
    for (let index = 0; index < width; index += 1) {
      value = (value << 8n) | BigInt(this.readByte());
    }
    return value;
  }

  private ensureMinimal(argument: bigint, previousMaximum: bigint, enabled: boolean): void {
    if (enabled && argument <= previousMaximum) fail("non-minimal-integer");
  }

  private checkedLength(declared: bigint, minimumBytesPerUnit: number): number {
    const maximum = Math.floor(this.remaining / minimumBytesPerUnit);
    if (declared > BigInt(maximum)) fail("length-too-large");
    return Number(declared);
  }

  private readText(length: number): string {
    const payload = this.readBytes(length);
    try {
      return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(payload);
    } catch (error) {
      if (error instanceof TypeError) fail("invalid-utf8");
      throw error;
    }
  }

  private readArray(count: number, depth: number): CborArray {
    const values: CborValue[] = [];
    for (let index = 0; index < count; index += 1) values.push(this.readValue(depth + 1));
    return new CborArray(values);
  }

  private readMap(count: number, depth: number): CborMap {
    const entries: CborMapEntry[] = [];
    let previousKey: Uint8Array | undefined;
    for (let index = 0; index < count; index += 1) {
      const keyStart = this.#position;
      const key = this.readValue(depth + 1);
      const encodedKey = this.#bytes.slice(keyStart, this.#position);
      if (previousKey !== undefined && compareLengthFirst(previousKey, encodedKey) >= 0) {
        fail("non-canonical-map-order");
      }
      previousKey = encodedKey;
      entries.push(new CborMapEntry(key, this.readValue(depth + 1)));
    }
    return new CborMap(entries);
  }

  private readSimple(info: number): CborValue {
    if (info === 20) return new CborBoolean(false);
    if (info === 21) return new CborBoolean(true);
    if (info === 22) return CborNull.instance;
    if (info === 25 || info === 26 || info === 27) fail("float-not-supported");
    fail("unsupported-simple");
  }
}

function compareLengthFirst(left: Uint8Array, right: Uint8Array): number {
  if (left.length !== right.length) return left.length < right.length ? -1 : 1;
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) return left[index] < right[index] ? -1 : 1;
  }
  return 0;
}

function equalBytes(left: Uint8Array, right: Uint8Array): boolean {
  return compareLengthFirst(left, right) === 0;
}

function argumentSize(argument: bigint): number {
  if (argument <= 23n) return 1;
  if (argument <= 0xffn) return 2;
  if (argument <= 0xffffn) return 3;
  if (argument <= 0xffff_ffffn) return 5;
  return 9;
}

function requireU64(value: bigint, name: string): bigint {
  if (typeof value !== "bigint" || value < 0n || value > U64_MAX) {
    throw new RangeError(`${name} must be an unsigned 64-bit bigint`);
  }
  return value;
}

function requireCollectionLength(value: number, name: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new TypeError(`canonical-cbor: ${name} must be a bounded non-negative integer`);
  }
  return value;
}

function validateScalarText(value: string): void {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw new TypeError("canonical-cbor: text is not Unicode scalar data");
      }
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) {
        throw new TypeError("canonical-cbor: text is not Unicode scalar data");
      }
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw new TypeError("canonical-cbor: text is not Unicode scalar data");
    }
  }
}

function utf8Length(value: string): number {
  let length = 0;
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit <= 0x7f) {
      length += 1;
    } else if (unit <= 0x7ff) {
      length += 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      length += 4;
      index += 1;
    } else {
      length += 3;
    }
  }
  return length;
}

function fail(id: CborErrorId): never {
  throw new CborError(id);
}
