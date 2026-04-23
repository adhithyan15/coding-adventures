import { describe, expect, it } from "vitest";

import {
  BodyKind,
  HttpVersion,
  RequestHead,
  ResponseHead,
  findHeader,
  parseContentLength,
  parseContentType,
} from "../src/index.js";

describe("http-core", () => {
  it("parses and renders versions", () => {
    const version = HttpVersion.parse("HTTP/1.1");
    expect(version.major).toBe(1);
    expect(version.minor).toBe(1);
    expect(version.toString()).toBe("HTTP/1.1");
  });

  it("looks up headers case-insensitively", () => {
    expect(findHeader([{ name: "Content-Type", value: "text/plain" }], "content-type")).toBe("text/plain");
  });

  it("parses content helpers", () => {
    const headers = [
      { name: "Content-Length", value: "42" },
      { name: "Content-Type", value: "text/html; charset=utf-8" },
    ];
    expect(parseContentLength(headers)).toBe(42);
    expect(parseContentType(headers)).toEqual(["text/html", "utf-8"]);
  });

  it("delegates from request and response heads", () => {
    const request = new RequestHead("POST", "/submit", new HttpVersion(1, 1), [
      { name: "Content-Length", value: "5" },
    ]);
    const response = new ResponseHead(new HttpVersion(1, 0), 200, "OK", [
      { name: "Content-Type", value: "application/json" },
    ]);

    expect(request.contentLength()).toBe(5);
    expect(response.contentType()).toEqual(["application/json", undefined]);
  });

  it("constructs body kinds", () => {
    expect(BodyKind.none()).toEqual(new BodyKind("none"));
    expect(BodyKind.contentLength(7)).toEqual(new BodyKind("content-length", 7));
    expect(BodyKind.untilEof()).toEqual(new BodyKind("until-eof"));
    expect(BodyKind.chunked()).toEqual(new BodyKind("chunked"));
  });
});
