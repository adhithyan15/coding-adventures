/**
 * ircd — IRC server executable.
 *
 * This module is the wiring layer — the topmost layer of the IRC stack.  It
 * connects the pure IRC logic (`irc-server`) to the TCP transport layer
 * (`irc-net-stdlib`) via two adapter objects:
 *
 * ## `DriverHandler`
 *
 * Implements the `Handler` interface expected by `irc-net-stdlib`.
 * Translates raw byte chunks from the network into `Message` objects,
 * feeds them to `IRCServer`, and sends the resulting responses back
 * over the network.
 *
 * ## `main()` / `parseArgs()` / `Config`
 *
 * Entry-point glue: parse command-line arguments, construct the server and
 * event loop, install signal handlers for graceful shutdown, and call
 * `loop.run()`.
 *
 * ## Wiring diagram
 *
 * ```
 * TCP socket
 *    ↓ raw bytes
 * EventLoop (socket 'data' event)       ← irc-net-stdlib
 *    ↓ connId, raw bytes
 * DriverHandler.onData()                ← THIS MODULE
 *    ↓ feeds bytes into per-connection Framer
 * Framer.frames()                       ← irc-framing
 *    ↓ Buffer("NICK alice")
 * irc-proto parse()                     ← irc-proto
 *    ↓ Message(command='NICK', ...)
 * IRCServer.onMessage()                 ← irc-server
 *    ↓ [ConnId, Message][]
 * irc-proto serialize()                 ← irc-proto
 *    ↓ Buffer(":irc.local 001 alice :Welcome\r\n")
 * EventLoop.sendTo()                    ← irc-net-stdlib
 *    ↓ bytes on the wire
 * ```
 *
 * None of the four dependency packages know about each other — only this module
 * imports all four and wires them together.  This is the Dependency Inversion
 * Principle at work: higher-level modules (`irc-server`) know nothing about
 * lower-level infrastructure (sockets), because both talk through a common
 * message interface.
 */

import { Framer } from "@coding-adventures/irc-framing";
import { ConnId, EventLoop, Handler } from "@coding-adventures/irc-net-stdlib";
import { ParseError, parse, serialize } from "@coding-adventures/irc-proto";
import { IRCServer } from "@coding-adventures/irc-server";

// ---------------------------------------------------------------------------
// DriverHandler — bridges irc-net-stdlib and irc-server
// ---------------------------------------------------------------------------

/**
 * Adapts `IRCServer` to the `Handler` interface expected by `irc-net-stdlib`.
 *
 * The `irc-net-stdlib` event loop calls three lifecycle callbacks on a Handler:
 *
 * * `onConnect(connId, host)` — a new TCP connection arrived.
 * * `onData(connId, data)`    — raw bytes from an established connection.
 * * `onDisconnect(connId)`    — the TCP connection has closed.
 *
 * `DriverHandler` translates these raw-bytes events into structured `Message`
 * objects that `IRCServer` can process, and sends the resulting
 * `[ConnId, Message]` responses back over the wire via `loop.sendTo()`.
 *
 * ## Per-connection framing
 *
 * IRC uses CRLF-terminated text lines.  TCP, however, delivers an arbitrary
 * byte stream — a single `data` event may return half a message, one complete
 * message, or five messages concatenated together.  To reassemble byte chunks
 * into complete lines, each connection gets its own `Framer` instance (from
 * `irc-framing`).  The `Framer` is stored in a `Map` keyed by `ConnId`,
 * created in `onConnect` and removed in `onDisconnect`.
 *
 * ## Concurrency
 *
 * Node.js is single-threaded.  All callbacks (`onConnect`, `onData`,
 * `onDisconnect`) run sequentially on the event loop thread.  The `IRCServer`
 * state machine is therefore naturally safe without any locking.
 */
export class DriverHandler implements Handler {
  private readonly server: IRCServer;
  private readonly loop: EventLoop;
  // One Framer per live connection.
  private readonly framers: Map<ConnId, Framer> = new Map();

  constructor(server: IRCServer, loop: EventLoop) {
    this.server = server;
    this.loop = loop;
  }

  // ------------------------------------------------------------------
  // Handler protocol — called by the event loop
  // ------------------------------------------------------------------

  /**
   * Record a new connection and notify the IRC state machine.
   *
   * We create a `Framer` for this connection so subsequent `onData` calls can
   * assemble complete IRC lines.  We also tell `IRCServer` about the new
   * connection so it can create a `Client` record with the peer's host address.
   */
  onConnect(connId: ConnId, host: string): void {
    // Create a per-connection framer before registering with the server.
    this.framers.set(connId, new Framer());

    // Notify the server.  Returns [] (no initial responses).
    const responses = this.server.onConnect(connId, host);
    this.sendResponses(responses);
  }

  /**
   * Feed raw bytes into the per-connection framer and dispatch messages.
   *
   * This is the hot path — called for every TCP `data` event.  The sequence:
   *
   * 1. Feed raw bytes into the `Framer`.
   * 2. Extract all complete lines (`Framer.frames()`).
   * 3. Decode each line from UTF-8 (`errors` are replaced with U+FFFD).
   * 4. Parse each line with `irc-proto.parse()`; skip unparseable lines
   *    without closing the connection (IRC servers traditionally ignore garbage).
   * 5. Pass the parsed `Message` to `IRCServer.onMessage()`.
   * 6. Send any resulting `[ConnId, Message]` responses.
   */
  onData(connId: ConnId, data: Buffer): void {
    const framer = this.framers.get(connId);
    if (!framer) {
      // Defensive: data arrived for a connection we have no framer for.
      return;
    }

    // Absorb the raw bytes into the framer's internal buffer.
    framer.feed(data);

    // Drain all complete lines from the framer.
    for (const rawLine of framer.frames()) {
      // IRC is specified as ASCII but UTF-8 is universally accepted.
      // We decode with replacement characters for invalid bytes rather than
      // crashing — a single bad byte should not kill the connection.
      const line = rawLine.toString("utf-8");

      let responses;
      try {
        const msg = parse(line);
        responses = this.server.onMessage(connId, msg);
      } catch (e) {
        if (e instanceof ParseError) {
          // Malformed or empty line — skip silently.
          continue;
        }
        throw e;
      }

      this.sendResponses(responses);
    }
  }

  /**
   * Clean up state for a closed connection.
   *
   * We notify `IRCServer` (which broadcasts a QUIT to all channels the client
   * was in), dispatch those responses, and then discard the framer for this
   * connection to free memory.
   */
  onDisconnect(connId: ConnId): void {
    const responses = this.server.onDisconnect(connId);
    this.sendResponses(responses);
    this.framers.delete(connId);
  }

  // ------------------------------------------------------------------
  // Internal helpers
  // ------------------------------------------------------------------

  /**
   * Serialize and deliver a list of `[ConnId, Message]` responses.
   *
   * `IRCServer` returns responses as an array of `[ConnId, Message]` tuples.
   * We serialize each `Message` to bytes using `irc-proto` and forward it to
   * the event loop's `sendTo()` method.
   */
  private sendResponses(responses: Array<[ConnId, unknown]>): void {
    for (const [targetConnId, msg] of responses) {
      if (
        msg !== null &&
        typeof msg === "object" &&
        "command" in msg &&
        "params" in msg &&
        "prefix" in msg
      ) {
        const wire = serialize(msg as Parameters<typeof serialize>[0]);
        this.loop.sendTo(targetConnId, wire);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Config — command-line configuration
// ---------------------------------------------------------------------------

/**
 * All runtime configuration for `ircd`.
 *
 * Values are populated by `parseArgs()` from `process.argv`.  Default values
 * match the conventional IRC server setup:
 *
 * - `host`          — bind address; `"0.0.0.0"` means all interfaces.
 * - `port`          — IRC standard port 6667 (no TLS).
 * - `serverName`    — hostname shown in the 001 welcome message.
 * - `motd`          — Message of the Day lines.
 * - `operPassword`  — password for the OPER command; empty disables.
 */
export interface Config {
  host: string;
  port: number;
  serverName: string;
  motd: string[];
  operPassword: string;
}

/**
 * Parse `argv` (e.g. `process.argv.slice(2)`) into a `Config`.
 *
 * Uses manual flag parsing to avoid a runtime dependency on a CLI library.
 * Flags:
 * - `--host <addr>`        (default: `"0.0.0.0"`)
 * - `--port <n>`           (default: `6667`)
 * - `--server-name <name>` (default: `"irc.local"`)
 * - `--motd <line>`        (default: `["Welcome."]`)
 * - `--oper-password <pw>` (default: `""`)
 */
export function parseArgs(argv: string[]): Config {
  const config: Config = {
    host: "0.0.0.0",
    port: 6667,
    serverName: "irc.local",
    motd: ["Welcome."],
    operPassword: "",
  };

  let i = 0;
  const motdLines: string[] = [];

  while (i < argv.length) {
    const arg = argv[i];
    switch (arg) {
      case "--host":
        config.host = argv[++i] ?? "0.0.0.0";
        break;
      case "--port":
        config.port = parseInt(argv[++i] ?? "6667", 10);
        break;
      case "--server-name":
        config.serverName = argv[++i] ?? "irc.local";
        break;
      case "--motd":
        motdLines.push(argv[++i] ?? "");
        break;
      case "--oper-password":
        config.operPassword = argv[++i] ?? "";
        break;
      default:
        // Unknown flag — silently ignore (forward-compatible).
        break;
    }
    i++;
  }

  if (motdLines.length > 0) {
    config.motd = motdLines;
  }

  return config;
}

// ---------------------------------------------------------------------------
// main — entry point
// ---------------------------------------------------------------------------

/**
 * Parse arguments, wire up all components, and run the IRC server.
 *
 * This function:
 * 1. Parses `process.argv.slice(2)` (or the supplied `argv` array) into a
 *    `Config`.
 * 2. Creates an `IRCServer` with the configured name, MOTD, and oper password.
 * 3. Creates an `EventLoop`.
 * 4. Creates a `DriverHandler` that bridges the network layer and IRC logic.
 * 5. Installs `SIGINT`/`SIGTERM` handlers for graceful shutdown.
 * 6. Calls `loop.run()` — this resolves when `loop.stop()` is called.
 *
 * The program can be run two ways:
 * ```
 * # As a compiled script:
 * node dist/index.js --port 6667
 *
 * # Via ts-node/tsx in development:
 * npx tsx src/index.ts --port 6667
 * ```
 */
export async function main(argv?: string[]): Promise<void> {
  const config = parseArgs(argv ?? process.argv.slice(2));

  const loop = new EventLoop();
  const server = new IRCServer(config.serverName, config.motd, config.operPassword);
  const handler = new DriverHandler(server, loop);

  // Graceful shutdown: SIGINT (Ctrl-C) and SIGTERM both call loop.stop(),
  // which closes the listening socket and lets loop.run() resolve.
  const shutdown = () => {
    loop.stop();
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  process.stdout.write(`ircd listening on ${config.host}:${config.port}\n`);

  await loop.run(config.host, config.port, handler);
}

// ---------------------------------------------------------------------------
// ESM main-module detection
// ---------------------------------------------------------------------------

// In Node.js ESM, `import.meta.url` is the file URL of the current module.
// We compare it to `process.argv[1]` (converted to a file URL) to detect
// whether this module is being run directly (as opposed to imported by another
// module).
//
// This is the ESM equivalent of Python's `if __name__ == "__main__":`.
//
// We use a try/catch because `new URL()` can throw if process.argv[1] contains
// characters that aren't valid in a URL (e.g. Windows backslashes on some Node
// versions).
let _isMain = false;
try {
  const { fileURLToPath, pathToFileURL } = await import("node:url");
  const argvPath = process.argv[1] ? pathToFileURL(process.argv[1]).href : "";
  _isMain = import.meta.url === argvPath || fileURLToPath(import.meta.url) === process.argv[1];
} catch {
  // If we can't determine, don't auto-run.
}

if (_isMain) {
  await main();
}
