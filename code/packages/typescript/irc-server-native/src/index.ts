// coding-adventures-irc-server-native — a high-performance IRC server for Node.js.
//
// All IRC and TCP logic runs in Rust (the `irc-net-reactor` engine on the
// home-grown kqueue/epoll reactor).  Node only launches and controls the
// server; there is no per-message callback into JavaScript, so the API is tiny.

import { createRequire } from "module";

interface NativeServer {
  serve(): void;
  stop(): void;
  running(): boolean;
  localHost(): string;
  localPort(): number;
  dispose(): void;
}

interface NativeModule {
  newServer(
    host: string,
    port: number,
    serverName: string,
    motd: string[],
    operPassword: string,
    maxConnections: number,
  ): NativeServer;
}

function loadNative(): NativeModule {
  // The compiled addon sits at the package root as `irc_native_node.node`.
  // `createRequire(import.meta.url)` lets an ESM module load a CommonJS `.node`
  // addon, and resolves correctly from both `src/` (tests) and `dist/` (built).
  const req = createRequire(import.meta.url);
  return req("../irc_native_node.node") as NativeModule;
}

export interface IrcServerOptions {
  /** Bind address. `"127.0.0.1"` is loopback; `"0.0.0.0"` is all interfaces. */
  host?: string;
  /** TCP port. `0` asks the OS for a free ephemeral port (read back via `localPort`). */
  port?: number;
  /** Hostname advertised in the `001` welcome and message prefixes. */
  serverName?: string;
  /** Message of the Day lines. Defaults to `["Welcome."]`. */
  motd?: string[];
  /** Password for the `OPER` command. Empty string (default) disables OPER. */
  operPassword?: string;
  /** Maximum simultaneous connections. */
  maxConnections?: number;
}

/**
 * A high-performance IRC server backed by the Rust `irc-net-reactor` engine.
 *
 * ```ts
 * const server = new IrcServer({ port: 6667 });
 * server.serve();           // returns immediately; the loop runs on a background thread
 * // ... later ...
 * server.stop();
 * ```
 */
export class IrcServer {
  private readonly native: NativeServer;

  constructor(options: IrcServerOptions = {}) {
    const host = options.host ?? "127.0.0.1";
    const port = options.port ?? 6667;
    const serverName = options.serverName ?? "irc.local";
    const motd = options.motd && options.motd.length > 0 ? options.motd : ["Welcome."];
    const operPassword = options.operPassword ?? "";
    const maxConnections = options.maxConnections ?? 1024;

    const native = loadNative();
    this.native = native.newServer(
      host,
      port,
      serverName,
      motd,
      operPassword,
      maxConnections,
    );
  }

  /** Start the event loop on a background thread. Returns immediately (does not
   *  block Node's event loop). Idempotent while already running. */
  serve(): void {
    this.native.serve();
  }

  /** Signal the loop to stop and join the background thread. */
  stop(): void {
    this.native.stop();
  }

  /** Release the engine (the server must be stopped first). */
  dispose(): void {
    this.native.dispose();
  }

  /** Whether the event loop is currently running. */
  get running(): boolean {
    return this.native.running();
  }

  /** The bound IP address. */
  get localHost(): string {
    return this.native.localHost();
  }

  /** The bound TCP port (the OS-assigned port when constructed with `port: 0`). */
  get localPort(): number {
    return this.native.localPort();
  }

  /** The bound `host:port` address. */
  get localAddr(): string {
    return `${this.localHost}:${this.localPort}`;
  }
}
