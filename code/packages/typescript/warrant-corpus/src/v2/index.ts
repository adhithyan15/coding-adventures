/**
 * # The v2 runner library at `ir.core`, `ir.fixed` and `ir.tuple` (VER04r, slices S7a and S10f)
 *
 * The TypeScript lane's second reading of the Warrant corpus v2 profile.  It is
 * a *library*: values, the IR typer and evaluator, domains, chunks and fault
 * wrappers, plus the harness that runs the conformance kit by `requires`.  The
 * corpus session, the binding and the manifest are slice S7b.
 *
 * Reading order for newcomers:
 *
 *     levels -> types -> values -> domains -> fixed -> ir -> chunks -> faults
 *            -> corpus -> kit
 *
 * **On independence.**  This lane is meant to be a *second, independent
 * reading* of one specification (VER04r with its errata, VER04i, VER04j and the
 * committed fixtures).  It was written from those documents and from this
 * package's own v1 code only.  The repository cannot prove that independence;
 * the evidence for it is that the code disagrees with nothing the kit pins and
 * that its falsification tests (`tests/v2/mutants.test.ts`) catch deliberately
 * wrong rules.
 */

export { KitError } from "./errors.ts";
export type { KitCode, OverflowClass } from "./errors.ts";
export { DEFINED_LEVELS, IMPLEMENTED_LEVELS, levelOfTag } from "./levels.ts";
export { parseType, readTypeTable, showType, flows, unify, faultCommon } from "./types.ts";
export type { Ty, TypeTable } from "./types.ts";
export { decodeJson, decodeTfv, encodeJson, makeWriter, streamBytes } from "./values.ts";
export type { Value } from "./values.ts";
export { buildDomain } from "./domains.ts";
export type { Domain } from "./domains.ts";
export { binaryFixed, boundValue, unaryFixed } from "./fixed.ts";
export { Program, readOp, COST_CEILING, DEFAULT_MAX_COST, MAX_DEPTH, MAX_IR_VALUE } from "./ir.ts";
export type { Compiled, OpSig, Param, Role } from "./ir.ts";
export { enumerateChunk, enumerateChunks, framedDigest, loadSuite, readChunkFile, readCorpusPart } from "./chunks.ts";
export type { ChunkResult, ChunkRow, Suite } from "./chunks.ts";
export { compileFault } from "./faults.ts";
export type { CompiledFault } from "./faults.ts";
export { checkWork, chunkFileName, loadV2Corpus, runSuite } from "./corpus.ts";
export type { ControlRecord, SuiteRecord, V2Corpus, Work } from "./corpus.ts";
export { KNOWN_CATEGORIES, runCase, runKit } from "./kit.ts";
export type { CategoryReport, KitReport } from "./kit.ts";
