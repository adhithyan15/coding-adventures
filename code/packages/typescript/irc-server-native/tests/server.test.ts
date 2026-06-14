import { describe, it, expect, afterEach } from "vitest";
import net from "node:net";
import { IrcServer } from "../src/index.js";

// End-to-end tests: start the real Rust IRC engine on an ephemeral port and
// drive live IRC clients over real TCP sockets via Node's `net`.

function connect(port: number): Promise<net.Socket> {
  return new Promise((resolve, reject) => {
    const sock = net.connect(port, "127.0.0.1");
    sock.once("connect", () => resolve(sock));
    sock.once("error", reject);
  });
}

function recvUntil(sock: net.Socket, needle: string, timeoutMs = 5000): Promise<string> {
  return new Promise((resolve) => {
    let buf = "";
    const onData = (chunk: Buffer) => {
      buf += chunk.toString("utf8");
      if (buf.includes(needle)) finish();
    };
    const timer = setTimeout(finish, timeoutMs);
    function finish() {
      clearTimeout(timer);
      sock.off("data", onData);
      resolve(buf);
    }
    sock.on("data", onData);
  });
}

async function register(sock: net.Socket, nick: string): Promise<void> {
  sock.write(`NICK ${nick}\r\nUSER ${nick} 0 * :${nick}\r\n`);
  const welcome = await recvUntil(sock, "001");
  expect(welcome, `expected 001 welcome for ${nick}`).toContain("001");
}

describe("IrcServer", () => {
  let server: IrcServer | undefined;
  const sockets: net.Socket[] = [];

  afterEach(() => {
    for (const s of sockets) s.destroy();
    sockets.length = 0;
    if (server) {
      server.stop();
      try {
        server.dispose();
      } catch {
        // already disposed or not disposable — ignore in teardown
      }
      server = undefined;
    }
  });

  it("reports the ephemeral bound address", () => {
    server = new IrcServer({ port: 0 });
    expect(server.localHost).toBe("127.0.0.1");
    expect(server.localPort).toBeGreaterThan(0);
    expect(server.localAddr).toBe(`127.0.0.1:${server.localPort}`);
  });

  it("flips running true after serve", async () => {
    server = new IrcServer({ port: 0 });
    expect(server.running).toBe(false);
    server.serve();
    await new Promise((r) => setTimeout(r, 100));
    expect(server.running).toBe(true);
  });

  it("registers a client and answers PING", async () => {
    server = new IrcServer({ port: 0, serverName: "irc.test" });
    server.serve();
    const alice = await connect(server.localPort);
    sockets.push(alice);
    await register(alice, "alice");
    alice.write("PING :liveness\r\n");
    const pong = await recvUntil(alice, "PONG");
    expect(pong).toContain("PONG");
  });

  it("broadcasts PRIVMSG between two clients", async () => {
    server = new IrcServer({ port: 0, serverName: "irc.test" });
    server.serve();
    const alice = await connect(server.localPort);
    const bob = await connect(server.localPort);
    sockets.push(alice, bob);
    await register(alice, "alice");
    await register(bob, "bob");
    alice.write("JOIN #test\r\n");
    bob.write("JOIN #test\r\n");
    await recvUntil(alice, "JOIN");
    await recvUntil(bob, "JOIN");

    // Alice speaks; Bob (a different connection) must receive it — exercises the
    // Rust engine's in-process mailbox fan-out.
    alice.write("PRIVMSG #test :hello bob\r\n");
    const received = await recvUntil(bob, "hello bob");
    expect(received).toContain("PRIVMSG");
    expect(received).toContain("hello bob");
  });

  it("refuses dispose while running", async () => {
    server = new IrcServer({ port: 0 });
    server.serve();
    await new Promise((r) => setTimeout(r, 100));
    expect(() => server!.dispose()).toThrow(/running/);
  });
});
