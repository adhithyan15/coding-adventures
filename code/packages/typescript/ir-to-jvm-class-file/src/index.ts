export const VERSION = "0.1.1";

export {
  JvmBackendError,
  lowerIrToJvmClassFile,
  writeClassFile,
} from "./backend.js";
export type {
  JVMClassArtifact,
  JvmBackendConfig,
} from "./backend.js";
