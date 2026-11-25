/**
 * # The v2 runner's refusals
 *
 * VER04r section 11 gives every conformance category a *closed* set of error
 * codes, and says the codes are the kit's own vocabulary: neither the oracle's
 * nor any lane's.  This runner therefore throws one exception class,
 * {@link KitError}, whose `code` is always a member of one of those closed
 * sets, so the kit harness can compare it with an `expect` object directly.
 *
 * | code             | category it belongs to | what it says                                     |
 * |------------------|------------------------|--------------------------------------------------|
 * | `malformed-json` | v2-value               | the strict JSON reader refused the bytes         |
 * | `type`           | v2-value               | the type text is not a section 2.1 type          |
 * | `value`          | v2-value               | the JSON does not fit the declared type          |
 * | `ir-overflow`    | v2-arith, v2-ir        | an exact integer or a bytes value left its bound |
 * | `unknown-tag`    | v2-ir                  | a tag no level defines                           |
 * | `level-missing`  | v2-ir, v2-fault        | a tag or type of a level this runner lacks       |
 * | `depth`          | v2-ir                  | expression or model-call nesting past 64         |
 * | `model-order`    | v2-ir                  | a `model_call` forward, missing or self reference|
 * | `subject-forbidden` | v2-ir, v2-fault     | a `subject_op` where none may stand              |
 * | `ill-typed`      | v2-ir                  | any other static type error                      |
 * | `impure-guard`   | v2-ir                  | a subject call under a guard that calls one      |
 * | `ir-cost`        | v2-ir                  | a worst-path cost over `max_ir_cost`             |
 * | `descriptor`     | v2-domain              | a descriptor section 5.1 refuses                 |
 * | `index`          | v2-domain              | an index or range outside the domain             |
 * | `corpus`         | v2-chunk               | the corpus part or suite row is refused          |
 * | `slim-chunks`    | v2-chunk               | `slim_chunks` is malformed                       |
 * | `chunk-file`     | v2-chunk               | the chunk file fails the section 5.5 grammar     |
 *
 * A message never echoes an input value (VER04r section 14: nothing that could
 * reproduce hostile or secret input leaves the process); it names the *kind*
 * of problem.
 */

/** Every code any category may use. */
export type KitCode =
  | "malformed-json"
  | "type"
  | "value"
  | "ir-overflow"
  | "unknown-tag"
  | "level-missing"
  | "depth"
  | "model-order"
  | "subject-forbidden"
  | "ill-typed"
  | "impure-guard"
  | "ir-cost"
  | "descriptor"
  | "index"
  | "corpus"
  | "slim-chunks"
  | "chunk-file";

/** Where an overflow happened, in the split of section 3.2 and R71. */
export type OverflowClass = "corpus" | "case";

/**
 * A refusal with a kit code.
 *
 * `overflowClass` is set only for `ir-overflow` raised by the IR evaluator:
 * `corpus` means the node is one the generator evaluates (so an honest corpus
 * never meets it and the corpus is corrupt), `case` means an ordinary case
 * failure (what a subject returned made the node overflow).  `level` names the
 * missing level for `level-missing` (so a lane can record `not-live` with the
 * level it lacks).
 */
export class KitError extends Error {
  readonly code: KitCode;
  readonly overflowClass: OverflowClass | undefined;
  readonly level: string | undefined;

  constructor(code: KitCode, message: string, extra: { overflowClass?: OverflowClass; level?: string } = {}) {
    super(`${code}: ${message}`);
    this.code = code;
    this.overflowClass = extra.overflowClass;
    this.level = extra.level;
  }
}
