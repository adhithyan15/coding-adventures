/**
 * Tests for irc-framing — Framer class.
 */

import assert from "node:assert/strict";
import { beforeEach, describe, it } from "node:test";
import { Framer } from "../src/index.js";

describe("Framer", () => {
  let framer: Framer;

  beforeEach(() => {
    framer = new Framer();
  });

  it("starts with an empty buffer", () => {
    assert.equal(framer.bufferSize, 0);
    assert.deepEqual(framer.frames(), []);
  });

  it("returns no frames when no newline yet", () => {
    framer.feed(Buffer.from("NICK ali"));
    assert.deepEqual(framer.frames(), []);
    assert.equal(framer.bufferSize, 8);
  });

  it("returns one frame on CRLF-terminated message", () => {
    framer.feed(Buffer.from("NICK alice\r\n"));
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    assert.equal(frames[0].toString("utf-8"), "NICK alice");
  });

  it("returns one frame on LF-only-terminated message", () => {
    framer.feed(Buffer.from("NICK alice\n"));
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    assert.equal(frames[0].toString("utf-8"), "NICK alice");
  });

  it("handles partial message followed by completion", () => {
    framer.feed(Buffer.from("NICK ali"));
    assert.deepEqual(framer.frames(), []);
    framer.feed(Buffer.from("ce\r\n"));
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    assert.equal(frames[0].toString("utf-8"), "NICK alice");
  });

  it("handles multiple messages in one feed", () => {
    framer.feed(Buffer.from("NICK alice\r\nJOIN #general\r\n"));
    const frames = framer.frames();
    assert.equal(frames.length, 2);
    assert.equal(frames[0].toString("utf-8"), "NICK alice");
    assert.equal(frames[1].toString("utf-8"), "JOIN #general");
  });

  it("handles three messages in one feed", () => {
    framer.feed(Buffer.from("A\r\nB\r\nC\r\n"));
    const frames = framer.frames();
    assert.equal(frames.length, 3);
    assert.equal(frames[0].toString("utf-8"), "A");
    assert.equal(frames[1].toString("utf-8"), "B");
    assert.equal(frames[2].toString("utf-8"), "C");
  });

  it("accumulates partial messages across multiple feeds", () => {
    framer.feed(Buffer.from("PRIV"));
    framer.feed(Buffer.from("MSG #c"));
    framer.feed(Buffer.from("han :hello\r\n"));
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    assert.equal(frames[0].toString("utf-8"), "PRIVMSG #chan :hello");
  });

  it("leaves partial data in buffer after extracting complete frames", () => {
    framer.feed(Buffer.from("NICK alice\r\nJOIN #g"));
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    assert.equal(framer.bufferSize, "JOIN #g".length);
  });

  it("discards overlong lines (>510 bytes)", () => {
    // Create a line of 511 bytes followed by CRLF — should be discarded.
    const longLine = Buffer.alloc(511, 0x41); // 511 'A' bytes
    const msg = Buffer.concat([longLine, Buffer.from("\r\n")]);
    framer.feed(msg);
    const frames = framer.frames();
    assert.equal(frames.length, 0);
  });

  it("accepts exactly 510-byte line (boundary)", () => {
    const line510 = Buffer.alloc(510, 0x41); // exactly 510 'A' bytes
    const msg = Buffer.concat([line510, Buffer.from("\r\n")]);
    framer.feed(msg);
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    assert.equal(frames[0].length, 510);
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
    assert.equal(frames.length, 1);
    assert.equal(frames[0].toString("utf-8"), "NICK alice");
  });

  it("reset() clears all buffered data", () => {
    framer.feed(Buffer.from("partial data without newline"));
    assert.ok(framer.bufferSize > 0);
    framer.reset();
    assert.equal(framer.bufferSize, 0);
    assert.deepEqual(framer.frames(), []);
  });

  it("reset() prevents stale data from bleeding into new session", () => {
    framer.feed(Buffer.from("NICK alice\r\nJOIN #c"));
    framer.frames(); // drain the complete frame
    framer.reset();  // simulate connection close
    framer.feed(Buffer.from("USER bob 0 * :Bob\r\n"));
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    assert.equal(frames[0].toString("utf-8"), "USER bob 0 * :Bob");
  });

  it("handles empty feed call gracefully", () => {
    framer.feed(Buffer.alloc(0));
    assert.equal(framer.bufferSize, 0);
    assert.deepEqual(framer.frames(), []);
  });

  it("handles message split exactly on CR/LF boundary", () => {
    framer.feed(Buffer.from("NICK alice\r"));
    assert.equal(framer.frames().length, 0);
    framer.feed(Buffer.from("\n"));
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    assert.equal(frames[0].toString("utf-8"), "NICK alice");
  });

  it("handles LF as first byte (empty line)", () => {
    framer.feed(Buffer.from("\n"));
    const frames = framer.frames();
    // An empty line (zero content bytes) is under the 510-byte limit, so
    // it is yielded as an empty buffer.
    assert.equal(frames.length, 1);
    assert.equal(frames[0].length, 0);
  });

  it("handles binary data with embedded nulls in content", () => {
    const data = Buffer.from("CMD \x00arg\r\n");
    framer.feed(data);
    const frames = framer.frames();
    assert.equal(frames.length, 1);
    // The null byte is preserved — framing is byte-level, not text-level.
    assert.equal(frames[0][4], 0x00);
  });

  it("bufferSize reflects accumulated bytes", () => {
    framer.feed(Buffer.from("ABC"));
    assert.equal(framer.bufferSize, 3);
    framer.feed(Buffer.from("DEF"));
    assert.equal(framer.bufferSize, 6);
    framer.feed(Buffer.from("\r\n"));
    framer.frames(); // drain
    assert.equal(framer.bufferSize, 0);
  });

  it("returns frames as Buffer instances, not strings", () => {
    framer.feed(Buffer.from("NICK alice\r\n"));
    const frames = framer.frames();
    assert.equal(Buffer.isBuffer(frames[0]), true);
  });

  it("handles a PRIVMSG with spaces in trailing param", () => {
    framer.feed(Buffer.from("PRIVMSG #chan :hello world\r\n"));
    const frames = framer.frames();
    assert.equal(frames[0].toString("utf-8"), "PRIVMSG #chan :hello world");
  });
});
