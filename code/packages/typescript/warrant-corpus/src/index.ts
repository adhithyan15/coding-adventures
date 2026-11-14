/**
 * # TypeScript lane runner for the language-neutral Warrant evidence corpus
 *
 * The Rust compiler (`warrant-grammar`) turns a package's `.warrant` file into a
 * `warrant-corpus-v1` directory. This package is the *TypeScript lane* of that
 * design (VER04i, WD-102): it loads the corpus, re-enumerates every exhaustive
 * domain in TypeScript, verifies the input and expected-value digests against
 * the corpus, drives a TypeScript library through its `evidence.bindings` table,
 * runs the negative controls that prove the runner can fail, and writes the
 * lane's `warrant-lane-evidence-v1` manifest.
 *
 * Reading order for newcomers:
 *
 *     values -> tfv -> domains -> ir -> corpus -> faults -> binding
 *            -> runner -> manifest -> session -> cli
 *
 * It has no runtime dependency (Node's built-in `crypto` is the only library it
 * uses) and no dependency on any library it tests.
 */

export { parseBinding, resolveExports, Subject, SubjectFailure } from "./binding.ts";
export type { Binding, Exports, OpBinding } from "./binding.ts";
export { main } from "./cli.ts";
export { loadCorpus } from "./corpus.ts";
export type { Corpus, Control, Example, Suite } from "./corpus.ts";
export { BindingError, CorpusError, LaneError, UsageError } from "./errors.ts";
export { buildManifest, frameSources, sourceDigest, validateManifest } from "./manifest.ts";
export { execute, outcomePassed, prepare } from "./session.ts";
export type { Outcome, Prepared, RunConfig } from "./session.ts";

export const VERSION = "0.1.0";
