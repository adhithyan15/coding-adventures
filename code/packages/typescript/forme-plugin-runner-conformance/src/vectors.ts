export const CONFORMANCE_STAGE = Object.freeze({
  id: "conformance",
  pluginName: "@forme/conformance",
  pluginVersion: "1.0.0",
  apiVersion: 1,
  protocolVersion: 1,
  configSchemaHash: "sha256:conformance-schema",
  capabilities: Object.freeze([
    "storage:read", "storage:write", "env:ALLOWED", "filesystem:user",
    "network:example.com", "system:time:wallclock", "system:shell",
  ]),
});

export const CONFORMANCE_VECTORS = Object.freeze([
  "lifecycle",
  "wire-values",
  "capabilities",
  "typed-error",
  "cancellation",
  "stream-stream",
  "stream-single",
  "single-stream",
  "identity-mismatch",
  "malformed-response-envelope",
  "malformed-peer",
  "resource-bounds",
] as const);

export type ConformanceVector = (typeof CONFORMANCE_VECTORS)[number];
