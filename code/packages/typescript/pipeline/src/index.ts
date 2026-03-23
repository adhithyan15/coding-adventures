/**
 * Pipeline — Orchestrator for the computing stack.
 *
 * Chains lexer, parser, compiler, and VM into a single execution flow,
 * capturing traces at every stage for the HTML visualizer.
 *
 * Usage:
 *
 *     import { Pipeline } from "@coding-adventures/pipeline";
 *
 *     const result = new Pipeline().run("x = 1 + 2");
 *
 *     // Inspect each stage:
 *     console.log(result.lexerStage.tokenCount);        // Number of tokens
 *     console.log(result.parserStage.astDict);           // JSON-serializable AST
 *     console.log(result.compilerStage.instructionsText); // Human-readable bytecode
 *     console.log(result.vmStage.finalVariables);         // { x: 3 }
 */

export type {
  CompilerStage,
  LexerStage,
  ParserStage,
  PipelineResult,
  VMStage,
} from "./orchestrator.js";

export { Pipeline, astToDict, instructionToText } from "./orchestrator.js";

// Re-export VM types for consumers that need to work with bytecode directly.
export type {
  CodeObject,
  Instruction,
  VMTrace,
  OpCodeValue,
} from "./vm-types.js";
export { OpCode, OpCodeName } from "./vm-types.js";

// Re-export the compiler and VM for direct use.
export { BytecodeCompiler } from "./compiler.js";
export { VirtualMachine } from "./vm.js";
