import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  SIMPLE_AGENT_RESPONSE_CONTENT_TYPE,
  ChannelClient,
  ChiefSdkError,
  HostRpcError,
  JsonRpcLineTransport,
  SimpleAgentRuntime,
  VaultClient,
  channel_ack,
  channel_read,
  channel_write,
  clearDefinedAgent,
  clearHostTransport,
  configureHostTransport,
  createDefinedAgentRuntime,
  defineAgent,
  vault_release_lease,
  vault_request_direct,
  vault_request_lease,
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
    const result = this.results.shift();
    if (result instanceof Error) throw result;
    return result;
  }
}

beforeEach(() => {
  clearHostTransport();
  clearDefinedAgent();
});

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

describe("VaultClient", () => {
  it("requests an opaque lease and redacts its bearer reference from diagnostics", async () => {
    const transport = new ScriptedTransport({
      vault_ref: "vault-ref-secret-bearer",
      expires_at_ms: 1_800_000_000_000,
    });

    const receipt = await new VaultClient(transport).requestLease(
      "bank-creds",
      10_000,
    );

    expect(transport.calls).toEqual([
      {
        method: "vault.requestLease",
        params: { name: "bank-creds", ttl_ms: 10_000 },
      },
    ]);
    expect(receipt.vault_ref).toBe("vault-ref-secret-bearer");
    expect(receipt.expires_at_ms).toBe(1_800_000_000_000);
    expect(Object.keys(receipt)).toEqual(["expires_at_ms"]);
    expect({ ...receipt }).toEqual({ expires_at_ms: 1_800_000_000_000 });
    expect(JSON.stringify(receipt)).toBe(
      '{"vault_ref":"<redacted>","expires_at_ms":1800000000000}',
    );
    expect(JSON.stringify(receipt)).not.toContain("secret-bearer");
    expect(Object.isFrozen(receipt)).toBe(true);
  });

  it("requests direct delivery and releases only through the host", async () => {
    const transport = new ScriptedTransport(null, null);
    const client = new VaultClient(transport);

    await expect(
      client.requestDirect("bank-creds", "browser-agent"),
    ).resolves.toBeUndefined();
    await expect(
      client.releaseLease("vault-ref-secret-bearer"),
    ).resolves.toBeUndefined();
    expect(transport.calls).toEqual([
      {
        method: "vault.requestDirect",
        params: {
          name: "bank-creds",
          consumer_agent_id: "browser-agent",
        },
      },
      {
        method: "vault.releaseLease",
        params: { vault_ref: "vault-ref-secret-bearer" },
      },
    ]);
  });

  it.each([
    [null, "object"],
    [{}, "vault_ref"],
    [{ vault_ref: "", expires_at_ms: 1 }, "vault_ref"],
    [{ vault_ref: 7, expires_at_ms: 1 }, "vault_ref"],
    [{ vault_ref: "ref", expires_at_ms: -1 }, "expires_at_ms"],
    [{ vault_ref: "ref", expires_at_ms: 1.5 }, "expires_at_ms"],
    [
      { vault_ref: "ref", expires_at_ms: Number.MAX_SAFE_INTEGER + 1 },
      "expires_at_ms",
    ],
    [{ vault_ref: "ref", expires_at_ms: "1" }, "expires_at_ms"],
  ])("rejects malformed vault.requestLease result %j", async (result, message) => {
    await expect(
      new VaultClient(new ScriptedTransport(result)).requestLease("secret", 1),
    ).rejects.toThrow(message);
  });

  it.each([0, 7_776_000_001, 1.5, Number.NaN])(
    "rejects invalid lease TTL %j before calling the host",
    async (ttlMs) => {
      const transport = new ScriptedTransport();
      await expect(
        new VaultClient(transport).requestLease("secret", ttlMs),
      ).rejects.toThrow("ttlMs");
      expect(transport.calls).toEqual([]);
    },
  );

  it("rejects invalid arguments and malformed null acknowledgements", async () => {
    const invalid = new ScriptedTransport();
    const invalidClient = new VaultClient(invalid);
    await expect(invalidClient.requestLease("", 1)).rejects.toThrow(
      "secretName",
    );
    await expect(invalidClient.requestDirect("secret", "")).rejects.toThrow(
      "consumerAgentId",
    );
    await expect(invalidClient.releaseLease("")).rejects.toThrow("vaultRef");
    expect(invalid.calls).toEqual([]);

    await expect(
      new VaultClient(new ScriptedTransport({})).requestDirect(
        "secret",
        "consumer",
      ),
    ).rejects.toThrow("must be null");
    await expect(
      new VaultClient(new ScriptedTransport("ok")).releaseLease("ref"),
    ).rejects.toThrow("must be null");
  });
});

describe("module-level Vault API", () => {
  it("fails closed until configured", () => {
    expect(() => vault_request_lease("secret", 1)).toThrow("not configured");
    expect(() => vault_request_direct("secret", "consumer")).toThrow(
      "not configured",
    );
    expect(() => vault_release_lease("ref")).toThrow("not configured");
  });

  it("delegates lease, direct delivery, and release to the configured transport", async () => {
    const transport = new ScriptedTransport(
      { vault_ref: "opaque-ref", expires_at_ms: 2_000 },
      null,
      null,
    );
    configureHostTransport(transport);

    await expect(vault_request_lease("secret", 1_000)).resolves.toMatchObject({
      vault_ref: "opaque-ref",
      expires_at_ms: 2_000,
    });
    await expect(
      vault_request_direct("secret", "consumer"),
    ).resolves.toBeUndefined();
    await expect(vault_release_lease("opaque-ref")).resolves.toBeUndefined();
    expect(transport.calls.map((call) => call.method)).toEqual([
      "vault.requestLease",
      "vault.requestDirect",
      "vault.releaseLease",
    ]);
  });
});

describe("Level 2 defineAgent runtime", () => {
  const incoming = {
    message_id: "0198-input",
    sequence: "7",
    timestamp_ns: "9007199254740993",
    content_type: "application/json",
    payload_b64: "eyJjaXR5IjoiU2VhdHRsZSJ9",
  };

  it("runs a defined one-file handler before publishing and acknowledging", async () => {
    const handler = vi.fn(async (message) => {
      const body = JSON.parse(message.plaintext) as { city: string };
      expect(message.sequence).toBe(7n);
      expect(message.contentType).toBe("application/json");
      return `The weather in ${body.city} is sunny.`;
    });
    defineAgent(handler);
    const transport = new ScriptedTransport(
      incoming,
      { message_id: "0198-output" },
      null,
    );
    const runtime = createDefinedAgentRuntime(new ChannelClient(transport), {
      inputChannelId: "weather-requests",
      outputChannelId: "weather-reports",
    });

    await expect(runtime.runOnce()).resolves.toEqual({
      status: "processed",
      inputMessageId: "0198-input",
      outputMessageId: "0198-output",
      response: "The weather in Seattle is sunny.",
    });
    expect(handler).toHaveBeenCalledOnce();
    expect(transport.calls.map((call) => call.method)).toEqual([
      "channel.read",
      "channel.write",
      "channel.ack",
    ]);
    expect(transport.calls[1]!.params).toEqual({
      channel_id: "weather-reports",
      payload_b64: "VGhlIHdlYXRoZXIgaW4gU2VhdHRsZSBpcyBzdW5ueS4=",
      content_type: SIMPLE_AGENT_RESPONSE_CONTENT_TYPE,
    });
  });

  it("returns idle without invoking the handler or output channel", async () => {
    const handler = vi.fn(async () => "unused");
    const runtime = new SimpleAgentRuntime(
      new ChannelClient(new ScriptedTransport(null)),
      { inputChannelId: "in", outputChannelId: "out" },
      handler,
    );

    await expect(runtime.runOnce()).resolves.toEqual({ status: "idle" });
    expect(handler).not.toHaveBeenCalled();
  });

  it("leaves input unacknowledged when handler or publication fails", async () => {
    const handlerFailure = new ScriptedTransport(incoming);
    const throwingRuntime = new SimpleAgentRuntime(
      new ChannelClient(handlerFailure),
      { inputChannelId: "in", outputChannelId: "out" },
      async () => {
        throw new Error("handler failed");
      },
    );
    await expect(throwingRuntime.runOnce()).rejects.toThrow("handler failed");
    expect(handlerFailure.calls.map((call) => call.method)).toEqual([
      "channel.read",
    ]);

    const publishFailure = new ScriptedTransport(
      incoming,
      new HostRpcError(-32001, "CapabilityDenied"),
    );
    const publishingRuntime = new SimpleAgentRuntime(
      new ChannelClient(publishFailure),
      { inputChannelId: "in", outputChannelId: "out" },
      async () => "response",
    );
    await expect(publishingRuntime.runOnce()).rejects.toThrow(
      "CapabilityDenied",
    );
    expect(publishFailure.calls.map((call) => call.method)).toEqual([
      "channel.read",
      "channel.write",
    ]);
  });

  it("rejects non-UTF-8 input and empty or non-string handler output", async () => {
    const nonUtf8 = { ...incoming, payload_b64: "/w==" };
    const invalidInputRuntime = new SimpleAgentRuntime(
      new ChannelClient(new ScriptedTransport(nonUtf8)),
      { inputChannelId: "in", outputChannelId: "out" },
      async () => "unused",
    );
    await expect(invalidInputRuntime.runOnce()).rejects.toThrow("UTF-8");

    for (const response of ["   ", 42] as unknown[]) {
      const transport = new ScriptedTransport(incoming);
      const runtime = new SimpleAgentRuntime(
        new ChannelClient(transport),
        { inputChannelId: "in", outputChannelId: "out" },
        async () => response as string,
      );
      await expect(runtime.runOnce()).rejects.toThrow("non-empty text");
      expect(transport.calls.map((call) => call.method)).toEqual([
        "channel.read",
      ]);
    }
  });

  it("preserves an acknowledgement failure after publication", async () => {
    const transport = new ScriptedTransport(
      incoming,
      { message_id: "output" },
      new HostRpcError(-32603, "ack failed"),
    );
    const runtime = new SimpleAgentRuntime(
      new ChannelClient(transport),
      { inputChannelId: "in", outputChannelId: "out" },
      async () => "response",
    );

    await expect(runtime.runOnce()).rejects.toThrow("ack failed");
    expect(transport.calls.map((call) => call.method)).toEqual([
      "channel.read",
      "channel.write",
      "channel.ack",
    ]);
  });

  it("validates registration and static one-way channel wiring", () => {
    expect(() =>
      createDefinedAgentRuntime(new ChannelClient(new ScriptedTransport()), {
        inputChannelId: "in",
        outputChannelId: "out",
      }),
    ).toThrow("no Level 2");
    expect(() => defineAgent(null as unknown as () => Promise<string>)).toThrow(
      "function",
    );
    expect(
      () =>
        new SimpleAgentRuntime(
          new ChannelClient(new ScriptedTransport()),
          { inputChannelId: "same", outputChannelId: "same" },
          async () => "response",
        ),
    ).toThrow("different");
    expect(
      () =>
        new SimpleAgentRuntime(
          new ChannelClient(new ScriptedTransport()),
          { inputChannelId: "", outputChannelId: "out" },
          async () => "response",
        ),
    ).toThrow("inputChannelId");
  });

  it("allows exactly one handler definition per wrapper process", async () => {
    defineAgent(async () => "first");
    const first = createDefinedAgentRuntime(
      new ChannelClient(
        new ScriptedTransport(incoming, { message_id: "one" }, null),
      ),
      { inputChannelId: "in", outputChannelId: "out" },
    );

    await expect(first.runOnce()).resolves.toMatchObject({ response: "first" });
    expect(() => defineAgent(async () => "second")).toThrow("already defined");
  });
});
