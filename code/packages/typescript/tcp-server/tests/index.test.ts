import * as net from "net";
import { afterEach, describe, expect, it } from "vitest";

import { type Connection, TcpServer, VERSION } from "../src/index.js";

const servers: TcpServer[] = [];

function makeConnection(): Connection {
  return {
    id: 1,
    peerAddress: "127.0.0.1",
    peerPort: 45_001,
    localAddress: "127.0.0.1",
    localPort: 63_079,
    readBuffer: Buffer.alloc(0),
    selectedDb: 0,
  };
}

async function start(server: TcpServer): Promise<number> {
  servers.push(server);
  await server.start();
  return server.tryAddress().port;
}

async function sendRecv(port: number, data: Buffer | string): Promise<Buffer> {
  return await new Promise<Buffer>((resolve, reject) => {
    const socket = net.createConnection({ host: "127.0.0.1", port });
    const chunks: Buffer[] = [];
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error("timed out waiting for response"));
    }, 1_000);

    socket.on("connect", () => {
      socket.write(data);
    });
    socket.on("data", (chunk) => {
      chunks.push(Buffer.from(chunk));
      clearTimeout(timer);
      socket.end();
    });
    socket.on("close", () => {
      if (chunks.length > 0) resolve(Buffer.concat(chunks));
    });
    socket.on("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
  });
}

afterEach(async () => {
  await Promise.all(servers.splice(0).map((server) => server.stop()));
});

describe("TcpServer", () => {
  it("exports a version string", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("echoes bytes through the default handler without sockets", async () => {
    const server = new TcpServer({ port: 0 });
    const connection = makeConnection();

    await expect(server.handle(connection, Buffer.from("hello"))).resolves.toEqual(Buffer.from("hello"));
  });

  it("allows connection-aware handlers to transform data and state", async () => {
    const server = TcpServer.withHandler((connection, data) => {
      connection.readBuffer = Buffer.concat([connection.readBuffer, data]);
      if (connection.readBuffer.length < 6) return undefined;
      const response = connection.readBuffer;
      connection.readBuffer = Buffer.alloc(0);
      connection.selectedDb = 2;
      return response;
    });
    const connection = makeConnection();

    expect(await server.handle(connection, "buf")).toEqual(Buffer.alloc(0));
    expect(await server.handle(connection, "fer")).toEqual(Buffer.from("buffer"));
    expect(connection.selectedDb).toBe(2);
    expect(connection.readBuffer).toHaveLength(0);
  });

  it("starts, reports its address, and stops", async () => {
    const server = new TcpServer({ port: 0 });
    expect(server.isRunning()).toBe(false);
    expect(server.address()).toBeNull();
    expect(() => server.tryAddress()).toThrow("server has not been started");

    await start(server);
    await server.start();

    expect(server.isRunning()).toBe(true);
    expect(server.tryAddress().address).toBe("127.0.0.1");
    expect(server.toString()).toContain("running");

    await server.stop();
    expect(server.isRunning()).toBe(false);
  });

  it("handles a loopback echo request", async () => {
    const server = new TcpServer({ port: 0 });
    const port = await start(server);

    await expect(sendRecv(port, "hello world")).resolves.toEqual(Buffer.from("hello world"));
  });

  it("handles multiple sequential clients", async () => {
    const server = TcpServer.withHandler((_connection, data) => Buffer.from(data.toString().toUpperCase()), {
      port: 0,
    });
    const port = await start(server);

    await expect(sendRecv(port, "one")).resolves.toEqual(Buffer.from("ONE"));
    await expect(sendRecv(port, "two")).resolves.toEqual(Buffer.from("TWO"));
  });
});
