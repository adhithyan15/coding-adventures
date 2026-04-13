/**
 * @coding-adventures/compiler-source-map — Source map chain for the AOT compiler pipeline.
 *
 * The source map chain is the sidecar data structure that flows through every
 * stage of the compiler pipeline, connecting source text positions to machine
 * code byte offsets — and back again.
 *
 * Four segments:
 *   Segment 1: SourceToAst       — source text position  → AST node ID
 *   Segment 2: AstToIr           — AST node ID           → IR instruction IDs
 *   Segment 3: IrToIr            — IR instruction ID     → optimised IR IDs
 *   Segment 4: IrToMachineCode   — IR instruction ID     → MC byte offset + length
 */

export type {
  SourcePosition,
  SourceToAstEntry,
  AstToIrEntry,
  IrToIrEntry,
  IrToMachineCodeEntry,
} from "./source_map.js";

export {
  SourceToAst,
  AstToIr,
  IrToIr,
  IrToMachineCode,
  SourceMapChain,
  sourcePositionToString,
} from "./source_map.js";
