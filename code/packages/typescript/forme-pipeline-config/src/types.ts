/**
 * `PipelineConfig` and friends — the user-authored description of *what
 * to build* (FM03 §2).
 *
 * Two surface forms in the spec — TypeScript and TOML — share this
 * single underlying type.  v0 implements the TypeScript form only
 * (`loadTsConfig`); the TOML loader lives in a sibling that compiles
 * to the same shape and is gated behind FM03's "TOML-only feature is
 * a feature gap" rule.
 *
 * === Stages by value vs. by reference ===
 *
 * In the TS form, `stage` is an imported `Stage<In, Out>` value — the
 * compiler verifies the consumes/produces descriptors at edit time.
 *
 * In the TOML form (and in plugin-host flows generally), `stage` is a
 * `StageRef` naming a package; FM02's plugin host resolves it to a
 * loaded Stage at typecheck time.  v0 doesn't ship FM02, so the
 * default-direct-import host (FM03 §12) treats every direct value as
 * already-loaded and refuses to honour `StageRef`s.
 *
 * === Why optional `id`, `wires`, `outputs`, and `capabilities` ===
 *
 * The common case is a linear pipeline of unique stages — one
 * `source-fs`, one `parse-markdown`, one `render-static`, one
 * `emit-fs`.  Defaults handle that:
 *
 *   - `id` defaults to `stage.name` when no other instance shares it.
 *     The validator rejects collisions and demands explicit IDs only
 *     when the same stage appears more than once.
 *
 *   - `wires` is empty: edges are inferred from kind compatibility,
 *     declaration order is the tiebreaker (FM03 §3.3).
 *
 *   - `outputs` is required only when more than one emitter is
 *     present (FM03 §3.3 step 5).  A single-emitter pipeline doesn't
 *     need to name its sole output.
 *
 *   - Per-instance `capabilities` defaults to the stage's own
 *     declarations.  Override only when the host wants to grant a
 *     subset (e.g. a pipeline run that explicitly denies network
 *     access for one specific instance).
 */

import type {
  KindDescriptor,
} from "@coding-adventures/forme-types";
import type { Stage } from "@coding-adventures/forme-stage";
import type { Capability } from "@coding-adventures/forme-capability";

// ─── Settings ─────────────────────────────────────────────────────────────

/** Pipeline-wide settings — see FM03 §2.1 PipelineSettings. */
export interface PipelineSettings {
  /** Storage root for the pipeline's StorageApi. */
  readonly storageRoot: string;
  /** Where the orchestrator's persistent cache lives, or null to disable. */
  readonly cacheDir: string | null;
  /** Reproducible-build mode (FM03 §8). */
  readonly reproducibleBuild: boolean;
  /** Maximum stage-level parallelism.  Null = hardware concurrency. */
  readonly maxConcurrency: number | null;
  /** Logging verbosity. */
  readonly logLevel: "trace" | "debug" | "info" | "warn" | "error";
  /** Continue past recoverable errors and report at the end. */
  readonly bestEffort: boolean;
  /** Maximum wall-clock for the entire run.  Null = unlimited. */
  readonly deadlineMs: number | null;
}

// ─── Stage references ─────────────────────────────────────────────────────

/**
 * Indirect reference to a stage that the plugin host (FM02) resolves
 * at typecheck time.  v0's default-direct-import host refuses to load
 * these — see FM03 §12.
 */
export interface StageRef {
  readonly kind: "stage-ref";
  /** Package name as known to the plugin host. */
  readonly packageName: string;
  /** Optional sub-export name; defaults to `default`. */
  readonly export?: string;
}

/** Predicate: is this value a `StageRef` rather than a direct `Stage`? */
export function isStageRef(value: unknown): value is StageRef {
  return (
    typeof value === "object"
    && value !== null
    && (value as { kind?: unknown }).kind === "stage-ref"
    && typeof (value as { packageName?: unknown }).packageName === "string"
  );
}

// ─── Stage instance spec ──────────────────────────────────────────────────

/** One use of a stage in a pipeline.  Same stage can appear multiple times. */
export interface StageInstanceSpec {
  /**
   * The stage value or a deferred `StageRef` for the plugin host to
   * resolve.  Direct-import flows pass values; manifest flows pass refs.
   */
  readonly stage: Stage<KindDescriptor, KindDescriptor> | StageRef;
  /**
   * Stable, user-chosen instance ID.  Defaults to `stage.name` when
   * no other instance shares it; required when collisions exist.
   */
  readonly id?: string;
  /** Configuration passed to the stage.  Validated against `stage.configSchema`. */
  readonly config?: unknown;
  /**
   * Capability grants for this instance.  Defaults to the stage's own
   * declared capabilities; explicit grants must be a subset.
   */
  readonly capabilities?: readonly Capability[];
}

// ─── Edges & outputs ──────────────────────────────────────────────────────

/** Explicit edge from one instance's output to another's input. */
export interface EdgeSpec {
  readonly from: { readonly id: string; readonly port?: string };
  readonly to:   { readonly id: string; readonly port?: string };
}

/** Friendly name for an emitter's output, required when >1 emitter exists. */
export interface OutputSpec {
  readonly fromInstance: string;
  readonly name: string;
}

// ─── PipelineConfig ───────────────────────────────────────────────────────

/**
 * The complete user-authored description of a pipeline.  In TypeScript
 * form, default-exported by a `forme.config.ts` and consumed by the
 * orchestrator via `loadTsConfig` + `validateConfig`.
 */
export interface PipelineConfig {
  /** Human-friendly identifier used in logs and the dev-server UI. */
  readonly name: string;
  /** Pipeline-wide settings. */
  readonly settings: PipelineSettings;
  /**
   * Ordered list of stage instances.  Order is human-readability +
   * tiebreaking only — the orchestrator infers the actual DAG from
   * declared kinds (FM03 §3.3).
   */
  readonly stages: readonly StageInstanceSpec[];
  /** Explicit edges, used when type inference is ambiguous.  Empty by default. */
  readonly wires?: readonly EdgeSpec[];
  /** Output destinations when more than one emitter is present. */
  readonly outputs?: readonly OutputSpec[];
}
