const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

export interface RespSimpleString {
  readonly kind: "simple-string";
  readonly value: string;
}

export interface RespErrorValue {
  readonly kind: "error";
  readonly value: string;
}

export interface RespInteger {
  readonly kind: "integer";
  readonly value: number;
}

export interface RespBulkString {
  readonly kind: "bulk-string";
  readonly value: Uint8Array | null;
}

export interface RespArray {
  readonly kind: "array";
  readonly value: RespValue[] | null;
}

export type RespValue =
  | RespSimpleString
  | RespErrorValue
  | RespInteger
  | RespBulkString
  | RespArray;

export interface RespDecodeResult {
  readonly value: RespValue;
  readonly consumed: number;
}

export interface RespDecodeAllResult {
  readonly values: RespValue[];
  readonly consumed: number;
}

export class RespDecodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "RespDecodeError";
  }
}

export class RespEncodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "RespEncodeError";
  }
}

export function simpleString(value: string): RespSimpleString {
  return { kind: "simple-string", value };
}

export function errorValue(value: string): RespErrorValue {
  return { kind: "error", value };
}

export function integer(value: number): RespInteger {
  return { kind: "integer", value };
}

export function bulkString(value: string | Uint8Array | null): RespBulkString {
  if (value === null) {
    return { kind: "bulk-string", value: null };
  }
  return {
    kind: "bulk-string",
    value: typeof value === "string" ? textEncoder.encode(value) : new Uint8Array(value),
  };
}

export function array(value: RespValue[] | null): RespArray {
  return { kind: "array", value };
}

export function encode(value: RespValue): Uint8Array {
  switch (value.kind) {
    case "simple-string":
      return encodeSimpleString(value.value);
    case "error":
      return encodeError(value.value);
    case "integer":
      return encodeInteger(value.value);
    case "bulk-string":
      return encodeBulkString(value.value);
    case "array":
      return encodeArray(value.value);
  }
}

export function encodeSimpleString(value: string): Uint8Array {
  return textEncoder.encode(`+${value}\r\n`);
}

export function encodeError(value: string): Uint8Array {
  return textEncoder.encode(`-${value}\r\n`);
}

export function encodeInteger(value: number): Uint8Array {
  if (!Number.isInteger(value)) {
    throw new RespEncodeError("RESP integer values must be integers");
  }
  return textEncoder.encode(`:${value}\r\n`);
}

export function encodeBulkString(value: Uint8Array | string | null): Uint8Array {
  if (value === null) {
    return textEncoder.encode("$-1\r\n");
  }
  const bytes = typeof value === "string" ? textEncoder.encode(value) : new Uint8Array(value);
  return concatBytes([
    textEncoder.encode(`$${bytes.length}\r\n`),
    bytes,
    textEncoder.encode("\r\n"),
  ]);
}

export function encodeArray(values: RespValue[] | null): Uint8Array {
  if (values === null) {
    return textEncoder.encode("*-1\r\n");
  }
  const parts: Uint8Array[] = [textEncoder.encode(`*${values.length}\r\n`)];
  for (const value of values) {
    parts.push(encode(value));
  }
  return concatBytes(parts);
}

export function decode(input: Uint8Array | string): RespDecodeResult | null {
  const buffer = asBytes(input);
  if (buffer.length === 0) {
    return null;
  }

  switch (buffer[0]) {
    case 43:
      return decodeSimpleString(buffer);
    case 45:
      return decodeError(buffer);
    case 58:
      return decodeInteger(buffer);
    case 36:
      return decodeBulkString(buffer);
    case 42:
      return decodeArray(buffer);
    default:
      return decodeInlineCommand(buffer);
  }
}

export function decodeAll(input: Uint8Array | string): RespDecodeAllResult {
  const buffer = asBytes(input);
  const values: RespValue[] = [];
  let offset = 0;
  while (offset < buffer.length) {
    const result = decode(buffer.slice(offset));
    if (result === null) {
      break;
    }
    values.push(result.value);
    offset += result.consumed;
  }
  return { values, consumed: offset };
}

export class RespDecoder {
  private buffer = new Uint8Array(0);
  private queue: RespValue[] = [];
  private error: RespDecodeError | null = null;

  feed(data: Uint8Array | string): void {
    this.buffer = concatBytes([this.buffer, asBytes(data)]);
    this.drain();
  }

  hasMessage(): boolean {
    return this.queue.length > 0;
  }

  getMessage(): RespValue {
    if (this.error) {
      throw this.error;
    }
    const value = this.queue.shift();
    if (value === undefined) {
      throw new RespDecodeError("decoder buffer is empty");
    }
    return value;
  }

  decodeAll(data: Uint8Array | string): RespValue[] {
    this.feed(data);
    if (this.error) {
      throw this.error;
    }
    const messages = this.queue.slice();
    this.queue = [];
    return messages;
  }

  private drain(): void {
    if (this.error) {
      return;
    }
    while (this.buffer.length > 0) {
      const result = decode(this.buffer);
      if (result === null) {
        return;
      }
      this.queue.push(result.value);
      this.buffer = this.buffer.slice(result.consumed);
    }
  }
}

function decodeSimpleString(buffer: Uint8Array): RespDecodeResult | null {
  const line = readLine(buffer, 1);
  if (line === null) {
    return null;
  }
  return {
    value: simpleString(textDecoder.decode(line.line)),
    consumed: line.consumed,
  };
}

function decodeError(buffer: Uint8Array): RespDecodeResult | null {
  const line = readLine(buffer, 1);
  if (line === null) {
    return null;
  }
  return {
    value: errorValue(textDecoder.decode(line.line)),
    consumed: line.consumed,
  };
}

function decodeInteger(buffer: Uint8Array): RespDecodeResult | null {
  const line = readLine(buffer, 1);
  if (line === null) {
    return null;
  }
  const value = Number.parseInt(textDecoder.decode(line.line), 10);
  if (!Number.isFinite(value)) {
    throw new RespDecodeError("invalid RESP integer");
  }
  return { value: integer(value), consumed: line.consumed };
}

function decodeBulkString(buffer: Uint8Array): RespDecodeResult | null {
  const line = readLine(buffer, 1);
  if (line === null) {
    return null;
  }
  const length = Number.parseInt(textDecoder.decode(line.line), 10);
  if (Number.isNaN(length)) {
    throw new RespDecodeError("invalid RESP bulk string length");
  }
  if (length === -1) {
    return { value: bulkString(null), consumed: line.consumed };
  }
  if (length < -1) {
    throw new RespDecodeError("bulk string length cannot be negative");
  }

  const bodyStart = line.consumed;
  const bodyEnd = bodyStart + length;
  const tailEnd = bodyEnd + 2;
  if (buffer.length < tailEnd) {
    return null;
  }
  if (buffer[bodyEnd] !== 13 || buffer[bodyEnd + 1] !== 10) {
    throw new RespDecodeError("missing trailing CRLF after bulk string body");
  }
  return {
    value: { kind: "bulk-string", value: buffer.slice(bodyStart, bodyEnd) },
    consumed: tailEnd,
  };
}

function decodeArray(buffer: Uint8Array): RespDecodeResult | null {
  const line = readLine(buffer, 1);
  if (line === null) {
    return null;
  }
  const count = Number.parseInt(textDecoder.decode(line.line), 10);
  if (Number.isNaN(count)) {
    throw new RespDecodeError("invalid RESP array length");
  }
  if (count === -1) {
    return { value: array(null), consumed: line.consumed };
  }
  if (count < -1) {
    throw new RespDecodeError("array length cannot be negative");
  }

  const values: RespValue[] = [];
  let offset = line.consumed;
  for (let i = 0; i < count; i += 1) {
    const result = decode(buffer.slice(offset));
    if (result === null) {
      return null;
    }
    values.push(result.value);
    offset += result.consumed;
  }
  return { value: array(values), consumed: offset };
}

function decodeInlineCommand(buffer: Uint8Array): RespDecodeResult | null {
  const line = readLine(buffer, 0);
  if (line === null) {
    return null;
  }
  const text = textDecoder.decode(line.line).trim();
  const tokens = text === ""
    ? []
    : text
        .split(/\s+/)
        .filter((token) => token.length > 0)
        .map((token) => bulkString(token));
  return {
    value: array(tokens),
    consumed: line.consumed,
  };
}

function readLine(buffer: Uint8Array, start: number): { line: Uint8Array; consumed: number } | null {
  for (let i = start; i < buffer.length - 1; i += 1) {
    if (buffer[i] === 13 && buffer[i + 1] === 10) {
      return {
        line: buffer.slice(start, i),
        consumed: i + 2,
      };
    }
  }
  return null;
}

function concatBytes(chunks: Uint8Array[]): Uint8Array {
  const length = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const result = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.length;
  }
  return result;
}

function asBytes(input: Uint8Array | string): Uint8Array {
  if (typeof input === "string") {
    return textEncoder.encode(input);
  }
  return new Uint8Array(input);
}
