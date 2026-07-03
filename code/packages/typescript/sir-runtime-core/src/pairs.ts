/**
 * Cons pairs — re-exported from `@coding-adventures/sir-runtime-pairs`.
 *
 * The SIR `Pair` value type (`cons` / `car` / `cdr`) used to live here, but it
 * is a self-contained per-concern quirk, so it has moved to its own publishable
 * package. This module is now a thin **re-export shim** kept for
 * back-compatibility: every existing intra-core import (`import { Pair } from
 * "./pairs.js"`) and external consumer keeps working unchanged, and a value
 * built by `core.cons` is the *same* class as one built by the dedicated
 * package (no two-`Pair`-classes hazard).
 *
 * The pairs package deliberately depends on **nothing** — its Lisp-list display
 * calls an injectable hook. Core wires its richer {@link toDisplay} into that
 * hook in `index.ts` via `setDisplay`, so a pair still renders as `(1 2 3)`
 * once core is imported. See `code/specs/sir-runtime.md`.
 */

export {
  Pair,
  cons,
  car,
  cdr,
  isPair,
  setDisplay,
} from "@coding-adventures/sir-runtime-pairs";
