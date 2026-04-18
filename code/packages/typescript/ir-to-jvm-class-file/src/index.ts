export const VERSION = "0.1.0";

export {
  JvmBackendError,
  lowerIrToJvmClassFile,
  writeClassFile,
} from "./backend.js";
export type {
  JVMClassArtifact,
  JvmBackendConfig,
} from "./backend.js";
