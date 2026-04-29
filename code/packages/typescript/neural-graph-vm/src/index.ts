export {
  NeuralGraphCompileError,
  compileNeuralGraphToBytecode,
  runNeuralBytecodeForward,
  type NeuralBytecodeFunction,
  type NeuralBytecodeInstruction,
  type NeuralBytecodeModule,
  type NeuralBytecodeOpcode,
} from "./neural-graph-vm.js";
export {
  addActivation,
  addInput,
  addOutput,
  addWeightedSum,
  createNeuralGraph,
  type ActivationKind,
  type NeuralGraph,
  type WeightedInput,
} from "./primitives.js";
