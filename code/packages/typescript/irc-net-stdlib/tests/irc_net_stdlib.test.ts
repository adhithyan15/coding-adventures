/**
 * Integration tests for irc-net-stdlib — EventLoop with real TCP sockets.
 *
 * These tests use the Node.js `net` module to create real TCP connections
 * and verify the EventLoop's lifecycle callbacks work correctly.
 *
 * ## Port selection
 *
 * We use port 0 in all tests so the OS picks a free ephemeral port.
 * The actual port is read from `loop.listenPort` after the server starts.
 */

import { describe, it, expect, afterEach } from "vitest";
import * as net from "node:net";
import { EventLoop, ConnId, Handler } from "../src/index.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Active loops to clean up after each test. */
const activeLoops: EventLoop[] = [];

afterEach(() => {
  for (const loop of activeLoops) {
    loop.stop();
  }
  activeLoops.length = 0;
});

/**
 * Start a server with the given handler and return the loop + port.
 * The loop is registered for cleanup in afterEach.
 */
async function startServer(handler: Handler): Promise<{ loop: EventLoop; port: number }> {
  const loop = new EventLoop();
  activeLoops.push(loop);

  // Start the server on an ephemeral port; don't await yet.
  loop.run("127.0.0.1", 0, handler);

  // Wait for the server to start listening (up to 1 second).
  await waitFor(() => loop.listenPort !== null);
  const port = loop.listenPort!;

  return { loop, port };
}

/**
 * Connect a TCP client socket to `port` on localhost.
 * Returns a socket in the 'connected' state.
 */
function connectClient(port: number): Promise<net.Socket> {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ port, host: "127.0.0.1" });
    socket.once("connect", () => resolve(socket));
    socket.once("error", reject);
  });
}

/**
 * Send data to the socket and wait for a response.
 */
function sendAndReceive(socket: net.Socket, data: string): Promise<string> {
  return new Promise((resolve) => {
    socket.once("data", (chunk: Buffer) => {
      resolve(chunk.toString("utf-8"));
    });
    socket.write(data);
  });
}

/**
 * Poll until `condition()` returns true or timeout expires.
 */
function waitFor(condition: () => boolean, timeoutMs: number = 1000): Promise<void> {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    const check = () => {
      if (condition()) {
        resolve();
      } else if (Date.now() - start > timeoutMs) {
        reject(new Error("waitFor timeout"));
      } else {
        setTimeout(check, 10);
      }
    };
    check();
  });
}

/**
 * Collect data received on a socket for a brief period.
 */
function collectData(socket: net.Socket, durationMs: number = 100): Promise<Buffer> {
  return new Promise((resolve) => {
    const chunks: Buffer[] = [];
    socket.on("data", (chunk: Buffer) => chunks.push(chunk));
    setTimeout(() => resolve(Buffer.concat(chunks)), durationMs);
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("EventLoop", () => {
  it("starts a TCP server and accepts connections", async () => {
    let connectCalled = false;
    const handler: Handler = {
      onConnect(_connId, _host) { connectCalled = true; },
      onData() {},
      onDisconnect() {},
    };

    const { port } = await startServer(handler);
    const client = await connectClient(port);

    await waitFor(() => connectCalled);
    expect(connectCalled).toBe(true);
    client.destroy();
  });

  it("assigns unique ConnIds to each connection", async () => {
    const connIds: ConnId[] = [];
    const handler: Handler = {
      onConnect(connId) { connIds.push(connId); },
      onData() {},
      onDisconnect() {},
    };

    const { port } = await startServer(handler);
    const c1 = await connectClient(port);
    const c2 = await connectClient(port);

    await waitFor(() => connIds.length >= 2);
    expect(connIds[0]).not.toBe(connIds[1]);
    c1.destroy();
    c2.destroy();
  });

  it("calls onData with received bytes", async () => {
    let receivedData: Buffer | null = null;
    const handler: Handler = {
      onConnect() {},
      onData(_connId, data) { receivedData = data; },
      onDisconnect() {},
    };

    const { port } = await startServer(handler);
    const client = await connectClient(port);
    client.write("NICK alice\r\n");

    await waitFor(() => receivedData !== null);
    expect(receivedData!.toString("utf-8")).toContain("NICK alice");
    client.destroy();
  });

  it("calls onDisconnect when client closes", async () => {
    let disconnected = false;
    const handler: Handler = {
      onConnect() {},
      onData() {},
      onDisconnect() { disconnected = true; },
    };

    const { port } = await startServer(handler);
    const client = await connectClient(port);
    client.destroy();

    await waitFor(() => disconnected);
    expect(disconnected).toBe(true);
  });

  it("sendTo delivers data to the correct client", async () => {
    let serverConnId: ConnId | null = null;
    let loop: EventLoop | null = null;

    const handler: Handler = {
      onConnect(connId) { serverConnId = connId; },
      onData() {},
      onDisconnect() {},
    };

    const result = await startServer(handler);
    loop = result.loop;
    const { port } = result;

    const client = await connectClient(port);
    await waitFor(() => serverConnId !== null);

    const dataPromise = collectData(client, 200);
    loop.sendTo(serverConnId!, Buffer.from(":irc.local 001 alice :Welcome\r\n"));

    const received = await dataPromise;
    expect(received.toString("utf-8")).toContain("001");
    client.destroy();
  });

  it("sendTo to closed connId is a silent no-op", async () => {
    const loop = new EventLoop();
    activeLoops.push(loop);

    loop.run("127.0.0.1", 0, {
      onConnect() {},
      onData() {},
      onDisconnect() {},
    });

    // Should not throw.
    expect(() => {
      loop.sendTo(999 as ConnId, Buffer.from("hello\r\n"));
    }).not.toThrow();
    loop.stop();
  });

  it("stop() resolves the run() promise", async () => {
    const loop = new EventLoop();
    let resolved = false;

    const runPromise = loop.run("127.0.0.1", 0, {
      onConnect() {},
      onData() {},
      onDisconnect() {},
    }).then(() => { resolved = true; });

    await waitFor(() => loop.listenPort !== null);
    loop.stop();

    await runPromise;
    expect(resolved).toBe(true);
  });

  it("listenPort returns null before run() and after stop()", async () => {
    const loop = new EventLoop();
    expect(loop.listenPort).toBeNull();

    loop.run("127.0.0.1", 0, {
      onConnect() {},
      onData() {},
      onDisconnect() {},
    });

    await waitFor(() => loop.listenPort !== null);
    expect(loop.listenPort).toBeGreaterThan(0);

    loop.stop();
    // After stop(), listenPort should be null.
    expect(loop.listenPort).toBeNull();
  });

  it("multiple clients can connect simultaneously", async () => {
    const connectCount = { value: 0 };
    const handler: Handler = {
      onConnect() { connectCount.value++; },
      onData() {},
      onDisconnect() {},
    };

    const { port } = await startServer(handler);
    const clients = await Promise.all([
      connectClient(port),
      connectClient(port),
      connectClient(port),
    ]);

    await waitFor(() => connectCount.value >= 3);
    expect(connectCount.value).toBe(3);

    for (const c of clients) c.destroy();
  });
});
