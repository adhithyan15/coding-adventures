import { describe, expect, it } from "vitest";
import {
  array,
  bulkString,
  decode,
  decodeAll,
  encode,
  errorValue,
  integer,
  RespDecoder,
  simpleString,
} from "../src/index.js";

describe("resp-protocol", () => {
  it("encodes and decodes RESP2 values", () => {
    expect(decode(encode(simpleString("OK")))).toEqual({
      value: simpleString("OK"),
      consumed: 5,
    });
    expect(decode(encode(errorValue("ERR boom")))).toEqual({
      value: errorValue("ERR boom"),
      consumed: 11,
    });
    expect(decode(encode(integer(42)))).toEqual({
      value: integer(42),
      consumed: 5,
    });
    expect(decode(encode(bulkString("foo")))).toEqual({
      value: bulkString("foo"),
      consumed: 9,
    });
  });

  it("round-trips command arrays", () => {
    const frame = array([
      bulkString("SET"),
      bulkString("counter"),
      bulkString("1"),
    ]);

    expect(decode(encode(frame))).toEqual({
      value: frame,
      consumed: encode(frame).length,
    });
  });

  it("decodes inline commands and blank lines", () => {
    expect(decode("PING\r\n")).toEqual({
      value: array([bulkString("PING")]),
      consumed: 6,
    });
    expect(decode("   \r\n")).toEqual({
      value: array([]),
      consumed: 5,
    });
  });

  it("supports incremental buffering", () => {
    const decoder = new RespDecoder();
    decoder.feed("*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n");
    expect(decoder.hasMessage()).toBe(true);
    expect(decoder.getMessage()).toEqual(
      array([bulkString("PING"), bulkString("hello")]),
    );
  });

  it("decodes multiple concatenated messages", () => {
    const bytes = encode(simpleString("OK"));
    const combined = new Uint8Array(bytes.length * 2);
    combined.set(bytes, 0);
    combined.set(bytes, bytes.length);

    expect(decodeAll(combined)).toEqual({
      values: [simpleString("OK"), simpleString("OK")],
      consumed: combined.length,
    });
  });
});
