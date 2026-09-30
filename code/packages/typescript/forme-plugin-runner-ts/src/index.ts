/** TypeScript plugin-side implementation of the bounded Forme FM02 wire protocol. */
export { runPlugin } from "./run-plugin.js";
export type { RunPluginOptions } from "./run-plugin.js";
export {
  FrameDecoder,
  RunnerProtocolError,
  RunnerRpcRemoteError,
  decodeWireValue,
  encodeFrame,
  encodeWireValue,
} from "./wire.js";
export type { FrameLimits, JsonRpcId, JsonRpcMessage } from "./wire.js";
