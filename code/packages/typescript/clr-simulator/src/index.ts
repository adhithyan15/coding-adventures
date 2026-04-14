export {
  CLROpcode,
  CEQ_BYTE,
  CGT_BYTE,
  CLT_BYTE,
  CLRSimulator,
  assembleClr,
  encodeLdcI4,
  encodeStloc,
  encodeLdloc,
} from "./simulator.js";

export type { CLRTrace } from "./simulator.js";
export type { CLRState } from "./state.js";
