/**
 * HTTP/1 request and response head parsing.
 *
 * The parser stops at the semantic boundary the rest of the stack cares about:
 * start line, ordered headers, the offset where the body begins, and the rule
 * for consuming that body.
 */

import {
  BodyKind,
  type Header,
  HttpVersion,
  RequestHead,
  ResponseHead,
} from "@coding-adventures/http-core";

export const VERSION = "0.1.0";

const DECODER = new TextDecoder("latin1");
const ENCODER = new TextEncoder();

export class Http1ParseError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "Http1ParseError";
  }
}

export class ParsedRequestHead {
  public readonly head: RequestHead;
  public readonly bodyOffset: number;
  public readonly bodyKind: BodyKind;

  public constructor(head: RequestHead, bodyOffset: number, bodyKind: BodyKind) {
    this.head = head;
    this.bodyOffset = bodyOffset;
    this.bodyKind = bodyKind;
  }
}

export class ParsedResponseHead {
  public readonly head: ResponseHead;
  public readonly bodyOffset: number;
  public readonly bodyKind: BodyKind;

  public constructor(head: ResponseHead, bodyOffset: number, bodyKind: BodyKind) {
    this.head = head;
    this.bodyOffset = bodyOffset;
    this.bodyKind = bodyKind;
  }
}

export function parseRequestHead(input: Uint8Array | string): ParsedRequestHead {
  const { lines, bodyOffset } = splitHeadLines(input);
  if (lines.length === 0) {
    throw new Http1ParseError("invalid HTTP/1 start line");
  }

  const parts = lines[0].trim().split(/\s+/u);
  if (parts.length !== 3) {
    throw new Http1ParseError(`invalid HTTP/1 start line: ${lines[0]}`);
  }

  let version: HttpVersion;
  try {
    version = HttpVersion.parse(parts[2]);
  } catch (error) {
    throw new Http1ParseError((error as Error).message);
  }

  const headers = parseHeaders(lines.slice(1));
  return new ParsedRequestHead(
    new RequestHead(parts[0], parts[1], version, headers),
    bodyOffset,
    requestBodyKind(headers),
  );
}

export function parseResponseHead(input: Uint8Array | string): ParsedResponseHead {
  const { lines, bodyOffset } = splitHeadLines(input);
  if (lines.length === 0) {
    throw new Http1ParseError("invalid HTTP/1 status line");
  }

  const parts = lines[0].trim().split(/\s+/u);
  if (parts.length < 2) {
    throw new Http1ParseError(`invalid HTTP/1 status line: ${lines[0]}`);
  }

  let version: HttpVersion;
  try {
    version = HttpVersion.parse(parts[0]);
  } catch (error) {
    throw new Http1ParseError((error as Error).message);
  }

  const status = Number.parseInt(parts[1], 10);
  if (!Number.isInteger(status)) {
    throw new Http1ParseError(`invalid HTTP status: ${parts[1]}`);
  }

  const headers = parseHeaders(lines.slice(1));
  return new ParsedResponseHead(
    new ResponseHead(version, status, parts.slice(2).join(" "), headers),
    bodyOffset,
    responseBodyKind(status, headers),
  );
}

function splitHeadLines(input: Uint8Array | string): { lines: string[]; bodyOffset: number } {
  const bytes = typeof input === "string" ? ENCODER.encode(input) : input;
  let index = 0;

  while (index < bytes.length) {
    if (bytes[index] === 13 && bytes[index + 1] === 10) {
      index += 2;
      continue;
    }
    if (bytes[index] === 10) {
      index += 1;
      continue;
    }
    break;
  }

  const lines: string[] = [];
  while (true) {
    if (index >= bytes.length) {
      throw new Http1ParseError("incomplete HTTP/1 head");
    }

    const lineStart = index;
    while (index < bytes.length && bytes[index] !== 10) {
      index += 1;
    }
    if (index >= bytes.length) {
      throw new Http1ParseError("incomplete HTTP/1 head");
    }

    const lineEnd = index > lineStart && bytes[index - 1] === 13 ? index - 1 : index;
    const line = DECODER.decode(bytes.slice(lineStart, lineEnd));
    index += 1;

    if (line.length === 0) {
      return { lines, bodyOffset: index };
    }
    lines.push(line);
  }
}

function parseHeaders(lines: string[]): Header[] {
  return lines.map((line) => {
    const separator = line.indexOf(":");
    if (separator <= 0) {
      throw new Http1ParseError(`invalid HTTP/1 header: ${line}`);
    }
    return {
      name: line.slice(0, separator).trim(),
      value: line.slice(separator + 1).trim(),
    };
  });
}

function requestBodyKind(headers: Header[]): BodyKind {
  if (hasChunkedTransferEncoding(headers)) {
    return BodyKind.chunked();
  }

  const length = declaredContentLength(headers);
  if (length === undefined || length === 0) {
    return BodyKind.none();
  }
  return BodyKind.contentLength(length);
}

function responseBodyKind(status: number, headers: Header[]): BodyKind {
  if ((status >= 100 && status < 200) || status === 204 || status === 304) {
    return BodyKind.none();
  }
  if (hasChunkedTransferEncoding(headers)) {
    return BodyKind.chunked();
  }

  const length = declaredContentLength(headers);
  if (length === undefined) {
    return BodyKind.untilEof();
  }
  if (length === 0) {
    return BodyKind.none();
  }
  return BodyKind.contentLength(length);
}

function declaredContentLength(headers: Header[]): number | undefined {
  const value = headers.find((header) => header.name.toLowerCase() === "content-length")?.value;
  if (value === undefined) {
    return undefined;
  }
  if (!/^\d+$/u.test(value)) {
    throw new Http1ParseError(`invalid Content-Length: ${value}`);
  }
  return Number.parseInt(value, 10);
}

function hasChunkedTransferEncoding(headers: Header[]): boolean {
  return headers
    .filter((header) => header.name.toLowerCase() === "transfer-encoding")
    .some((header) => header.value.split(",").some((piece) => piece.trim().toLowerCase() === "chunked"));
}
