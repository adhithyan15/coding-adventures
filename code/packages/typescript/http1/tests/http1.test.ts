import { describe, it, expect } from "vitest";
import { BodyKind, HttpVersion } from "@coding-adventures/http-core";
import {
  Http1ParseError,
  VERSION,
  parseRequestHead,
  parseResponseHead,
} from "../src/index.js";

describe("http1", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("parses a simple request head", () => {
    const parsed = parseRequestHead("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");
    expect(parsed.head.method).toBe("GET");
    expect(parsed.head.target).toBe("/");
    expect(parsed.head.version).toEqual(new HttpVersion(1, 0));
    expect(parsed.bodyKind).toEqual(BodyKind.none());
  });

  it("parses a request body framing rule", () => {
    const parsed = parseRequestHead("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello");
    expect(parsed.bodyKind).toEqual(BodyKind.contentLength(5));
  });

  it("parses a response head", () => {
    const parsed = parseResponseHead("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody");
    expect(parsed.head.status).toBe(200);
    expect(parsed.head.reason).toBe("OK");
    expect(parsed.bodyKind).toEqual(BodyKind.contentLength(4));
  });

  it("uses until-eof when no length is present", () => {
    const parsed = parseResponseHead("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n");
    expect(parsed.bodyKind).toEqual(BodyKind.untilEof());
  });

  it("treats bodyless status codes as bodyless", () => {
    const parsed = parseResponseHead("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n");
    expect(parsed.bodyKind).toEqual(BodyKind.none());
  });

  it("accepts LF-only input and preserves duplicate headers", () => {
    const parsed = parseResponseHead("\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload");
    expect(parsed.head.headers.map((header) => header.value)).toEqual(["a=1", "b=2"]);
  });

  it("rejects malformed headers", () => {
    expect(() => parseRequestHead("GET / HTTP/1.1\r\nHost example.com\r\n\r\n")).toThrow(Http1ParseError);
  });

  it("rejects malformed content length values", () => {
    expect(() => parseResponseHead("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n")).toThrow(Http1ParseError);
  });
});
