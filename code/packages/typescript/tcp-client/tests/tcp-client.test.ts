/**
 * # tcp-client tests
 *
 * These tests verify the TCP client against real local servers running on
 * OS-assigned ports (port 0). Each test helper spins up a `net.Server`,
 * lets the OS pick a free port, and returns the port number for the test
 * to connect to.
 *
 * ## Test groups
 *
 * 1. **Echo server tests** -- write data, read it back
 * 2. **Timeout tests** -- verify connect and read timeouts
 * 3. **Error tests** -- connection refused, DNS failure, unexpected EOF
 * 4. **Half-close tests** -- shutdownWrite() while still reading
 * 5. **Edge cases** -- empty writes, address methods, defaults
 */

import { describe, it, expect, afterEach } from "vitest";
import * as net from "net";
import {
  VERSION,
  connect,
  TcpError,
  DnsResolutionFailed,
  ConnectionRefusedError,
  TimeoutError,
  ConnectionResetError,
  BrokenPipeError,
  UnexpectedEofError,
  TcpConnection,
  ConnectOptions,
} from "../src/index.js";

// ============================================================================
// Test helpers: local TCP servers
// ============================================================================
//
// Each helper returns { port, server } so the test can connect to the
// server and close it when done. Using port 0 lets the OS pick an
// available port, avoiding conflicts when tests run in parallel.

/** Servers to clean up after each test. */
const servers: net.Server[] = [];
const connections: TcpConnection[] = [];

afterEach(() => {
  // Close all servers and connections to avoid dangling handles
  for (const conn of connections) {
    try {
      conn.close();
    } catch {
      // ignore
    }
  }
  connections.length = 0;

  for (const server of servers) {
    try {
      server.close();
    } catch {
      // ignore
    }
  }
  servers.length = 0;
});

/**
 * Start an echo server that reads data and writes it back.
 *
 * This is the simplest possible TCP server:
 * 1. Accept a connection
 * 2. On 'data', write the same bytes back
 * 3. On client EOF, close
 */
function startEchoServer(): Promise<number> {
  return new Promise((resolve) => {
    const server = net.createServer((socket) => {
      socket.on("data", (chunk) => {
        socket.write(chunk);
      });
      socket.on("error", () => {
        // Ignore errors (client may close abruptly)
      });
    });
    servers.push(server);
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address() as net.AddressInfo;
      resolve(addr.port);
    });
  });
}

/**
 * Start a server that accepts connections but never sends data.
 * Used to test read timeouts.
 */
function startSilentServer(): Promise<number> {
  return new Promise((resolve) => {
    const server = net.createServer((_socket) => {
      // Accept but never write -- just hold the connection open
      _socket.on("error", () => {});
    });
    servers.push(server);
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address() as net.AddressInfo;
      resolve(addr.port);
    });
  });
}

/**
 * Start a server that sends exactly the given data, then closes.
 * Used to test partial reads and unexpected EOF.
 */
function startPartialServer(data: Buffer): Promise<number> {
  return new Promise((resolve) => {
    const server = net.createServer((socket) => {
      socket.write(data, () => {
        // Small delay so client can read before we close
        setTimeout(() => {
          socket.end();
        }, 50);
      });
      socket.on("error", () => {});
    });
    servers.push(server);
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address() as net.AddressInfo;
      resolve(addr.port);
    });
  });
}

/**
 * Start a server that reads a request, then sends a canned response.
 * Used for request/response pattern tests (like HTTP).
 */
function startRequestResponseServer(response: Buffer): Promise<number> {
  return new Promise((resolve) => {
    const server = net.createServer((socket) => {
      socket.once("data", () => {
        socket.write(response, () => {
          setTimeout(() => {
            socket.end();
          }, 50);
        });
      });
      socket.on("error", () => {});
    });
    servers.push(server);
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address() as net.AddressInfo;
      resolve(addr.port);
    });
  });
}

/** Default test options with short timeouts to keep tests fast. */
const testOptions: ConnectOptions = {
  connectTimeout: 5000,
  readTimeout: 5000,
  writeTimeout: 5000,
  bufferSize: 4096,
};

// ============================================================================
// Group 1: Basic connectivity
// ============================================================================

describe("tcp-client", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  // ── Group 1: Echo server tests ──────────────────────────────────────

  it("connect and disconnect", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);
    // If we got here without throwing, the connection succeeded
    expect(conn).toBeInstanceOf(TcpConnection);
    conn.close();
  });

  it("write and read back", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    await conn.writeAll("Hello, TCP!");
    await conn.flush();

    const result = await conn.readExact(11);
    expect(result.toString("utf-8")).toBe("Hello, TCP!");
    conn.close();
  });

  it("readLine from echo", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    await conn.writeAll("Hello\r\nWorld\r\n");
    await conn.flush();

    const line1 = await conn.readLine();
    expect(line1).toBe("Hello\r\n");

    const line2 = await conn.readLine();
    expect(line2).toBe("World\r\n");
    conn.close();
  });

  it("readExact from echo", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Send 100 bytes with known pattern
    const data = Buffer.alloc(100);
    for (let i = 0; i < 100; i++) data[i] = i % 256;

    await conn.writeAll(data);
    await conn.flush();

    const result = await conn.readExact(100);
    expect(Buffer.compare(result, data)).toBe(0);
    conn.close();
  });

  it("readUntil from echo", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    await conn.writeAll("key:value\0next");
    await conn.flush();

    const result = await conn.readUntil(0x00); // null byte
    expect(result.toString("utf-8")).toBe("key:value\0");
    conn.close();
  });

  it("large data transfer", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Send 64 KiB
    const data = Buffer.alloc(65536);
    for (let i = 0; i < 65536; i++) data[i] = i % 256;

    await conn.writeAll(data);
    await conn.flush();

    const result = await conn.readExact(65536);
    expect(result.length).toBe(65536);
    expect(Buffer.compare(result, data)).toBe(0);
    conn.close();
  });

  it("multiple exchanges", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Exchange 1
    await conn.writeAll("ping\n");
    await conn.flush();
    const line1 = await conn.readLine();
    expect(line1).toBe("ping\n");

    // Exchange 2
    await conn.writeAll("pong\n");
    await conn.flush();
    const line2 = await conn.readLine();
    expect(line2).toBe("pong\n");
    conn.close();
  });

  // ── Group 2: Timeout tests ──────────────────────────────────────────

  it("read timeout", async () => {
    const port = await startSilentServer();
    const conn = await connect("127.0.0.1", port, {
      ...testOptions,
      readTimeout: 500,
    });
    connections.push(conn);

    try {
      await conn.readLine();
      expect.fail("should have timed out");
    } catch (err) {
      expect(err).toBeInstanceOf(TimeoutError);
      expect((err as TimeoutError).phase).toBe("read");
    }
    conn.close();
  });

  // ── Group 3: Error tests ────────────────────────────────────────────

  it("connection refused", async () => {
    // Bind to a port, then immediately close the server -- nothing listens
    const server = net.createServer();
    const port = await new Promise<number>((resolve) => {
      server.listen(0, "127.0.0.1", () => {
        const addr = server.address() as net.AddressInfo;
        resolve(addr.port);
      });
    });
    server.close();

    // Small delay to let the OS release the port
    await new Promise((r) => setTimeout(r, 100));

    try {
      const conn = await connect("127.0.0.1", port, testOptions);
      connections.push(conn);
      expect.fail("should have been refused");
    } catch (err) {
      expect(err).toBeInstanceOf(ConnectionRefusedError);
    }
  });

  it("DNS failure", async () => {
    try {
      const conn = await connect(
        "this.host.does.not.exist.example",
        80,
        testOptions,
      );
      connections.push(conn);
      expect.fail("should have failed DNS");
    } catch (err) {
      // Some ISPs hijack NXDOMAIN, so accept multiple error types
      expect(err).toBeInstanceOf(TcpError);
    }
  });

  it("unexpected EOF on readExact", async () => {
    // Server sends 50 bytes then closes, client tries to read 100
    const data = Buffer.alloc(50);
    for (let i = 0; i < 50; i++) data[i] = i;
    const port = await startPartialServer(data);

    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Wait for server to send and close
    await new Promise((r) => setTimeout(r, 200));

    try {
      await conn.readExact(100);
      expect.fail("should have thrown UnexpectedEofError");
    } catch (err) {
      expect(err).toBeInstanceOf(UnexpectedEofError);
      const eof = err as UnexpectedEofError;
      expect(eof.expected).toBe(100);
      expect(eof.received).toBe(50);
    }
  });

  it("broken pipe on write after server close", async () => {
    // Server accepts then immediately closes
    const port = await startPartialServer(Buffer.alloc(0));

    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Wait for server to close its end
    await new Promise((r) => setTimeout(r, 200));

    // Try to write -- should eventually get an error.
    // The first write may succeed (goes to OS buffer), so we retry.
    let gotError = false;
    for (let i = 0; i < 10; i++) {
      try {
        const bigData = Buffer.alloc(65536);
        await conn.writeAll(bigData);
        await conn.flush();
        await new Promise((r) => setTimeout(r, 50));
      } catch {
        gotError = true;
        break;
      }
    }
    expect(gotError).toBe(true);
  });

  // ── Group 4: Half-close tests ───────────────────────────────────────

  it("client half-close (shutdownWrite)", async () => {
    // Server reads until EOF, then sends "DONE\n"
    const port = await new Promise<number>((resolve) => {
      const server = net.createServer((socket) => {
        const chunks: Buffer[] = [];
        socket.on("data", (chunk) => {
          chunks.push(chunk);
        });
        socket.on("end", () => {
          // Client closed its write half. Send response.
          socket.write("DONE\n", () => {
            socket.end();
          });
        });
        socket.on("error", () => {});
      });
      servers.push(server);
      server.listen(0, "127.0.0.1", () => {
        const addr = server.address() as net.AddressInfo;
        resolve(addr.port);
      });
    });

    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    await conn.writeAll("request data");
    await conn.shutdownWrite();

    // Now read the server's response
    const response = await conn.readLine();
    expect(response).toBe("DONE\n");
    conn.close();
  });

  // ── Group 5: Edge cases ─────────────────────────────────────────────

  it("empty read at EOF", async () => {
    const data = Buffer.from("hello\n");
    const port = await startPartialServer(data);

    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Wait for server to send and close
    await new Promise((r) => setTimeout(r, 200));

    const line = await conn.readLine();
    expect(line).toBe("hello\n");

    // Next read should return empty string (EOF)
    const eof = await conn.readLine();
    expect(eof).toBe("");
    conn.close();
  });

  it("zero byte write succeeds", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Writing zero bytes should succeed without error
    await conn.writeAll(Buffer.alloc(0));
    conn.close();
  });

  it("peer address", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    const peer = conn.peerAddr();
    expect(peer.host).toBe("127.0.0.1");
    expect(peer.port).toBe(port);

    const local = conn.localAddr();
    expect(local.host).toBe("127.0.0.1");
    expect(local.port).toBeGreaterThan(0);
    conn.close();
  });

  it("local and peer address differ in port", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    const peer = conn.peerAddr();
    const local = conn.localAddr();
    // The local (ephemeral) port should differ from the server port
    expect(local.port).not.toBe(peer.port);
    conn.close();
  });

  it("connect options defaults", () => {
    // Verify the ConnectOptions interface accepts empty object
    const opts: ConnectOptions = {};
    expect(opts.connectTimeout).toBeUndefined();
    expect(opts.readTimeout).toBeUndefined();
    expect(opts.writeTimeout).toBeUndefined();
    expect(opts.bufferSize).toBeUndefined();
  });

  it("error display messages", () => {
    const dns = new DnsResolutionFailed("example.com", "no such host");
    expect(dns.message).toBe(
      "DNS resolution failed for 'example.com': no such host",
    );
    expect(dns.host).toBe("example.com");

    const refused = new ConnectionRefusedError("127.0.0.1:8080");
    expect(refused.message).toBe("connection refused by 127.0.0.1:8080");
    expect(refused.addr).toBe("127.0.0.1:8080");

    const timeout = new TimeoutError("read", 5000);
    expect(timeout.message).toBe("read timed out after 5000ms");
    expect(timeout.phase).toBe("read");
    expect(timeout.duration).toBe(5000);

    const reset = new ConnectionResetError();
    expect(reset.message).toBe("connection reset by peer");

    const broken = new BrokenPipeError();
    expect(broken.message).toBe("broken pipe (remote closed)");

    const eof = new UnexpectedEofError(100, 50);
    expect(eof.message).toBe("unexpected EOF: expected 100 bytes, got 50");
    expect(eof.expected).toBe(100);
    expect(eof.received).toBe(50);
  });

  it("error hierarchy", () => {
    // All errors should be instances of TcpError and Error
    const errors: TcpError[] = [
      new DnsResolutionFailed("host", "msg"),
      new ConnectionRefusedError("addr"),
      new TimeoutError("phase", 0),
      new ConnectionResetError(),
      new BrokenPipeError(),
      new UnexpectedEofError(0, 0),
    ];
    for (const err of errors) {
      expect(err).toBeInstanceOf(TcpError);
      expect(err).toBeInstanceOf(Error);
    }
  });

  it("request-response pattern (HTTP-like)", async () => {
    const response = Buffer.from(
      "HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello",
    );
    const port = await startRequestResponseServer(response);

    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Send request
    await conn.writeAll("GET / HTTP/1.0\r\n\r\n");
    await conn.flush();

    // Read response line by line
    const status = await conn.readLine();
    expect(status).toContain("HTTP/1.0 200");

    const header = await conn.readLine();
    expect(header).toContain("Content-Length:");

    const blank = await conn.readLine();
    expect(blank).toBe("\r\n");

    const body = await conn.readExact(5);
    expect(body.toString("utf-8")).toBe("hello");
    conn.close();
  });

  it("write string and Buffer interchangeably", async () => {
    const port = await startEchoServer();
    const conn = await connect("127.0.0.1", port, testOptions);
    connections.push(conn);

    // Write as string
    await conn.writeAll("abc");
    await conn.flush();
    const r1 = await conn.readExact(3);
    expect(r1.toString()).toBe("abc");

    // Write as Buffer
    await conn.writeAll(Buffer.from("def"));
    await conn.flush();
    const r2 = await conn.readExact(3);
    expect(r2.toString()).toBe("def");
    conn.close();
  });

  it("connect with localhost hostname", async () => {
    const port = await startEchoServer();
    // "localhost" should resolve to 127.0.0.1 via the OS resolver
    const conn = await connect("localhost", port, testOptions);
    connections.push(conn);
    expect(conn).toBeInstanceOf(TcpConnection);
    conn.close();
  });
});
