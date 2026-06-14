/**
 * `Stage<In, Out>` — the universal contract every Forme pipeline
 * package implements (FM01 §3).
 *
 * A stage is a *value*, not a class.  The convention is for a stage
 * package to default-export the result of `defineStage(...)`.  The
 * helper exists purely to improve TypeScript inference; at runtime
 * `defineStage` is the identity function.
 *
 * === Why values, not classes ===
 *
 * Three reasons.  First, value-style stages are debuggable as plain
 * objects in the inspector — no instance hidden behind `this`.
 * Second, the orchestrator can serialise stage *metadata* (name,
 * version, capabilities) without instantiating anything; useful for
 * dry-run and dependency-graph dumps.  Third, the FM01 purity rule
 * (§3.3 — no module-level mutable state, no implicit dependencies)
 * is easier to enforce when there's no class to hide instance fields
 * in.
 *
 * === The three forms of `run` ===
 *
 * `run` may return:
 *
 *   - a `KindPayload<Out>` directly (synchronous one-to-one transform),
 *   - a `Promise<KindPayload<Out>>` (asynchronous one-to-one), or
 *   - an `AsyncIterable<KindPayload<Out>>` (streaming output).
 *
 * The orchestrator inspects the produced descriptor's `name` to know
 * which form to expect: a `Stream<K>` descriptor (built via `streamOf`
 * from `forme-types`) signals streaming output; anything else signals
 * single-value output.  Stages MUST match their declared shape — a
 * Stream-declared stage that returns a single value, or a single-value
 * stage that returns an AsyncIterable, will be flagged by the
 * orchestrator's first-call validation.
 */

import type {
  JsonValue,
  KindDescriptor,
  KindPayload,
} from "@coding-adventures/forme-types";
import type { Capability } from "@coding-adventures/forme-capability";
import type { StageContext, StageInitContext } from "./context.js";

/** A JSON-Schema-shaped value; the orchestrator runs the actual validator. */
export type JsonSchema = JsonValue;

/** The three valid output shapes (FM01 §3.6). */
export type StageOutput<Out extends KindDescriptor> =
  | KindPayload<Out>
  | Promise<KindPayload<Out>>
  | AsyncIterable<KindPayload<Out>>;

/**
 * The contract every stage implements.  Generic over the input and
 * output kind descriptors so TypeScript can infer the `run` method's
 * input/output types.
 */
export interface Stage<
  In extends KindDescriptor = KindDescriptor,
  Out extends KindDescriptor = KindDescriptor,
> {
  // ─── Static identification ──────────────────────────────────────────
  /** Package-qualified name, e.g. `"@forme/parse-markdown"`. */
  readonly name: string;
  /** Semver of this stage package. */
  readonly version: string;
  /** Forme kernel `apiVersion` this stage targets. */
  readonly apiVersion: number;
  /** Short human description for logs and tool UI. */
  readonly description: string;

  // ─── Type contract ──────────────────────────────────────────────────
  readonly consumes: In;
  readonly produces: Out;

  // ─── Capability declarations ────────────────────────────────────────
  /**
   * Every capability this stage may exercise.  Calls into context APIs
   * without a matching declaration receive a denied wrapper that throws
   * `CapabilityError` per the security contract.
   */
  readonly capabilities: readonly Capability[];

  // ─── Configuration ──────────────────────────────────────────────────
  /**
   * JSON Schema describing the stage's configuration object.  Null for
   * stages with no config.  The orchestrator validates pipeline config
   * against this before calling `run`.
   */
  readonly configSchema: JsonSchema | null;

  // ─── Execution ──────────────────────────────────────────────────────
  /**
   * Process a single input value to a single output, a Promise for a
   * single output, or a stream of outputs.
   *
   * Stages MUST NOT mutate their input.  Stages MUST NOT retain
   * references to context APIs after `run` resolves.
   */
  run(
    input: KindPayload<In>,
    config: unknown,
    ctx: StageContext,
  ): StageOutput<Out>;

  // ─── Optional lifecycle hooks ───────────────────────────────────────
  /**
   * Called once before the first `run`.  Use to prepare caches or
   * validate configuration that needs the host context.
   */
  init?(config: unknown, ctx: StageInitContext): Promise<void>;

  /**
   * Called once after the pipeline completes or cancels.  Stages MUST
   * release any resources acquired in `init` or across `run` calls.
   */
  dispose?(ctx: StageInitContext): Promise<void>;
}

/**
 * Type-narrowing identity helper for stage definitions.
 *
 * Without `defineStage`, an object literal exported as a `Stage` is
 * widened — `consumes`/`produces` become `KindDescriptor` instead of
 * the specific Kind, breaking inference for downstream use of
 * `KindPayload<typeof someStage.consumes>`.
 *
 * `defineStage(s)` returns the *same object* with the precise generic
 * parameters preserved.  At runtime it's just `s => s`.
 *
 * Usage:
 *
 *   export default defineStage({
 *     name:        "@forme/parse-markdown",
 *     version:     "0.1.0",
 *     apiVersion:  1,
 *     description: "Parses CommonMark + GFM into a ContentNode.",
 *     consumes:    Kinds.ContentSource,
 *     produces:    Kinds.ContentNode,
 *     capabilities: [],
 *     configSchema: null,
 *     async run(source, config, ctx) { ... },
 *   });
 */
export function defineStage<
  In extends KindDescriptor,
  Out extends KindDescriptor,
>(stage: Stage<In, Out>): Stage<In, Out> {
  return stage;
}
