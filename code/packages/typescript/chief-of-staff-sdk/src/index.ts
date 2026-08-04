/** Capability-safe TypeScript SDK for Chief of Staff agents. */

const MAX_IDENTIFIER_BYTES = 4 * 1024;
const MAX_CONTENT_TYPE_BYTES = 1024;
const MAX_PAYLOAD_BYTES = 64 * 1024 * 1024;
const MAX_PROTOCOL_LINE_BYTES = 90 * 1024 * 1024;
const BASE64_ALPHABET =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/** A protocol or local SDK validation failure. */
export class ChiefSdkError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ChiefSdkError";
  }
}

/** A structured error returned by the Chief host. */
export class HostRpcError extends Error {
  /** JSON-RPC or D18 host error code. */
  readonly code: number;
  /** Optional structured host diagnostic. */
  readonly data: unknown;

  constructor(code: number, message: string, data?: unknown) {
    super(message);
    this.name = "HostRpcError";
    this.code = code;
    this.data = data;
  }
}

/** One ordered pair of line-oriented input and output streams. */
export interface JsonLineDuplex {
  /** Read one UTF-8 line without its trailing newline, or `null` on EOF. */
  readLine(): Promise<string | null>;
  /** Write one UTF-8 line and its trailing newline. */
  writeLine(line: string): Promise<void>;
}

/** Minimum host operation required by the typed SDK clients. */
export interface HostTransport {
  /** Send one request and return its validated JSON result. */
  request(
    method: string,
    params: Readonly<Record<string, unknown>>,
  ): Promise<unknown>;
}

/**
 * Strict JSON-RPC 2.0 client over the D18 newline-delimited stdio transport.
 *
 * Calls are deliberately serialized. This keeps one line stream ordered and
 * allows an unexpected response identifier to be treated as corruption rather
 * than silently discarded.
 */
export class JsonRpcLineTransport implements HostTransport {
  private nextId = 0;
  private tail: Promise<void> = Promise.resolve();

  constructor(private readonly lines: JsonLineDuplex) {}

  request(
    method: string,
    params: Readonly<Record<string, unknown>>,
  ): Promise<unknown> {
    validateNonEmptyString(method, "method", MAX_IDENTIFIER_BYTES);
    if (!isRecord(params)) {
      return Promise.reject(new ChiefSdkError("params must be a JSON object"));
    }

    const operation = this.tail.then(() => this.exchange(method, params));
    this.tail = operation.then(
      () => undefined,
      () => undefined,
    );
    return operation;
  }

  private async exchange(
    method: string,
    params: Readonly<Record<string, unknown>>,
  ): Promise<unknown> {
    if (this.nextId >= Number.MAX_SAFE_INTEGER) {
      throw new ChiefSdkError("JSON-RPC request identifier space exhausted");
    }
    const id = ++this.nextId;
    const requestLine = JSON.stringify({ jsonrpc: "2.0", id, method, params });
    validateLineSize(requestLine, "outgoing");
    await this.lines.writeLine(requestLine);

    const responseLine = await this.lines.readLine();
    if (responseLine === null) {
      throw new ChiefSdkError(
        "host closed the protocol stream before responding",
      );
    }
    validateLineSize(responseLine, "incoming");

    let parsed: unknown;
    try {
      parsed = JSON.parse(responseLine);
    } catch {
      throw new ChiefSdkError("host response is not valid JSON");
    }
    if (!isRecord(parsed) || parsed["jsonrpc"] !== "2.0") {
      throw new ChiefSdkError("host response is not JSON-RPC 2.0");
    }
    if (parsed["id"] !== id) {
      throw new ChiefSdkError(
        "host response identifier does not match the request",
      );
    }

    const hasResult = Object.hasOwn(parsed, "result");
    const hasError = Object.hasOwn(parsed, "error");
    if (hasResult === hasError) {
      throw new ChiefSdkError(
        "host response must contain exactly one of result or error",
      );
    }
    if (hasResult) {
      return parsed["result"];
    }

    const error = parsed["error"];
    if (
      !isRecord(error) ||
      !Number.isInteger(error["code"]) ||
      typeof error["message"] !== "string" ||
      error["message"].length === 0
    ) {
      throw new ChiefSdkError("host returned a malformed JSON-RPC error");
    }
    throw new HostRpcError(
      error["code"] as number,
      error["message"],
      error["data"],
    );
  }
}

/** A verified and decrypted channel message. */
export interface Message {
  /** UUID-v7 message identity used for acknowledgement. */
  readonly id: string;
  /** Durable channel sequence without JavaScript precision loss. */
  readonly sequence: bigint;
  /** Authenticated originator timestamp in monotonic nanoseconds. */
  readonly timestampNs: bigint;
  /** Authenticated MIME content type. */
  readonly contentType: string;
  /** Verified plaintext payload. */
  readonly payload: Uint8Array;
}

/** UTF-8 message presented to a one-file Level 2 agent handler. */
export interface AgentMessage extends Message {
  /** Verified payload decoded as strict UTF-8 text. */
  readonly plaintext: string;
}

/** The complete author-facing contract for a Level 2 one-file agent. */
export type AgentHandler = (message: AgentMessage) => Promise<string>;

/** Static channel wiring supplied by the trusted agent wrapper. */
export interface SimpleAgentRuntimeConfig {
  /** Channel from which the agent receives its next input. */
  readonly inputChannelId: string;
  /** Different one-way channel to which the agent publishes responses. */
  readonly outputChannelId: string;
}

/** Result of polling at most one Level 2 input message. */
export type SimpleAgentRunOutcome =
  | { readonly status: "idle" }
  | {
      readonly status: "processed";
      /** Input identity acknowledged after successful publication. */
      readonly inputMessageId: string;
      /** Output identity returned by the host. */
      readonly outputMessageId: string;
      /** Exact handler text encoded and published by the runtime. */
      readonly response: string;
    };

/** MIME type used for Level 2 handler responses. */
export const SIMPLE_AGENT_RESPONSE_CONTENT_TYPE = "text/plain; charset=utf-8";

/**
 * Injected receive/handler/publish/ack runtime for one-file Level 2 agents.
 *
 * A handler or publication failure leaves the input unacknowledged so the
 * host can redeliver it after recovery. The runtime performs no ambient I/O.
 */
export class SimpleAgentRuntime {
  private readonly inputChannelId: string;
  private readonly outputChannelId: string;

  constructor(
    private readonly channels: ChannelClient,
    config: SimpleAgentRuntimeConfig,
    private readonly handler: AgentHandler,
  ) {
    validateIdentifier(config.inputChannelId, "inputChannelId");
    validateIdentifier(config.outputChannelId, "outputChannelId");
    if (config.inputChannelId === config.outputChannelId) {
      throw new ChiefSdkError(
        "Level 2 input and output channels must be different",
      );
    }
    validateHandler(handler);
    this.inputChannelId = config.inputChannelId;
    this.outputChannelId = config.outputChannelId;
  }

  /** Poll and process at most one input message. */
  async runOnce(): Promise<SimpleAgentRunOutcome> {
    const input = await this.channels.read(this.inputChannelId);
    if (input === null) {
      return { status: "idle" };
    }

    let plaintext: string;
    try {
      plaintext = new TextDecoder("utf-8", { fatal: true }).decode(
        input.payload,
      );
    } catch {
      throw new ChiefSdkError("Level 2 input payload is not UTF-8 text");
    }
    const message: AgentMessage = { ...input, plaintext };
    const response = await this.handler(message);
    if (typeof response !== "string" || response.trim().length === 0) {
      throw new ChiefSdkError(
        "Level 2 agent handler must return non-empty text",
      );
    }

    const outputMessageId = await this.channels.write(
      this.outputChannelId,
      new TextEncoder().encode(response),
      SIMPLE_AGENT_RESPONSE_CONTENT_TYPE,
    );
    await this.channels.ack(this.inputChannelId, input.id);
    return {
      status: "processed",
      inputMessageId: input.id,
      outputMessageId,
      response,
    };
  }
}

let definedAgentHandler: AgentHandler | undefined;

/** Register the single handler exported by a Level 2 agent module. */
export function defineAgent(handler: AgentHandler): void {
  validateHandler(handler);
  if (definedAgentHandler !== undefined) {
    throw new ChiefSdkError("a Level 2 agent handler is already defined");
  }
  definedAgentHandler = handler;
}

/** Clear the registered Level 2 handler during wrapper shutdown or tests. */
export function clearDefinedAgent(): void {
  definedAgentHandler = undefined;
}

/** Bind the currently defined handler to trusted channel wiring. */
export function createDefinedAgentRuntime(
  channels: ChannelClient,
  config: SimpleAgentRuntimeConfig,
): SimpleAgentRuntime {
  if (definedAgentHandler === undefined) {
    throw new ChiefSdkError("no Level 2 agent handler has been defined");
  }
  return new SimpleAgentRuntime(channels, config, definedAgentHandler);
}

/** Typed Level 3 channel client over one host transport. */
export class ChannelClient {
  constructor(private readonly transport: HostTransport) {}

  /** Read the next unacknowledged message, if one is available. */
  async read(channelId: string): Promise<Message | null> {
    validateIdentifier(channelId, "channelId");
    const result = await this.transport.request("channel.read", {
      channel_id: channelId,
    });
    if (result === null) {
      return null;
    }
    return decodeMessage(result);
  }

  /** Publish one plaintext payload and return its UUID-v7 message identity. */
  async write(
    channelId: string,
    payload: Uint8Array,
    contentType = "application/octet-stream",
  ): Promise<string> {
    validateIdentifier(channelId, "channelId");
    if (!(payload instanceof Uint8Array)) {
      throw new ChiefSdkError("payload must be a Uint8Array");
    }
    if (payload.byteLength > MAX_PAYLOAD_BYTES) {
      throw new ChiefSdkError("payload exceeds the 64 MiB channel bound");
    }
    validateNonEmptyString(contentType, "contentType", MAX_CONTENT_TYPE_BYTES);

    const result = await this.transport.request("channel.write", {
      channel_id: channelId,
      payload_b64: encodeBase64(payload),
      content_type: contentType,
    });
    if (!isRecord(result)) {
      throw new ChiefSdkError("channel.write result must be an object");
    }
    const messageId = result["message_id"];
    validateIdentifier(messageId, "channel.write message_id");
    return messageId;
  }

  /** Advance the receiver cursor only after successful message processing. */
  async ack(channelId: string, messageId: string): Promise<void> {
    validateIdentifier(channelId, "channelId");
    validateIdentifier(messageId, "messageId");
    const result = await this.transport.request("channel.ack", {
      channel_id: channelId,
      message_id: messageId,
    });
    if (result !== null) {
      throw new ChiefSdkError("channel.ack result must be null");
    }
  }
}

let defaultChannelClient: ChannelClient | undefined;

/** Configure the host transport used by the module-level SDK functions. */
export function configureHostTransport(transport: HostTransport): void {
  if (
    transport === null ||
    typeof transport !== "object" ||
    typeof transport.request !== "function"
  ) {
    throw new ChiefSdkError("transport must implement request(method, params)");
  }
  defaultChannelClient = new ChannelClient(transport);
}

/** Remove the configured transport, primarily for wrapper shutdown and tests. */
export function clearHostTransport(): void {
  defaultChannelClient = undefined;
}

/** Read through the configured Level 3 channel client. */
export function channel_read(channelId: string): Promise<Message | null> {
  return configuredClient().read(channelId);
}

/** Publish through the configured Level 3 channel client. */
export function channel_write(
  channelId: string,
  payload: Uint8Array,
  contentType?: string,
): Promise<string> {
  return configuredClient().write(channelId, payload, contentType);
}

/** Acknowledge through the configured Level 3 channel client. */
export function channel_ack(
  channelId: string,
  messageId: string,
): Promise<void> {
  return configuredClient().ack(channelId, messageId);
}

function configuredClient(): ChannelClient {
  if (defaultChannelClient === undefined) {
    throw new ChiefSdkError("host transport is not configured");
  }
  return defaultChannelClient;
}

function decodeMessage(value: unknown): Message {
  if (!isRecord(value)) {
    throw new ChiefSdkError("channel.read result must be an object or null");
  }
  const id = value["message_id"];
  const contentType = value["content_type"];
  const payload = value["payload_b64"];
  validateIdentifier(id, "channel.read message_id");
  validateNonEmptyString(
    contentType,
    "channel.read content_type",
    MAX_CONTENT_TYPE_BYTES,
  );
  if (typeof payload !== "string") {
    throw new ChiefSdkError("channel.read payload_b64 must be a string");
  }
  return {
    id,
    sequence: decodeDecimal(value["sequence"], "channel.read sequence"),
    timestampNs: decodeDecimal(
      value["timestamp_ns"],
      "channel.read timestamp_ns",
    ),
    contentType,
    payload: decodeBase64(payload),
  };
}

function decodeDecimal(value: unknown, field: string): bigint {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new ChiefSdkError(
      `${field} must be a canonical unsigned decimal string`,
    );
  }
  return BigInt(value);
}

function validateIdentifier(
  value: unknown,
  field: string,
): asserts value is string {
  validateNonEmptyString(value, field, MAX_IDENTIFIER_BYTES);
}

function validateHandler(value: unknown): asserts value is AgentHandler {
  if (typeof value !== "function") {
    throw new ChiefSdkError("Level 2 agent handler must be a function");
  }
}

function validateNonEmptyString(
  value: unknown,
  field: string,
  maximumBytes: number,
): asserts value is string {
  if (typeof value !== "string" || value.length === 0) {
    throw new ChiefSdkError(`${field} must be a non-empty string`);
  }
  if (new TextEncoder().encode(value).byteLength > maximumBytes) {
    throw new ChiefSdkError(`${field} exceeds its UTF-8 byte bound`);
  }
}

function validateLineSize(line: string, direction: string): void {
  if (new TextEncoder().encode(line).byteLength > MAX_PROTOCOL_LINE_BYTES) {
    throw new ChiefSdkError(`${direction} protocol line exceeds 90 MiB`);
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function encodeBase64(bytes: Uint8Array): string {
  let encoded = "";
  for (let offset = 0; offset < bytes.length; offset += 3) {
    const first = bytes[offset]!;
    const hasSecond = offset + 1 < bytes.length;
    const hasThird = offset + 2 < bytes.length;
    const second = hasSecond ? bytes[offset + 1]! : 0;
    const third = hasThird ? bytes[offset + 2]! : 0;
    const word = (first << 16) | (second << 8) | third;
    encoded += BASE64_ALPHABET[(word >>> 18) & 63];
    encoded += BASE64_ALPHABET[(word >>> 12) & 63];
    encoded += hasSecond ? BASE64_ALPHABET[(word >>> 6) & 63] : "=";
    encoded += hasThird ? BASE64_ALPHABET[word & 63] : "=";
  }
  return encoded;
}

function decodeBase64(value: string): Uint8Array {
  if (
    value.length % 4 !== 0 ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(
      value,
    )
  ) {
    throw new ChiefSdkError("channel.read payload_b64 is not canonical base64");
  }
  const padding = value.endsWith("==") ? 2 : value.endsWith("=") ? 1 : 0;
  const output = new Uint8Array((value.length / 4) * 3 - padding);
  let outputOffset = 0;
  for (let offset = 0; offset < value.length; offset += 4) {
    const a = BASE64_ALPHABET.indexOf(value[offset]!);
    const b = BASE64_ALPHABET.indexOf(value[offset + 1]!);
    const c =
      value[offset + 2] === "="
        ? 0
        : BASE64_ALPHABET.indexOf(value[offset + 2]!);
    const d =
      value[offset + 3] === "="
        ? 0
        : BASE64_ALPHABET.indexOf(value[offset + 3]!);
    const word = (a << 18) | (b << 12) | (c << 6) | d;
    if (outputOffset < output.length)
      output[outputOffset++] = (word >>> 16) & 255;
    if (outputOffset < output.length)
      output[outputOffset++] = (word >>> 8) & 255;
    if (outputOffset < output.length) output[outputOffset++] = word & 255;
  }
  if (encodeBase64(output) !== value) {
    throw new ChiefSdkError("channel.read payload_b64 is not canonical base64");
  }
  return output;
}
