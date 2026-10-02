/**
 * Public FM05 Interactivity IR v1 shapes.
 *
 * These interfaces contain data only. No field carries source text, a module
 * path, a host handle, or an ambient capability grant.
 */

import type { JsonValue } from "@coding-adventures/forme-types";

export type StateScope = "block" | "document" | "site" | "session";
export type StateType = "boolean" | "number" | "string" | "enum" | "json";
export type Persistence = "none" | "session" | "local";
export type IslandActivation = "load" | "visible" | "interaction";

export type NodeRef =
  | { readonly kind: "document" }
  | { readonly kind: "element"; readonly id: string };

export interface StateDeclaration {
  readonly name: string;
  readonly scope: StateScope;
  readonly type: StateType;
  readonly initial: JsonValue;
  readonly persist: Persistence;
  readonly owner?: NodeRef;
  readonly values?: readonly string[];
}

export type ValueExpr =
  | { readonly kind: "literal"; readonly value: JsonValue }
  | { readonly kind: "state"; readonly state: string };

export type Predicate =
  | { readonly kind: "truthy"; readonly value: ValueExpr }
  | { readonly kind: "equals"; readonly left: ValueExpr; readonly right: ValueExpr }
  | { readonly kind: "not"; readonly predicate: Predicate }
  | { readonly kind: "all"; readonly predicates: readonly Predicate[] }
  | { readonly kind: "any"; readonly predicates: readonly Predicate[] };

export type BindingApplication =
  | { readonly kind: "visible"; readonly whenTrue: boolean }
  | { readonly kind: "text"; readonly value: ValueExpr }
  | { readonly kind: "value"; readonly value: ValueExpr }
  | { readonly kind: "extension"; readonly name: string; readonly value: JsonValue };

export interface Binding {
  readonly id: string;
  readonly target: NodeRef;
  readonly when: Predicate;
  readonly apply: BindingApplication;
}

export type Trigger =
  | { readonly kind: "click" }
  | { readonly kind: "focus" }
  | { readonly kind: "blur" }
  | { readonly kind: "input" }
  | { readonly kind: "change" }
  | { readonly kind: "submit" }
  | { readonly kind: "visible"; readonly threshold: number }
  | { readonly kind: "timer"; readonly afterMs: number }
  | { readonly kind: "custom"; readonly name: string };

export type Effect =
  | { readonly kind: "set-state"; readonly state: string; readonly value: ValueExpr }
  | { readonly kind: "toggle-state"; readonly state: string }
  | { readonly kind: "navigate"; readonly to: string }
  | { readonly kind: "dispatch"; readonly event: string; readonly detail: JsonValue }
  | { readonly kind: "run-island"; readonly island: string; readonly args: JsonValue };

export interface Handler {
  readonly id: string;
  readonly target: NodeRef;
  readonly on: Trigger;
  readonly effects: readonly Effect[];
}

export interface IslandDeclaration {
  readonly id: string;
  readonly packageName: string;
  readonly export: string;
  readonly target: Extract<NodeRef, { readonly kind: "element" }>;
  readonly fallback: Extract<NodeRef, { readonly kind: "element" }>;
  readonly activation: IslandActivation;
  readonly config: JsonValue;
}

export interface InteractivityDocument {
  readonly kind: "Interactivity";
  readonly version: 1;
  readonly state: readonly StateDeclaration[];
  readonly bindings: readonly Binding[];
  readonly handlers: readonly Handler[];
  readonly islands: readonly IslandDeclaration[];
}

export type InteractivityErrorCode =
  | "INVALID_TYPE"
  | "INVALID_VALUE"
  | "INVALID_LIMIT"
  | "MISSING_FIELD"
  | "UNKNOWN_FIELD"
  | "ACCESSOR"
  | "SYMBOL_KEY"
  | "SPARSE_ARRAY"
  | "REPEATED_VALUE"
  | "INVALID_IDENTIFIER"
  | "DUPLICATE_ID"
  | "DANGLING_REFERENCE"
  | "TYPE_MISMATCH"
  | "LIMIT_EXCEEDED"
  | "INVALID_URL";

export interface InteractivityLimits {
  readonly maxCanonicalBytes: number;
  readonly maxState: number;
  readonly maxBindings: number;
  readonly maxHandlers: number;
  readonly maxIslands: number;
  readonly maxEffectsPerHandler: number;
  readonly maxPredicateOperands: number;
  readonly maxExpressionDepth: number;
  readonly maxExpressionNodes: number;
  readonly maxJsonDepth: number;
  readonly maxJsonNodes: number;
  readonly maxStringBytes: number;
}
