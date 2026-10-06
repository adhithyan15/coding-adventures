export const PLUGIN_HOST_ERROR_CODES = Object.freeze([
  "PLUGIN_NOT_FOUND",
  "STAGE_NOT_FOUND",
  "MANIFEST_INVALID",
  "ENTRY_OUTSIDE_PLUGIN",
  "CONFIG_SCHEMA_INVALID",
  "REQUIRED_CAPABILITY_DENIED",
  "SANDBOX_UNAVAILABLE",
  "HANDSHAKE_TIMEOUT",
  "REQUEST_TIMEOUT",
  "MANIFEST_MISMATCH",
  "PROTOCOL_VIOLATION",
  "FRAME_TOO_LARGE",
  "HEADER_TOO_LARGE",
  "TRUNCATED_FRAME",
  "PLUGIN_CRASHED",
  "RESOURCE_LIMIT_EXCEEDED",
  "AUTHORITY_FILE_INVALID",
  "AUTHORITY_FILE_UNSAFE",
] as const);

export type PluginHostErrorCode = (typeof PLUGIN_HOST_ERROR_CODES)[number];

export class PluginHostError extends Error {
  readonly code: PluginHostErrorCode;
  readonly details: Readonly<Record<string, unknown>>;

  constructor(
    code: PluginHostErrorCode,
    message: string,
    details: Readonly<Record<string, unknown>> = {},
    options?: ErrorOptions,
  ) {
    super(`${code}: ${message}`, options);
    this.name = "PluginHostError";
    this.code = code;
    this.details = Object.freeze({ ...details });
  }
}

export class RpcFault extends Error {
  constructor(
    readonly rpcCode: number,
    message: string,
    readonly data?: unknown,
  ) {
    super(message);
    this.name = "RpcFault";
  }
}

export class RpcRemoteError extends Error {
  constructor(
    readonly rpcCode: number,
    message: string,
    readonly data?: unknown,
  ) {
    super(message);
    this.name = "RpcRemoteError";
  }
}
