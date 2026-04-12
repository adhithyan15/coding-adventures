/**
 * Tests for irc-framing — Framer class.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { Framer } from "../src/index.js";

describe("Framer", () => {
  let framer: Framer;

  beforeEach(() => {
    framer = new Framer();
  });

  it("starts with an empty buffer", () => {
    expect(framer.bufferSize).toBe(0);
    expect(framer.frames()).toEqual([]);
  });

  it("returns no frames when no newline yet", () => {
    framer.feed(Buffer.from("NICK ali"));
    expect(framer.frames()).toEqual([]);
    expect(framer.bufferSize).toBe(8);
  });

  it("returns one frame on CRLF-terminated message", () => {
    framer.feed(Buffer.from("NICK alice\r\n"));
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    expect(frames[0].toString("utf-8")).toBe("NICK alice");
  });

  it("returns one frame on LF-only-terminated message", () => {
    framer.feed(Buffer.from("NICK alice\n"));
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    expect(frames[0].toString("utf-8")).toBe("NICK alice");
  });

  it("handles partial message followed by completion", () => {
    framer.feed(Buffer.from("NICK ali"));
    expect(framer.frames()).toEqual([]);
    framer.feed(Buffer.from("ce\r\n"));
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    expect(frames[0].toString("utf-8")).toBe("NICK alice");
  });

  it("handles multiple messages in one feed", () => {
    framer.feed(Buffer.from("NICK alice\r\nJOIN #general\r\n"));
    const frames = framer.frames();
    expect(frames).toHaveLength(2);
    expect(frames[0].toString("utf-8")).toBe("NICK alice");
    expect(frames[1].toString("utf-8")).toBe("JOIN #general");
  });

  it("handles three messages in one feed", () => {
    framer.feed(Buffer.from("A\r\nB\r\nC\r\n"));
    const frames = framer.frames();
    expect(frames).toHaveLength(3);
    expect(frames[0].toString("utf-8")).toBe("A");
    expect(frames[1].toString("utf-8")).toBe("B");
    expect(frames[2].toString("utf-8")).toBe("C");
  });

  it("accumulates partial messages across multiple feeds", () => {
    framer.feed(Buffer.from("PRIV"));
    framer.feed(Buffer.from("MSG #c"));
    framer.feed(Buffer.from("han :hello\r\n"));
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    expect(frames[0].toString("utf-8")).toBe("PRIVMSG #chan :hello");
  });

  it("leaves partial data in buffer after extracting complete frames", () => {
    framer.feed(Buffer.from("NICK alice\r\nJOIN #g"));
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    expect(framer.bufferSize).toBe("JOIN #g".length);
  });

  it("discards overlong lines (>510 bytes)", () => {
    // Create a line of 511 bytes followed by CRLF — should be discarded.
    const longLine = Buffer.alloc(511, 0x41); // 511 'A' bytes
    const msg = Buffer.concat([longLine, Buffer.from("\r\n")]);
    framer.feed(msg);
    const frames = framer.frames();
    expect(frames).toHaveLength(0);
  });

  it("accepts exactly 510-byte line (boundary)", () => {
    const line510 = Buffer.alloc(510, 0x41); // exactly 510 'A' bytes
    const msg = Buffer.concat([line510, Buffer.from("\r\n")]);
    framer.feed(msg);
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    expect(frames[0].length).toBe(510);
  });

  it("discards overlong line but continues processing subsequent lines", () => {
    const longLine = Buffer.alloc(511, 0x42); // 511 'B' bytes
    const msg = Buffer.concat([
      longLine,
      Buffer.from("\r\n"),
      Buffer.from("NICK alice\r\n"),
    ]);
    framer.feed(msg);
    const frames = framer.frames();
    // The overlong line is discarded; the short line is yielded.
    expect(frames).toHaveLength(1);
    expect(frames[0].toString("utf-8")).toBe("NICK alice");
  });

  it("reset() clears all buffered data", () => {
    framer.feed(Buffer.from("partial data without newline"));
    expect(framer.bufferSize).toBeGreaterThan(0);
    framer.reset();
    expect(framer.bufferSize).toBe(0);
    expect(framer.frames()).toEqual([]);
  });

  it("reset() prevents stale data from bleeding into new session", () => {
    framer.feed(Buffer.from("NICK alice\r\nJOIN #c"));
    framer.frames(); // drain the complete frame
    framer.reset();  // simulate connection close
    framer.feed(Buffer.from("USER bob 0 * :Bob\r\n"));
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    expect(frames[0].toString("utf-8")).toBe("USER bob 0 * :Bob");
  });

  it("handles empty feed call gracefully", () => {
    framer.feed(Buffer.alloc(0));
    expect(framer.bufferSize).toBe(0);
    expect(framer.frames()).toEqual([]);
  });

  it("handles message split exactly on CR/LF boundary", () => {
    framer.feed(Buffer.from("NICK alice\r"));
    expect(framer.frames()).toHaveLength(0);
    framer.feed(Buffer.from("\n"));
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    expect(frames[0].toString("utf-8")).toBe("NICK alice");
  });

  it("handles LF as first byte (empty line)", () => {
    framer.feed(Buffer.from("\n"));
    const frames = framer.frames();
    // An empty line (zero content bytes) is under the 510-byte limit, so
    // it is yielded as an empty buffer.
    expect(frames).toHaveLength(1);
    expect(frames[0].length).toBe(0);
  });

  it("handles binary data with embedded nulls in content", () => {
    const data = Buffer.from("CMD \x00arg\r\n");
    framer.feed(data);
    const frames = framer.frames();
    expect(frames).toHaveLength(1);
    // The null byte is preserved — framing is byte-level, not text-level.
    expect(frames[0][4]).toBe(0x00);
  });

  it("bufferSize reflects accumulated bytes", () => {
    framer.feed(Buffer.from("ABC"));
    expect(framer.bufferSize).toBe(3);
    framer.feed(Buffer.from("DEF"));
    expect(framer.bufferSize).toBe(6);
    framer.feed(Buffer.from("\r\n"));
    framer.frames(); // drain
    expect(framer.bufferSize).toBe(0);
  });

  it("returns frames as Buffer instances, not strings", () => {
    framer.feed(Buffer.from("NICK alice\r\n"));
    const frames = framer.frames();
    expect(Buffer.isBuffer(frames[0])).toBe(true);
  });

  it("handles a PRIVMSG with spaces in trailing param", () => {
    framer.feed(Buffer.from("PRIVMSG #chan :hello world\r\n"));
    const frames = framer.frames();
    expect(frames[0].toString("utf-8")).toBe("PRIVMSG #chan :hello world");
  });
});
