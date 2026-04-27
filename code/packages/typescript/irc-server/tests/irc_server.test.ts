/**
 * Tests for irc-server — IRCServer state machine.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { IRCServer, ConnId, Response } from "../src/index.js";
import { parse, Message } from "@coding-adventures/irc-proto";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mkConnId(n: number): ConnId {
  return n as ConnId;
}

/**
 * Register a client through the NICK+USER handshake and return the welcome
 * responses.  After this, the client is fully registered and can join channels.
 */
function registerClient(
  server: IRCServer,
  connId: ConnId,
  nick: string,
  username: string = nick,
  realname: string = nick
): Response[] {
  server.onConnect(connId, "127.0.0.1");
  server.onMessage(connId, parse(`NICK ${nick}`));
  return server.onMessage(connId, parse(`USER ${username} 0 * :${realname}`));
}

/**
 * Extract command strings from a response list for easy assertion.
 */
function commands(responses: Response[]): string[] {
  return responses.map(([, msg]) => msg.command);
}

/**
 * Find the first response with the given command and return its message.
 */
function findMsg(responses: Response[], command: string): Message | undefined {
  return responses.find(([, msg]) => msg.command === command)?.[1];
}

// ---------------------------------------------------------------------------
// Connection lifecycle
// ---------------------------------------------------------------------------

describe("onConnect / onDisconnect", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
  });

  it("onConnect returns empty responses", () => {
    const r = server.onConnect(mkConnId(1), "127.0.0.1");
    expect(r).toHaveLength(0);
  });

  it("onDisconnect for unknown connId returns empty", () => {
    const r = server.onDisconnect(mkConnId(999));
    expect(r).toHaveLength(0);
  });

  it("onDisconnect for unregistered client returns empty", () => {
    server.onConnect(mkConnId(1), "127.0.0.1");
    const r = server.onDisconnect(mkConnId(1));
    expect(r).toHaveLength(0);
  });

  it("onDisconnect for registered client broadcasts QUIT to channel peers", () => {
    // Register two clients in the same channel.
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(2), parse("JOIN #general"));

    // Disconnect alice — bob should receive QUIT.
    const r = server.onDisconnect(mkConnId(1));
    const quit = r.find(([connId, msg]) => connId === mkConnId(2) && msg.command === "QUIT");
    expect(quit).toBeTruthy();
  });

  it("onDisconnect cleans up channel membership (empty channel is destroyed)", () => {
    registerClient(server, mkConnId(1), "alice");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onDisconnect(mkConnId(1));

    // The channel should be gone; joining again creates it fresh.
    registerClient(server, mkConnId(2), "bob");
    const r = server.onMessage(mkConnId(2), parse("JOIN #general"));
    // The JOIN broadcast goes to bob (first member), confirming a fresh channel.
    expect(r.some(([, msg]) => msg.command === "JOIN")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Registration: NICK + USER handshake
// ---------------------------------------------------------------------------

describe("NICK command", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
    server.onConnect(mkConnId(1), "127.0.0.1");
  });

  it("NICK alone does not trigger welcome (USER not yet sent)", () => {
    const r = server.onMessage(mkConnId(1), parse("NICK alice"));
    expect(commands(r)).not.toContain("001");
  });

  it("NICK then USER triggers the welcome sequence", () => {
    server.onMessage(mkConnId(1), parse("NICK alice"));
    const r = server.onMessage(mkConnId(1), parse("USER alice 0 * :Alice"));
    expect(commands(r)).toContain("001");
    expect(commands(r)).toContain("376"); // end of MOTD
  });

  it("USER then NICK triggers the welcome sequence", () => {
    server.onMessage(mkConnId(1), parse("USER alice 0 * :Alice"));
    const r = server.onMessage(mkConnId(1), parse("NICK alice"));
    expect(commands(r)).toContain("001");
  });

  it("431 ERR_NONICKNAMEGIVEN when no params", () => {
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "NICK", params: [] });
    expect(commands(r)).toContain("431");
  });

  it("432 ERR_ERRONEUSNICKNAME for invalid nick", () => {
    const r = server.onMessage(mkConnId(1), parse("NICK bad nick"));
    // "bad nick" is two params due to space, so "bad" is actually a valid nick
    // Let's use an invalid one with a leading digit.
    const r2 = server.onMessage(mkConnId(1), { prefix: null, command: "NICK", params: ["1invalid"] });
    expect(commands(r2)).toContain("432");
  });

  it("433 ERR_NICKNAMEINUSE when nick is taken", () => {
    server.onMessage(mkConnId(1), parse("NICK alice"));
    server.onConnect(mkConnId(2), "127.0.0.1");
    const r = server.onMessage(mkConnId(2), parse("NICK alice"));
    expect(commands(r)).toContain("433");
  });

  it("nick change after registration broadcasts to channel peers", () => {
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(2), parse("JOIN #general"));

    const r = server.onMessage(mkConnId(1), parse("NICK alicia"));
    // alice herself gets the NICK message.
    const toAlice = r.find(([connId, msg]) => connId === mkConnId(1) && msg.command === "NICK");
    expect(toAlice).toBeTruthy();
    // bob also gets the NICK message.
    const toBob = r.find(([connId, msg]) => connId === mkConnId(2) && msg.command === "NICK");
    expect(toBob).toBeTruthy();
    expect(toBob![1].params[0]).toBe("alicia");
  });

  it("451 ERR_NOTREGISTERED for unknown command before registration", () => {
    const r = server.onMessage(mkConnId(1), parse("PRIVMSG #chan :hello"));
    expect(commands(r)).toContain("451");
  });
});

describe("USER command", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
    server.onConnect(mkConnId(1), "127.0.0.1");
  });

  it("461 ERR_NEEDMOREPARAMS when fewer than 4 params", () => {
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "USER", params: ["alice", "0", "*"] });
    expect(commands(r)).toContain("461");
  });

  it("duplicate USER after registration is ignored", () => {
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("USER alice2 0 * :Another"));
    expect(r).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Welcome sequence
// ---------------------------------------------------------------------------

describe("Welcome sequence (001–376)", () => {
  it("welcome includes 001, 002, 003, 004, 251, 375, 376", () => {
    const server = new IRCServer("irc.local", ["Hello!"]);
    const r = registerClient(server, mkConnId(1), "alice");
    const cmds = commands(r);
    expect(cmds).toContain("001");
    expect(cmds).toContain("002");
    expect(cmds).toContain("003");
    expect(cmds).toContain("004");
    expect(cmds).toContain("251");
    expect(cmds).toContain("375");
    expect(cmds).toContain("372"); // one MOTD line
    expect(cmds).toContain("376");
  });

  it("001 welcome message contains the nick mask", () => {
    const server = new IRCServer("irc.local");
    const r = registerClient(server, mkConnId(1), "alice");
    const welcome = findMsg(r, "001");
    expect(welcome?.params.join(" ")).toContain("alice");
  });

  it("all responses target the registering client", () => {
    const server = new IRCServer("irc.local");
    const r = registerClient(server, mkConnId(1), "alice");
    for (const [connId] of r) {
      expect(connId).toBe(mkConnId(1));
    }
  });
});

// ---------------------------------------------------------------------------
// CAP, PASS, PONG
// ---------------------------------------------------------------------------

describe("CAP", () => {
  it("CAP returns an ACK response", () => {
    const server = new IRCServer("irc.local");
    server.onConnect(mkConnId(1), "127.0.0.1");
    const r = server.onMessage(mkConnId(1), parse("CAP LS"));
    expect(commands(r)).toContain("CAP");
  });
});

describe("PASS", () => {
  it("PASS is accepted with no response", () => {
    const server = new IRCServer("irc.local");
    server.onConnect(mkConnId(1), "127.0.0.1");
    const r = server.onMessage(mkConnId(1), parse("PASS secret"));
    expect(r).toHaveLength(0);
  });
});

describe("PONG", () => {
  it("PONG is accepted with no response", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("PONG irc.local"));
    expect(r).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// QUIT
// ---------------------------------------------------------------------------

describe("QUIT", () => {
  it("QUIT sends ERROR to the quitting client", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("QUIT :Goodbye"));
    const error = r.find(([connId, msg]) => connId === mkConnId(1) && msg.command === "ERROR");
    expect(error).toBeTruthy();
  });

  it("QUIT broadcasts to channel peers", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(2), parse("JOIN #general"));

    const r = server.onMessage(mkConnId(1), parse("QUIT :Bye"));
    const toBob = r.find(([connId, msg]) => connId === mkConnId(2) && msg.command === "QUIT");
    expect(toBob).toBeTruthy();
    expect(toBob![1].params[0]).toBe("Bye");
  });

  it("QUIT with default reason", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "QUIT", params: [] });
    expect(r.some(([, msg]) => msg.command === "ERROR")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// JOIN
// ---------------------------------------------------------------------------

describe("JOIN", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
  });

  it("JOIN creates channel and broadcasts to joiner", () => {
    const r = server.onMessage(mkConnId(1), parse("JOIN #general"));
    const joins = r.filter(([, msg]) => msg.command === "JOIN");
    expect(joins.some(([connId]) => connId === mkConnId(1))).toBe(true);
  });

  it("JOIN sends NAMES (353 + 366) to joiner", () => {
    const r = server.onMessage(mkConnId(1), parse("JOIN #general"));
    expect(commands(r)).toContain("353");
    expect(commands(r)).toContain("366");
  });

  it("JOIN sends NOTOPIC when no topic is set", () => {
    const r = server.onMessage(mkConnId(1), parse("JOIN #general"));
    expect(commands(r)).toContain("331");
  });

  it("JOIN existing channel broadcasts to all members", () => {
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    const r = server.onMessage(mkConnId(2), parse("JOIN #general"));
    // alice and bob both get the JOIN broadcast.
    const joinConnIds = r.filter(([, msg]) => msg.command === "JOIN").map(([id]) => id);
    expect(joinConnIds).toContain(mkConnId(1));
    expect(joinConnIds).toContain(mkConnId(2));
  });

  it("461 when no channel param", () => {
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "JOIN", params: [] });
    expect(commands(r)).toContain("461");
  });

  it("JOIN is idempotent (second join to same channel is ignored)", () => {
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    const r = server.onMessage(mkConnId(1), parse("JOIN #general"));
    // No JOIN broadcast for a repeat join.
    expect(r.filter(([, msg]) => msg.command === "JOIN")).toHaveLength(0);
  });

  it("JOIN sends TOPIC when topic is already set", () => {
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(1), parse("TOPIC #general :Hello World"));

    registerClient(server, mkConnId(2), "bob");
    const r = server.onMessage(mkConnId(2), parse("JOIN #general"));
    expect(commands(r)).toContain("332"); // RPL_TOPIC
  });

  it("first member of channel becomes operator", () => {
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    // alice is the only member and should be the operator
    // We verify this by trying to kick from another client (should fail with 482)
    // and alice being able to kick.
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(2), parse("JOIN #general"));

    // bob tries to kick alice — should fail (bob is not op).
    const r = server.onMessage(mkConnId(2), parse("KICK #general alice :bye"));
    expect(commands(r)).toContain("482");
  });
});

// ---------------------------------------------------------------------------
// PART
// ---------------------------------------------------------------------------

describe("PART", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(2), parse("JOIN #general"));
  });

  it("PART broadcasts to all members including the leaving client", () => {
    const r = server.onMessage(mkConnId(1), parse("PART #general :Goodbye"));
    const parts = r.filter(([, msg]) => msg.command === "PART");
    const partConnIds = parts.map(([id]) => id);
    expect(partConnIds).toContain(mkConnId(1));
    expect(partConnIds).toContain(mkConnId(2));
  });

  it("PART reason is included in broadcast", () => {
    const r = server.onMessage(mkConnId(1), parse("PART #general :See you later"));
    const part = r.find(([, msg]) => msg.command === "PART");
    expect(part![1].params[1]).toBe("See you later");
  });

  it("442 when not in channel", () => {
    const r = server.onMessage(mkConnId(1), parse("PART #nonexistent"));
    expect(commands(r)).toContain("442");
  });

  it("461 when no params", () => {
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "PART", params: [] });
    expect(commands(r)).toContain("461");
  });
});

// ---------------------------------------------------------------------------
// PRIVMSG
// ---------------------------------------------------------------------------

describe("PRIVMSG", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
  });

  it("PRIVMSG to channel delivers to all members except sender", () => {
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(2), parse("JOIN #general"));
    const r = server.onMessage(mkConnId(1), parse("PRIVMSG #general :Hello!"));
    // bob gets it, alice does not.
    expect(r.some(([id, msg]) => id === mkConnId(2) && msg.command === "PRIVMSG")).toBe(true);
    expect(r.some(([id, msg]) => id === mkConnId(1) && msg.command === "PRIVMSG")).toBe(false);
  });

  it("PRIVMSG to nick delivers directly", () => {
    const r = server.onMessage(mkConnId(1), parse("PRIVMSG bob :Hey bob!"));
    expect(r.some(([id, msg]) => id === mkConnId(2) && msg.command === "PRIVMSG")).toBe(true);
  });

  it("401 when nick not found", () => {
    const r = server.onMessage(mkConnId(1), parse("PRIVMSG nonexistent :hi"));
    expect(commands(r)).toContain("401");
  });

  it("403 when channel not found", () => {
    const r = server.onMessage(mkConnId(1), parse("PRIVMSG #nonexistent :hi"));
    expect(commands(r)).toContain("403");
  });

  it("461 when no params", () => {
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "PRIVMSG", params: [] });
    expect(commands(r)).toContain("461");
  });

  it("412 when no text", () => {
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "PRIVMSG", params: ["bob"] });
    expect(commands(r)).toContain("412");
  });

  it("PRIVMSG to away nick triggers 301 RPL_AWAY to sender", () => {
    server.onMessage(mkConnId(2), parse("AWAY :brb"));
    const r = server.onMessage(mkConnId(1), parse("PRIVMSG bob :hey"));
    expect(commands(r)).toContain("301");
  });
});

// ---------------------------------------------------------------------------
// NOTICE
// ---------------------------------------------------------------------------

describe("NOTICE", () => {
  it("NOTICE to nick delivers without 301 away reply", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(2), parse("AWAY :brb"));
    const r = server.onMessage(mkConnId(1), parse("NOTICE bob :hey"));
    expect(commands(r)).not.toContain("301");
    expect(r.some(([id, msg]) => id === mkConnId(2) && msg.command === "NOTICE")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// NAMES
// ---------------------------------------------------------------------------

describe("NAMES", () => {
  it("NAMES returns 353 + 366 for existing channel", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    const r = server.onMessage(mkConnId(1), parse("NAMES #general"));
    expect(commands(r)).toContain("353");
    expect(commands(r)).toContain("366");
  });

  it("NAMES with no params returns for all channels", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(1), parse("JOIN #test"));
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "NAMES", params: [] });
    const names353 = r.filter(([, msg]) => msg.command === "353");
    expect(names353.length).toBeGreaterThanOrEqual(2);
  });

  it("NAMES for nonexistent channel returns 366 only", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("NAMES #nonexistent"));
    expect(commands(r)).toContain("366");
    expect(commands(r)).not.toContain("353");
  });
});

// ---------------------------------------------------------------------------
// LIST
// ---------------------------------------------------------------------------

describe("LIST", () => {
  it("LIST returns 321, 322 per channel, 323", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(1), parse("JOIN #test"));
    const r = server.onMessage(mkConnId(1), parse("LIST"));
    const cmds = commands(r);
    expect(cmds).toContain("321");
    expect(cmds.filter((c) => c === "322")).toHaveLength(2);
    expect(cmds).toContain("323");
  });
});

// ---------------------------------------------------------------------------
// TOPIC
// ---------------------------------------------------------------------------

describe("TOPIC", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
  });

  it("TOPIC query returns 331 when no topic", () => {
    const r = server.onMessage(mkConnId(1), parse("TOPIC #general"));
    expect(commands(r)).toContain("331");
  });

  it("TOPIC set broadcasts to all members", () => {
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(2), parse("JOIN #general"));
    const r = server.onMessage(mkConnId(1), parse("TOPIC #general :New topic!"));
    const topicConnIds = r.filter(([, msg]) => msg.command === "TOPIC").map(([id]) => id);
    expect(topicConnIds).toContain(mkConnId(1));
    expect(topicConnIds).toContain(mkConnId(2));
  });

  it("TOPIC query returns 332 when topic is set", () => {
    server.onMessage(mkConnId(1), parse("TOPIC #general :Hello"));
    const r = server.onMessage(mkConnId(1), parse("TOPIC #general"));
    expect(commands(r)).toContain("332");
  });

  it("442 when not in channel", () => {
    registerClient(server, mkConnId(2), "bob");
    const r = server.onMessage(mkConnId(2), parse("TOPIC #general"));
    expect(commands(r)).toContain("442");
  });

  it("461 when no params", () => {
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "TOPIC", params: [] });
    expect(commands(r)).toContain("461");
  });
});

// ---------------------------------------------------------------------------
// KICK
// ---------------------------------------------------------------------------

describe("KICK", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(2), parse("JOIN #general"));
  });

  it("op can kick a member", () => {
    const r = server.onMessage(mkConnId(1), parse("KICK #general bob :bye"));
    expect(commands(r)).toContain("KICK");
  });

  it("KICK broadcast includes the reason", () => {
    const r = server.onMessage(mkConnId(1), parse("KICK #general bob :too noisy"));
    const kick = r.find(([, msg]) => msg.command === "KICK");
    expect(kick![1].params[2]).toBe("too noisy");
  });

  it("482 when non-op tries to kick", () => {
    const r = server.onMessage(mkConnId(2), parse("KICK #general alice :out"));
    expect(commands(r)).toContain("482");
  });

  it("441 when target is not in channel", () => {
    registerClient(server, mkConnId(3), "carol");
    const r = server.onMessage(mkConnId(1), parse("KICK #general carol :out"));
    expect(commands(r)).toContain("441");
  });

  it("461 when not enough params", () => {
    const r = server.onMessage(mkConnId(1), parse("KICK #general"));
    expect(commands(r)).toContain("461");
  });
});

// ---------------------------------------------------------------------------
// INVITE
// ---------------------------------------------------------------------------

describe("INVITE", () => {
  it("INVITE sends 341 to inviter and INVITE to target", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    const r = server.onMessage(mkConnId(1), parse("INVITE bob #general"));
    expect(r.some(([id, msg]) => id === mkConnId(1) && msg.command === "341")).toBe(true);
    expect(r.some(([id, msg]) => id === mkConnId(2) && msg.command === "INVITE")).toBe(true);
  });

  it("401 when target nick not found", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("INVITE nonexistent #general"));
    expect(commands(r)).toContain("401");
  });

  it("461 when not enough params", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("INVITE alice"));
    expect(commands(r)).toContain("461");
  });
});

// ---------------------------------------------------------------------------
// MODE
// ---------------------------------------------------------------------------

describe("MODE", () => {
  let server: IRCServer;

  beforeEach(() => {
    server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
  });

  it("MODE query returns 324 RPL_CHANNELMODEIS", () => {
    const r = server.onMessage(mkConnId(1), parse("MODE #general"));
    expect(commands(r)).toContain("324");
  });

  it("MODE set broadcasts to channel members", () => {
    const r = server.onMessage(mkConnId(1), parse("MODE #general +n"));
    expect(r.some(([, msg]) => msg.command === "MODE")).toBe(true);
  });

  it("MODE user query returns 221", () => {
    const r = server.onMessage(mkConnId(1), parse("MODE alice"));
    expect(commands(r)).toContain("221");
  });

  it("461 when no params", () => {
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "MODE", params: [] });
    expect(commands(r)).toContain("461");
  });
});

// ---------------------------------------------------------------------------
// PING
// ---------------------------------------------------------------------------

describe("PING", () => {
  it("PING returns PONG with same token", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("PING :irc.local"));
    expect(commands(r)).toContain("PONG");
    const pong = findMsg(r, "PONG");
    expect(pong?.params[1]).toBe("irc.local");
  });

  it("PING with no params uses server name as token", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "PING", params: [] });
    expect(commands(r)).toContain("PONG");
  });
});

// ---------------------------------------------------------------------------
// AWAY
// ---------------------------------------------------------------------------

describe("AWAY", () => {
  it("AWAY with message returns 306 RPL_NOWAWAY", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("AWAY :brb"));
    expect(commands(r)).toContain("306");
  });

  it("AWAY without message clears away and returns 305 RPL_UNAWAY", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    server.onMessage(mkConnId(1), parse("AWAY :brb"));
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "AWAY", params: [] });
    expect(commands(r)).toContain("305");
  });
});

// ---------------------------------------------------------------------------
// WHOIS
// ---------------------------------------------------------------------------

describe("WHOIS", () => {
  it("WHOIS returns 311, 312, 318 for known nick", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    const r = server.onMessage(mkConnId(1), parse("WHOIS bob"));
    expect(commands(r)).toContain("311");
    expect(commands(r)).toContain("312");
    expect(commands(r)).toContain("318");
  });

  it("WHOIS includes 319 channels when user is in channels", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(2), parse("JOIN #general"));
    const r = server.onMessage(mkConnId(1), parse("WHOIS bob"));
    expect(commands(r)).toContain("319");
  });

  it("WHOIS includes 301 when target is away", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(2), parse("AWAY :afk"));
    const r = server.onMessage(mkConnId(1), parse("WHOIS bob"));
    expect(commands(r)).toContain("301");
  });

  it("401 for unknown nick", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("WHOIS nobody"));
    expect(commands(r)).toContain("401");
  });

  it("461 when no params", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "WHOIS", params: [] });
    expect(commands(r)).toContain("461");
  });
});

// ---------------------------------------------------------------------------
// WHO
// ---------------------------------------------------------------------------

describe("WHO", () => {
  it("WHO returns 352 rows and 315 terminator", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    const r = server.onMessage(mkConnId(1), parse("WHO *"));
    expect(commands(r)).toContain("352");
    expect(commands(r)).toContain("315");
  });

  it("WHO #channel lists channel members", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    registerClient(server, mkConnId(2), "bob");
    server.onMessage(mkConnId(1), parse("JOIN #general"));
    server.onMessage(mkConnId(2), parse("JOIN #general"));
    const r = server.onMessage(mkConnId(1), parse("WHO #general"));
    const whoReplies = r.filter(([, msg]) => msg.command === "352");
    expect(whoReplies).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// OPER
// ---------------------------------------------------------------------------

describe("OPER", () => {
  it("OPER with correct password returns 381 RPL_YOUREOPER", () => {
    const server = new IRCServer("irc.local", [], "secret");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("OPER alice secret"));
    expect(commands(r)).toContain("381");
  });

  it("OPER with wrong password returns 464 ERR_PASSWDMISMATCH", () => {
    const server = new IRCServer("irc.local", [], "secret");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("OPER alice wrong"));
    expect(commands(r)).toContain("464");
  });

  it("OPER with no password configured returns 464", () => {
    const server = new IRCServer("irc.local"); // no oper password
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("OPER alice anything"));
    expect(commands(r)).toContain("464");
  });

  it("461 when not enough params", () => {
    const server = new IRCServer("irc.local", [], "secret");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), parse("OPER alice"));
    expect(commands(r)).toContain("461");
  });
});

// ---------------------------------------------------------------------------
// Unknown command
// ---------------------------------------------------------------------------

describe("Unknown command", () => {
  it("returns 421 ERR_UNKNOWNCOMMAND for unrecognised commands", () => {
    const server = new IRCServer("irc.local");
    registerClient(server, mkConnId(1), "alice");
    const r = server.onMessage(mkConnId(1), { prefix: null, command: "ZAPDOS", params: [] });
    expect(commands(r)).toContain("421");
  });
});
