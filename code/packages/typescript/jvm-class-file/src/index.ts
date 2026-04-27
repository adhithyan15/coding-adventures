export const VERSION = "0.1.0";

export {
  ACC_PUBLIC,
  ACC_STATIC,
  ACC_SUPER,
  ClassFileFormatError,
  buildMinimalClassFile,
  parseClassFile,
} from "./class_file.js";
export type {
  BuildMinimalClassFileParams,
  JVMAttributeInfo,
  JVMClassFile,
  JVMClassVersion,
  JVMCodeAttribute,
  JVMConstantPoolEntry,
  JVMFieldReference,
  JVMMethodInfo,
  JVMMethodReference,
} from "./class_file.js";
