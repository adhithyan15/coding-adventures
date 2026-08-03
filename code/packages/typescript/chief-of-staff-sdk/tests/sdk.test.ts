import { beforeEach, describe, expect, it } from "vitest";
import {
  ChannelClient,
  ChiefSdkError,
  HostRpcError,
  JsonRpcLineTransport,
  channel_ack,
  channel_read,
  channel_write,
  clearHostTransport,
  configureHostTransport,
  type HostTransport,
  type JsonLineDuplex,
} from "../src/index.js";

class ScriptedLines implements JsonLineDuplex {
  readonly writes: string[] = [];
  private readonly reads: Array<string | null>;

  constructor(...reads: Array<string | null>) {
    this.reads = reads;
  }

  async readLine(): Promise<string | null> {
    return this.reads.shift() ?? null;
  }

  async writeLine(line: string): Promise<void> {
    this.writes.push(line);
  }
}

class ScriptedTransport implements HostTransport {
  readonly calls: Array<{
    method: string;
    params: Readonly<Record<string, unknown>>;
  }> = [];
  private readonly results: unknown[];

  constructor(...results: unknown[]) {
    this.results = results;
  }

  async request(
    method: string,
    params: Readonly<Record<string, unknown>>,
  ): Promise<unknown> {
    this.calls.push({ method, params });
    return this.results.shift();
  }
}

beforeEach(() => clearHostTransport());

describe("JsonRpcLineTransport", () => {
  it("writes strict requests and returns matching results", async () => {
    const lines = new ScriptedLines(
      '{"jsonrpc":"2.0","id":1,"result":{"ok":true}}',
    );
    const transport = new JsonRpcLineTransport(lines);

    await expect(transport.request("system.identity", {})).resolves.toEqual({
      ok: true,
    });
    expect(JSON.parse(lines.writes[0]!)).toEqual({
      jsonrpc: "2.0",
      id: 1,
      method: "system.identity",
      params: {},
    });
  });

  it("serializes concurrent requests and increments identifiers", async () => {
    const lines = new ScriptedLines(
      '{"jsonrpc":"2.0","id":1,"result":"first"}',
      '{"jsonrpc":"2.0","id":2,"result":"second"}',
    );
    const transport = new JsonRpcLineTransport(lines);

    await expect(
      Promise.all([
        transport.request("first", {}),
        transport.request("second", {}),
      ]),
    ).resolves.toEqual(["first", "second"]);
    expect(lines.writes.map((line) => JSON.parse(line).id)).toEqual([1, 2]);
  });

  it("preserves structured host errors", async () => {
    const lines = new ScriptedLines(
      '{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"CapabilityDenied","data":{"reason":"not wired"}}}',
    );
    const transport = new JsonRpcLineTransport(lines);
    const error = await transport
      .request("channel.read", {})
      .catch((cause) => cause);

    expect(error).toBeInstanceOf(HostRpcError);
    expect(error).toMatchObject({
      code: -32001,
      data: { reason: "not wired" },
    });
  });

  it.each([
    [null, "closed"],
    ["not json", "valid JSON"],
    ['{"id":1,"result":null}', "JSON-RPC 2.0"],
    ['{"jsonrpc":"2.0","id":2,"result":null}', "identifier"],
    ['{"jsonrpc":"2.0","id":1}', "exactly one"],
    ['{"jsonrpc":"2.0","id":1,"result":null,"error":{}}', "exactly one"],
    [
      '{"jsonrpc":"2.0","id":1,"error":{"code":"bad","message":"x"}}',
      "malformed",
    ],
  ])("rejects malformed response %j", async (response, message) => {
    const transport = new JsonRpcLineTransport(new ScriptedLines(response));
    await expect(transport.request("system.now", {})).rejects.toThrow(message);
  });

  it("rejects invalid local request fields", async () => {
    const transport = new JsonRpcLineTransport(new ScriptedLines());
    expect(() => transport.request("", {})).toThrow("non-empty");
    await expect(
      transport.request("ok", [] as unknown as Record<string, unknown>),
    ).rejects.toThrow("params");
  });
});

describe("ChannelClient", () => {
  it("reads null without inventing a message", async () => {
    const transport = new ScriptedTransport(null);
    await expect(
      new ChannelClient(transport).read("inbox"),
    ).resolves.toBeNull();
    expect(transport.calls).toEqual([
      { method: "channel.read", params: { channel_id: "inbox" } },
    ]);
  });

  it("decodes lossless metadata and binary payloads", async () => {
    const transport = new ScriptedTransport({
      message_id: "0198-message",
      sequence: "9007199254740993",
      timestamp_ns: "18446744073709551615",
      content_type: "application/octet-stream",
      payload_b64: "AP+AQQ==",
    });

    await expect(new ChannelClient(transport).read("inbox")).resolves.toEqual({
      id: "0198-message",
      sequence: 9007199254740993n,
      timestampNs: 18446744073709551615n,
      contentType: "application/octet-stream",
      payload: new Uint8Array([0, 255, 128, 65]),
    });
  });

  it("encodes payloads and returns the publish identity", async () => {
    const transport = new ScriptedTransport({ message_id: "0198-published" });
    const result = await new ChannelClient(transport).write(
      "outbox",
      new Uint8Array([0, 1, 2, 253, 254, 255]),
      "application/test",
    );

    expect(result).toBe("0198-published");
    expect(transport.calls).toEqual([
      {
        method: "channel.write",
        params: {
          channel_id: "outbox",
          payload_b64: "AAEC/f7/",
          content_type: "application/test",
        },
      },
    ]);
  });

  it("uses the binary default content type and acknowledges with a null result", async () => {
    const transport = new ScriptedTransport({ message_id: "published" }, null);
    const client = new ChannelClient(transport);
    await client.write("outbox", new Uint8Array());
    await expect(client.ack("inbox", "message")).resolves.toBeUndefined();
    expect(transport.calls[0]!.params.content_type).toBe(
      "application/octet-stream",
    );
    expect(transport.calls[1]).toEqual({
      method: "channel.ack",
      params: { channel_id: "inbox", message_id: "message" },
    });
  });

  it.each([
    [{}, "message_id"],
    [
      {
        message_id: "m",
        sequence: 1,
        timestamp_ns: "0",
        content_type: "x",
        payload_b64: "",
      },
      "sequence",
    ],
    [
      {
        message_id: "m",
        sequence: "01",
        timestamp_ns: "0",
        content_type: "x",
        payload_b64: "",
      },
      "sequence",
    ],
    [
      {
        message_id: "m",
        sequence: "0",
        timestamp_ns: "-1",
        content_type: "x",
        payload_b64: "",
      },
      "timestamp",
    ],
    [
      {
        message_id: "m",
        sequence: "0",
        timestamp_ns: "0",
        content_type: "x",
        payload_b64: "%%%=",
      },
      "base64",
    ],
  ])("rejects malformed channel.read result %j", async (result, message) => {
    await expect(
      new ChannelClient(new ScriptedTransport(result)).read("inbox"),
    ).rejects.toThrow(message);
  });

  it("rejects malformed write and ack results", async () => {
    await expect(
      new ChannelClient(new ScriptedTransport(null)).write(
        "out",
        new Uint8Array(),
      ),
    ).rejects.toThrow("object");
    await expect(
      new ChannelClient(new ScriptedTransport({})).ack("in", "msg"),
    ).rejects.toThrow("must be null");
  });

  it("rejects invalid arguments before calling the host", async () => {
    const client = new ChannelClient(new ScriptedTransport());
    await expect(client.read("")).rejects.toThrow("channelId");
    await expect(
      client.write("out", "bad" as unknown as Uint8Array),
    ).rejects.toThrow("Uint8Array");
    await expect(client.write("out", new Uint8Array(), "")).rejects.toThrow(
      "contentType",
    );
    await expect(client.ack("in", "")).rejects.toThrow("messageId");
  });
});

describe("module-level channel API", () => {
  it("fails closed until configured", () => {
    expect(() => channel_read("inbox")).toThrow("not configured");
    expect(() =>
      configureHostTransport(null as unknown as HostTransport),
    ).toThrow("transport");
  });

  it("delegates read, write, and ack to the configured transport", async () => {
    const transport = new ScriptedTransport(null, { message_id: "new" }, null);
    configureHostTransport(transport);

    await expect(channel_read("in")).resolves.toBeNull();
    await expect(
      channel_write("out", new Uint8Array([65]), "text/plain"),
    ).resolves.toBe("new");
    await expect(channel_ack("in", "old")).resolves.toBeUndefined();
    expect(transport.calls.map((call) => call.method)).toEqual([
      "channel.read",
      "channel.write",
      "channel.ack",
    ]);
  });
});
