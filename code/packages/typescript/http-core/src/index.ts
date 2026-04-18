/**
 * Shared HTTP message types and helpers.
 *
 * Version-specific parsers can disagree wildly about wire syntax, but callers
 * still need the same semantic objects: headers, request heads, response heads,
 * versions, and body framing instructions.
 */

export const VERSION = "0.1.0";

export interface Header {
  name: string;
  value: string;
}

export class HttpVersion {
  public readonly major: number;
  public readonly minor: number;

  public constructor(major: number, minor: number) {
    this.major = major;
    this.minor = minor;
  }

  public static parse(text: string): HttpVersion {
    if (!text.startsWith("HTTP/")) {
      throw new Error(`invalid HTTP version: ${text}`);
    }
    const [majorText, minorText] = text.slice(5).split(".", 2);
    if (majorText === undefined || minorText === undefined || !/^\d+$/.test(majorText) || !/^\d+$/.test(minorText)) {
      throw new Error(`invalid HTTP version: ${text}`);
    }
    return new HttpVersion(Number.parseInt(majorText, 10), Number.parseInt(minorText, 10));
  }

  public toString(): string {
    return `HTTP/${this.major}.${this.minor}`;
  }
}

export type BodyMode = "none" | "content-length" | "until-eof" | "chunked";

export class BodyKind {
  public readonly mode: BodyMode;
  public readonly length: number | null;

  public constructor(mode: BodyMode, length: number | null = null) {
    this.mode = mode;
    this.length = length;
  }

  public static none(): BodyKind {
    return new BodyKind("none");
  }

  public static contentLength(length: number): BodyKind {
    return new BodyKind("content-length", length);
  }

  public static untilEof(): BodyKind {
    return new BodyKind("until-eof");
  }

  public static chunked(): BodyKind {
    return new BodyKind("chunked");
  }
}

export class RequestHead {
  public readonly method: string;
  public readonly target: string;
  public readonly version: HttpVersion;
  public readonly headers: Header[];

  public constructor(method: string, target: string, version: HttpVersion, headers: Header[]) {
    this.method = method;
    this.target = target;
    this.version = version;
    this.headers = headers;
  }

  public header(name: string): string | undefined {
    return findHeader(this.headers, name);
  }

  public contentLength(): number | undefined {
    return parseContentLength(this.headers);
  }

  public contentType(): [string, string | undefined] | undefined {
    return parseContentType(this.headers);
  }
}

export class ResponseHead {
  public readonly version: HttpVersion;
  public readonly status: number;
  public readonly reason: string;
  public readonly headers: Header[];

  public constructor(version: HttpVersion, status: number, reason: string, headers: Header[]) {
    this.version = version;
    this.status = status;
    this.reason = reason;
    this.headers = headers;
  }

  public header(name: string): string | undefined {
    return findHeader(this.headers, name);
  }

  public contentLength(): number | undefined {
    return parseContentLength(this.headers);
  }

  public contentType(): [string, string | undefined] | undefined {
    return parseContentType(this.headers);
  }
}

export function findHeader(headers: Header[], name: string): string | undefined {
  const lowered = name.toLowerCase();
  return headers.find((header) => header.name.toLowerCase() === lowered)?.value;
}

export function parseContentLength(headers: Header[]): number | undefined {
  const value = findHeader(headers, "Content-Length");
  if (value === undefined || !/^\d+$/.test(value)) {
    return undefined;
  }
  return Number.parseInt(value, 10);
}

export function parseContentType(headers: Header[]): [string, string | undefined] | undefined {
  const value = findHeader(headers, "Content-Type");
  if (value === undefined) {
    return undefined;
  }

  const pieces = value.split(";").map((piece) => piece.trim());
  const mediaType = pieces[0];
  if (!mediaType) {
    return undefined;
  }

  let charset: string | undefined;
  for (const piece of pieces.slice(1)) {
    const [key, rawValue] = piece.split("=", 2);
    if (rawValue !== undefined && key.trim().toLowerCase() === "charset") {
      charset = rawValue.trim().replace(/^"|"$/g, "");
      break;
    }
  }

  return [mediaType, charset];
}
