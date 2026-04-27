/**
 * Tests for irc-proto — parse() and serialize() round-trips.
 */

import { describe, it, expect } from "vitest";
import { parse, serialize, ParseError, Message } from "../src/index.js";

describe("parse()", () => {
  it("parses a simple command with no prefix and one param", () => {
    const msg = parse("NICK alice");
    expect(msg.prefix).toBeNull();
    expect(msg.command).toBe("NICK");
    expect(msg.params).toEqual(["alice"]);
  });

  it("parses a command with no prefix and no params", () => {
    const msg = parse("QUIT");
    expect(msg.prefix).toBeNull();
    expect(msg.command).toBe("QUIT");
    expect(msg.params).toEqual([]);
  });

  it("parses a command with a server prefix", () => {
    const msg = parse(":irc.local 001 alice :Welcome to the server");
    expect(msg.prefix).toBe("irc.local");
    expect(msg.command).toBe("001");
    expect(msg.params).toEqual(["alice", "Welcome to the server"]);
  });

  it("parses a command with a nick!user@host prefix", () => {
    const msg = parse(":alice!alice@127.0.0.1 PRIVMSG #general :hello world");
    expect(msg.prefix).toBe("alice!alice@127.0.0.1");
    expect(msg.command).toBe("PRIVMSG");
    expect(msg.params).toEqual(["#general", "hello world"]);
  });

  it("normalises command to uppercase", () => {
    const msg = parse("join #foo");
    expect(msg.command).toBe("JOIN");
    expect(msg.params).toEqual(["#foo"]);
  });

  it("handles trailing param that contains multiple spaces", () => {
    const msg = parse("PRIVMSG #chan :hello   world   spaces");
    expect(msg.params).toEqual(["#chan", "hello   world   spaces"]);
  });

  it("parses USER command with 4 params", () => {
    const msg = parse("USER alice 0 * :Alice Smith");
    expect(msg.command).toBe("USER");
    expect(msg.params).toEqual(["alice", "0", "*", "Alice Smith"]);
  });

  it("parses PING command", () => {
    const msg = parse("PING :irc.local");
    expect(msg.command).toBe("PING");
    expect(msg.params).toEqual(["irc.local"]);
  });

  it("parses numeric commands", () => {
    const msg = parse(":server 433 * alice :Nickname is already in use");
    expect(msg.command).toBe("433");
    expect(msg.params).toEqual(["*", "alice", "Nickname is already in use"]);
  });

  it("strips the leading colon from trailing param", () => {
    const msg = parse(":s 001 nick :Welcome!");
    expect(msg.params[1]).toBe("Welcome!");
    expect(msg.params[1].startsWith(":")).toBe(false);
  });

  it("handles param that is a colon-only trailing (empty string)", () => {
    const msg = parse("PART #chan :");
    expect(msg.params).toEqual(["#chan", ""]);
  });

  it("handles multiple params with no trailing", () => {
    const msg = parse("MODE #chan +o alice");
    expect(msg.params).toEqual(["#chan", "+o", "alice"]);
  });

  it("enforces max 15 params — extra tokens are silently dropped", () => {
    const manyParams = Array.from({ length: 20 }, (_, i) => `p${i}`).join(" ");
    const msg = parse(`CMD ${manyParams}`);
    expect(msg.params.length).toBe(15);
    expect(msg.params[14]).toBe("p14");
  });

  it("parses QUIT with prefix", () => {
    const msg = parse(":alice!alice@host QUIT :Goodbye");
    expect(msg.prefix).toBe("alice!alice@host");
    expect(msg.command).toBe("QUIT");
    expect(msg.params).toEqual(["Goodbye"]);
  });

  it("parses JOIN with no trailing", () => {
    const msg = parse("JOIN #general");
    expect(msg.command).toBe("JOIN");
    expect(msg.params).toEqual(["#general"]);
  });

  it("parses NAMES reply numeric 353", () => {
    const msg = parse(":irc.local 353 alice = #general :@alice bob");
    expect(msg.command).toBe("353");
    expect(msg.params).toEqual(["alice", "=", "#general", "@alice bob"]);
  });

  it("handles command-only (CAP LS)", () => {
    const msg = parse("CAP LS");
    expect(msg.prefix).toBeNull();
    expect(msg.command).toBe("CAP");
    expect(msg.params).toEqual(["LS"]);
  });
});

describe("parse() — error cases", () => {
  it("throws ParseError for empty string", () => {
    expect(() => parse("")).toThrow(ParseError);
  });

  it("throws ParseError for whitespace-only string", () => {
    expect(() => parse("   ")).toThrow(ParseError);
  });

  it("throws ParseError for prefix-only line (no command)", () => {
    expect(() => parse(":irc.local")).toThrow(ParseError);
  });
});

describe("serialize()", () => {
  it("serializes a simple command with no prefix", () => {
    const buf = serialize({ prefix: null, command: "NICK", params: ["alice"] });
    expect(buf.toString("utf-8")).toBe("NICK alice\r\n");
  });

  it("serializes a command with a server prefix", () => {
    const buf = serialize({
      prefix: "irc.local",
      command: "001",
      params: ["alice", "Welcome to the server"],
    });
    expect(buf.toString("utf-8")).toBe(":irc.local 001 alice :Welcome to the server\r\n");
  });

  it("adds trailing colon for last param containing a space", () => {
    const buf = serialize({
      prefix: null,
      command: "PRIVMSG",
      params: ["#chan", "hello world"],
    });
    expect(buf.toString("utf-8")).toBe("PRIVMSG #chan :hello world\r\n");
  });

  it("does not add trailing colon for last param without a space", () => {
    const buf = serialize({ prefix: null, command: "JOIN", params: ["#general"] });
    expect(buf.toString("utf-8")).toBe("JOIN #general\r\n");
  });

  it("serializes with no params", () => {
    const buf = serialize({ prefix: null, command: "QUIT", params: [] });
    expect(buf.toString("utf-8")).toBe("QUIT\r\n");
  });

  it("returns a Buffer, not a string", () => {
    const buf = serialize({ prefix: null, command: "PING", params: [] });
    expect(Buffer.isBuffer(buf)).toBe(true);
  });

  it("always terminates with CRLF", () => {
    const buf = serialize({ prefix: null, command: "PONG", params: ["server"] });
    const s = buf.toString("utf-8");
    expect(s.endsWith("\r\n")).toBe(true);
  });
});

describe("parse() -> serialize() round-trips", () => {
  const roundTrip = (line: string): string => {
    const msg = parse(line);
    return serialize(msg).toString("utf-8");
  };

  it("round-trips NICK", () => {
    expect(roundTrip("NICK alice")).toBe("NICK alice\r\n");
  });

  it("round-trips PRIVMSG with trailing", () => {
    expect(roundTrip(":alice!a@h PRIVMSG #c :hello world")).toBe(
      ":alice!a@h PRIVMSG #c :hello world\r\n"
    );
  });

  it("round-trips 001 welcome numeric (no colon needed for single word)", () => {
    // "Welcome!" has no spaces, so the serializer does NOT add a trailing colon.
    // The colon is only needed for params with spaces (the "trailing" param signal).
    // This is correct per RFC 1459: the colon is a wire-format framing hint, not
    // preserved for non-space params.
    expect(roundTrip(":irc.local 001 alice :Welcome!")).toBe(
      ":irc.local 001 alice Welcome!\r\n"
    );
  });

  it("round-trips QUIT with no params", () => {
    expect(roundTrip("QUIT")).toBe("QUIT\r\n");
  });

  it("round-trips JOIN", () => {
    expect(roundTrip("JOIN #general")).toBe("JOIN #general\r\n");
  });

  it("round-trips USER with realname", () => {
    const msg = parse("USER alice 0 * :Alice Smith");
    const out = serialize(msg).toString("utf-8");
    expect(out).toBe("USER alice 0 * :Alice Smith\r\n");
  });

  it("round-trips PING with server token (no colon for single word)", () => {
    // "irc.local" has no spaces, so no trailing colon is added.
    expect(roundTrip("PING :irc.local")).toBe("PING irc.local\r\n");
  });
});

describe("Message interface", () => {
  it("accepts null prefix", () => {
    const msg: Message = { prefix: null, command: "NICK", params: ["bob"] };
    expect(msg.prefix).toBeNull();
  });

  it("accepts string prefix", () => {
    const msg: Message = { prefix: "irc.local", command: "001", params: [] };
    expect(msg.prefix).toBe("irc.local");
  });

  it("params can be empty array", () => {
    const msg: Message = { prefix: null, command: "QUIT", params: [] };
    expect(msg.params).toHaveLength(0);
  });
});
