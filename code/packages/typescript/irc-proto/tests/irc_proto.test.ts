/**
 * Tests for irc-proto — parse() and serialize() round-trips.
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { parse, serialize, ParseError, Message } from "../src/index.js";

describe("parse()", () => {
  it("parses a simple command with no prefix and one param", () => {
    const msg = parse("NICK alice");
    assert.equal(msg.prefix, null);
    assert.equal(msg.command, "NICK");
    assert.deepEqual(msg.params, ["alice"]);
  });

  it("parses a command with no prefix and no params", () => {
    const msg = parse("QUIT");
    assert.equal(msg.prefix, null);
    assert.equal(msg.command, "QUIT");
    assert.deepEqual(msg.params, []);
  });

  it("parses a command with a server prefix", () => {
    const msg = parse(":irc.local 001 alice :Welcome to the server");
    assert.equal(msg.prefix, "irc.local");
    assert.equal(msg.command, "001");
    assert.deepEqual(msg.params, ["alice", "Welcome to the server"]);
  });

  it("parses a command with a nick!user@host prefix", () => {
    const msg = parse(":alice!alice@127.0.0.1 PRIVMSG #general :hello world");
    assert.equal(msg.prefix, "alice!alice@127.0.0.1");
    assert.equal(msg.command, "PRIVMSG");
    assert.deepEqual(msg.params, ["#general", "hello world"]);
  });

  it("normalises command to uppercase", () => {
    const msg = parse("join #foo");
    assert.equal(msg.command, "JOIN");
    assert.deepEqual(msg.params, ["#foo"]);
  });

  it("handles trailing param that contains multiple spaces", () => {
    const msg = parse("PRIVMSG #chan :hello   world   spaces");
    assert.deepEqual(msg.params, ["#chan", "hello   world   spaces"]);
  });

  it("parses USER command with 4 params", () => {
    const msg = parse("USER alice 0 * :Alice Smith");
    assert.equal(msg.command, "USER");
    assert.deepEqual(msg.params, ["alice", "0", "*", "Alice Smith"]);
  });

  it("parses PING command", () => {
    const msg = parse("PING :irc.local");
    assert.equal(msg.command, "PING");
    assert.deepEqual(msg.params, ["irc.local"]);
  });

  it("parses numeric commands", () => {
    const msg = parse(":server 433 * alice :Nickname is already in use");
    assert.equal(msg.command, "433");
    assert.deepEqual(msg.params, ["*", "alice", "Nickname is already in use"]);
  });

  it("strips the leading colon from trailing param", () => {
    const msg = parse(":s 001 nick :Welcome!");
    assert.equal(msg.params[1], "Welcome!");
    assert.equal(msg.params[1].startsWith(":"), false);
  });

  it("handles param that is a colon-only trailing (empty string)", () => {
    const msg = parse("PART #chan :");
    assert.deepEqual(msg.params, ["#chan", ""]);
  });

  it("handles multiple params with no trailing", () => {
    const msg = parse("MODE #chan +o alice");
    assert.deepEqual(msg.params, ["#chan", "+o", "alice"]);
  });

  it("enforces max 15 params — extra tokens are silently dropped", () => {
    const manyParams = Array.from({ length: 20 }, (_, i) => `p${i}`).join(" ");
    const msg = parse(`CMD ${manyParams}`);
    assert.equal(msg.params.length, 15);
    assert.equal(msg.params[14], "p14");
  });

  it("parses QUIT with prefix", () => {
    const msg = parse(":alice!alice@host QUIT :Goodbye");
    assert.equal(msg.prefix, "alice!alice@host");
    assert.equal(msg.command, "QUIT");
    assert.deepEqual(msg.params, ["Goodbye"]);
  });

  it("parses JOIN with no trailing", () => {
    const msg = parse("JOIN #general");
    assert.equal(msg.command, "JOIN");
    assert.deepEqual(msg.params, ["#general"]);
  });

  it("parses NAMES reply numeric 353", () => {
    const msg = parse(":irc.local 353 alice = #general :@alice bob");
    assert.equal(msg.command, "353");
    assert.deepEqual(msg.params, ["alice", "=", "#general", "@alice bob"]);
  });

  it("handles command-only (CAP LS)", () => {
    const msg = parse("CAP LS");
    assert.equal(msg.prefix, null);
    assert.equal(msg.command, "CAP");
    assert.deepEqual(msg.params, ["LS"]);
  });
});

describe("parse() — error cases", () => {
  it("throws ParseError for empty string", () => {
    assert.throws(() => parse(""), ParseError);
  });

  it("throws ParseError for whitespace-only string", () => {
    assert.throws(() => parse("   "), ParseError);
  });

  it("throws ParseError for prefix-only line (no command)", () => {
    assert.throws(() => parse(":irc.local"), ParseError);
  });
});

describe("serialize()", () => {
  it("serializes a simple command with no prefix", () => {
    const buf = serialize({ prefix: null, command: "NICK", params: ["alice"] });
    assert.equal(buf.toString("utf-8"), "NICK alice\r\n");
  });

  it("serializes a command with a server prefix", () => {
    const buf = serialize({
      prefix: "irc.local",
      command: "001",
      params: ["alice", "Welcome to the server"],
    });
    assert.equal(buf.toString("utf-8"), ":irc.local 001 alice :Welcome to the server\r\n");
  });

  it("adds trailing colon for last param containing a space", () => {
    const buf = serialize({
      prefix: null,
      command: "PRIVMSG",
      params: ["#chan", "hello world"],
    });
    assert.equal(buf.toString("utf-8"), "PRIVMSG #chan :hello world\r\n");
  });

  it("does not add trailing colon for last param without a space", () => {
    const buf = serialize({ prefix: null, command: "JOIN", params: ["#general"] });
    assert.equal(buf.toString("utf-8"), "JOIN #general\r\n");
  });

  it("serializes with no params", () => {
    const buf = serialize({ prefix: null, command: "QUIT", params: [] });
    assert.equal(buf.toString("utf-8"), "QUIT\r\n");
  });

  it("returns a Buffer, not a string", () => {
    const buf = serialize({ prefix: null, command: "PING", params: [] });
    assert.equal(Buffer.isBuffer(buf), true);
  });

  it("always terminates with CRLF", () => {
    const buf = serialize({ prefix: null, command: "PONG", params: ["server"] });
    const s = buf.toString("utf-8");
    assert.equal(s.endsWith("\r\n"), true);
  });
});

describe("parse() -> serialize() round-trips", () => {
  const roundTrip = (line: string): string => {
    const msg = parse(line);
    return serialize(msg).toString("utf-8");
  };

  it("round-trips NICK", () => {
    assert.equal(roundTrip("NICK alice"), "NICK alice\r\n");
  });

  it("round-trips PRIVMSG with trailing", () => {
    assert.equal(
      roundTrip(":alice!a@h PRIVMSG #c :hello world"),
      ":alice!a@h PRIVMSG #c :hello world\r\n",
    );
  });

  it("round-trips 001 welcome numeric (no colon needed for single word)", () => {
    // "Welcome!" has no spaces, so the serializer does NOT add a trailing colon.
    // The colon is only needed for params with spaces (the "trailing" param signal).
    // This is correct per RFC 1459: the colon is a wire-format framing hint, not
    // preserved for non-space params.
    assert.equal(
      roundTrip(":irc.local 001 alice :Welcome!"),
      ":irc.local 001 alice Welcome!\r\n",
    );
  });

  it("round-trips QUIT with no params", () => {
    assert.equal(roundTrip("QUIT"), "QUIT\r\n");
  });

  it("round-trips JOIN", () => {
    assert.equal(roundTrip("JOIN #general"), "JOIN #general\r\n");
  });

  it("round-trips USER with realname", () => {
    const msg = parse("USER alice 0 * :Alice Smith");
    const out = serialize(msg).toString("utf-8");
    assert.equal(out, "USER alice 0 * :Alice Smith\r\n");
  });

  it("round-trips PING with server token (no colon for single word)", () => {
    // "irc.local" has no spaces, so no trailing colon is added.
    assert.equal(roundTrip("PING :irc.local"), "PING irc.local\r\n");
  });
});

describe("Message interface", () => {
  it("accepts null prefix", () => {
    const msg: Message = { prefix: null, command: "NICK", params: ["bob"] };
    assert.equal(msg.prefix, null);
  });

  it("accepts string prefix", () => {
    const msg: Message = { prefix: "irc.local", command: "001", params: [] };
    assert.equal(msg.prefix, "irc.local");
  });

  it("params can be empty array", () => {
    const msg: Message = { prefix: null, command: "QUIT", params: [] };
    assert.equal(msg.params.length, 0);
  });
});
