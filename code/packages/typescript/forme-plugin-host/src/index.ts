/**
 * @coding-adventures/forme-plugin-host
 *
 * FM02/FM-B014 plugin discovery, manifest-authored stage proxies, strict
 * Content-Length JSON-RPC, lifecycle, streaming, diagnostics, cancellation,
 * crash isolation, and host-mediated capabilities.
 *
 * Runtime/OS sandbox launchers are deliberately injected and remain FM-B015.
 * The host has no unsandboxed fallback and accepts only an explicit sandbox
 * attestation from the configured process factory.
 */

export { createPluginHost, FORME_PLUGIN_PROTOCOL_VERSION } from "./host.js";
export { discoverPlugins, resolveContainedFile } from "./discovery.js";
export { descriptorForKindReference } from "./kinds.js";
export {
  formatGrantsFile,
  formatTrustStore,
  parseGrantsFile,
  parseTrustStore,
  readGrantsFile,
  readTrustStore,
  writeGrantsFile,
  writeTrustStore,
} from "./persistent-authority.js";
export type {
  LoadedPluginGrants,
  PluginGrantDecision,
  PluginGrantsFile,
  PluginTrustStore,
  TrustedPluginKey,
} from "./persistent-authority.js";
export { mediateCapabilityRequest } from "./capability-mediator.js";
export { decodeWireValue, encodeFrame, encodeWireValue, FrameDecoder, RpcPeer } from "./wire.js";
export type {
  FrameLimits,
  JsonRpcId,
  JsonRpcMessage,
  PendingRpc,
  RpcPeerOptions,
} from "./wire.js";
export {
  PLUGIN_HOST_ERROR_CODES,
  PluginHostError,
  RpcFault,
  RpcRemoteError,
} from "./errors.js";
export type { PluginHostErrorCode } from "./errors.js";
export type {
  DiscoveredPlugin,
  LaunchedPluginProcess,
  PluginHost,
  PluginHostOptions,
  PluginLaunchRequest,
  PluginProcessExit,
  PluginProcessFactory,
  VerifiedPluginSnapshot,
  PluginStageLoader,
} from "./types.js";
