/**
 * @coding-adventures/forme-interactivity-ir
 *
 * FM05's bounded, declarative Interactivity IR v1. The package exposes only
 * immutable types, hostile-input validation, and deterministic serialization;
 * rendering and executable island loading remain separate trust boundaries.
 */

export { canonicalInteractivityDocument } from "./canonical.js";
export { InteractivityError } from "./error.js";
export {
  DEFAULT_INTERACTIVITY_LIMITS,
  HARD_INTERACTIVITY_LIMITS,
} from "./limits.js";
export {
  EMPTY_INTERACTIVITY,
  validateInteractivityDocument,
} from "./validate.js";
export type {
  Binding,
  BindingApplication,
  Effect,
  Handler,
  InteractivityDocument,
  InteractivityErrorCode,
  InteractivityLimits,
  IslandActivation,
  IslandDeclaration,
  NodeRef,
  Persistence,
  Predicate,
  StateDeclaration,
  StateScope,
  StateType,
  Trigger,
  ValueExpr,
} from "./types.js";
