import * as net from "net";

export const VERSION = "0.1.0";

export interface Connection {
  id: number;
  peerAddress: string;
  peerPort: number;
  localAddress: string;
  localPort: number;
  readBuffer: Buffer;
  selectedDb: number;
}

export type HandlerResponse = Buffer | Uint8Array | string | null | undefined;
export type Handler = (
  connection: Connection,
  data: Buffer,
) => HandlerResponse | Promise<HandlerResponse>;

export interface TcpServerOptions {
  host?: string;
  port?: number;
  backlog?: number;
  bufferSize?: number;
  handler?: Handler;
}

export class TcpServer {
  readonly host: string;
  readonly port: number;
  readonly backlog: number;
  readonly bufferSize: number;

  private readonly handler: Handler;
  private server: net.Server | null = null;
  private readonly sockets = new Map<number, net.Socket>();
  private nextConnectionId = 1;
  private running = false;

  constructor(options: TcpServerOptions = {}) {
    this.host = options.host ?? "127.0.0.1";
    this.port = options.port ?? 6380;
    this.backlog = Math.max(1, options.backlog ?? 128);
    this.bufferSize = Math.max(1, options.bufferSize ?? 4096);
    this.handler = options.handler ?? ((_connection, data) => data);
  }

  static withHandler(handler: Handler, options: Omit<TcpServerOptions, "handler"> = {}): TcpServer {
    return new TcpServer({ ...options, handler });
  }

  async start(): Promise<void> {
    if (this.running && this.server !== null) return;

    this.server = net.createServer((socket) => {
      this.accept(socket);
    });

    await new Promise<void>((resolve, reject) => {
      const onError = (error: Error) => {
        this.server?.off("listening", onListening);
        reject(error);
      };
      const onListening = () => {
        this.server?.off("error", onError);
        this.running = true;
        resolve();
      };

      this.server?.once("error", onError);
      this.server?.once("listening", onListening);
      this.server?.listen({ host: this.host, port: this.port, backlog: this.backlog });
    });
  }

  async serve(): Promise<void> {
    await this.start();
    await new Promise<void>((resolve, reject) => {
      this.server?.once("close", resolve);
      this.server?.once("error", reject);
    });
  }

  serveForever(): Promise<void> {
    return this.serve();
  }

  async handle(connection: Connection, data: Buffer | Uint8Array | string): Promise<Buffer> {
    const input = Buffer.isBuffer(data) ? data : Buffer.from(data);
    const response = await this.handler(connection, input);
    return normalizeResponse(response);
  }

  async stop(): Promise<void> {
    for (const socket of this.sockets.values()) {
      socket.destroy();
    }
    this.sockets.clear();

    if (this.server === null) {
      this.running = false;
      return;
    }

    const server = this.server;
    this.server = null;
    await new Promise<void>((resolve, reject) => {
      server.close((error?: Error) => {
        this.running = false;
        if (error) reject(error);
        else resolve();
      });
    });
  }

  isRunning(): boolean {
    return this.running;
  }

  address(): net.AddressInfo | null {
    const address = this.server?.address();
    return typeof address === "object" && address !== null ? address : null;
  }

  tryAddress(): net.AddressInfo {
    const address = this.address();
    if (address === null) {
      throw new Error("server has not been started");
    }
    return address;
  }

  toString(): string {
    const status = this.isRunning() ? "running" : "stopped";
    return `TcpServer(host=${JSON.stringify(this.host)}, port=${this.port}, status=${status})`;
  }

  private accept(socket: net.Socket): void {
    const id = this.nextConnectionId++;
    const connection: Connection = {
      id,
      peerAddress: socket.remoteAddress ?? "",
      peerPort: socket.remotePort ?? 0,
      localAddress: socket.localAddress ?? "",
      localPort: socket.localPort ?? 0,
      readBuffer: Buffer.alloc(0),
      selectedDb: 0,
    };

    this.sockets.set(id, socket);
    socket.on("data", (chunk) => {
      void this.respond(socket, connection, Buffer.from(chunk));
    });
    socket.on("close", () => {
      this.sockets.delete(id);
    });
    socket.on("error", () => {
      this.sockets.delete(id);
      socket.destroy();
    });
  }

  private async respond(socket: net.Socket, connection: Connection, data: Buffer): Promise<void> {
    try {
      const response = await this.handle(connection, data);
      if (response.length > 0 && !socket.destroyed) {
        socket.write(response);
      }
    } catch {
      socket.destroy();
    }
  }
}

function normalizeResponse(response: HandlerResponse): Buffer {
  if (response === null || response === undefined) return Buffer.alloc(0);
  if (Buffer.isBuffer(response)) return response;
  if (typeof response === "string") return Buffer.from(response);
  return Buffer.from(response);
}
