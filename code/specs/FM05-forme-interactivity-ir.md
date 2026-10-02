# FM05 — Forme Interactivity IR

> **Status:** Normative v1 contract active in FM-B059.
> **Scope:** The backend-neutral state, predicate, binding, event-handler, and
> island representation carried alongside Content IR and Style IR.
> **Delivery owner:** FM-B059–FM-B061 and the FM-B013 completion milestone in
> the [Forme completion roadmap](FM00-forme-completion-roadmap.md).

## Implementation status

| Surface | Status | Evidence / next step |
|---|---|---|
| Kernel kind name | Placeholder implemented | FM01 and `forme-types` reserve the `Interactivity` kind. |
| Behavior/event/state schema | Active | FM-B059 implements this contract in `forme-interactivity-ir`. |
| Bounded validation and canonical bytes | Active | FM-B059 rejects hostile values and emits deterministic JSON. |
| Per-page island tracking | Blocked | FM-B060 follows the validated contract and records exact `usedIslands`. |
| Progressive-enhancement proof | Blocked | FM-B061 ships a live fallback plus one bounded enhancement. |
| Non-web degradation | Pending | FM-B017 owns explicit terminal/print/email behavior. |

## 1. Purpose and authority

This file is the canonical Interactivity IR contract. Earlier Forme
specifications reserved FM05 for Interactivity IR, but a deploy-runner draft
later reused the number. That deploy contract now lives at
[FM08](FM08-forme-deploy-runner.md).

Interactivity IR is data, not executable source. It lets a backend answer four
questions without running authored code:

1. Which named state can change?
2. Which stable authored element is affected?
3. Which bounded declarative effect follows an event?
4. Which separately reviewed island module must be selected for this page?

The contract is parallel to Content IR and
[Style IR](FM04-forme-style-ir.md). A backend may realize supported behavior,
preserve the required fallback, or explicitly report a degradation. It MUST
NOT silently remove the fallback merely because it can load an island.

## 2. Design constraints

The v1 representation follows these rules:

- Every union is discriminated by `kind`; unknown fields and union members are
  errors rather than extension points.
- Names are document-local ASCII identifiers. They are stable references, not
  JavaScript property paths or selectors.
- Element references identify authored semantic elements by stable ID. They do
  not contain CSS selectors, XPath, HTML, or executable expressions.
- Effects are a closed declarative set. Network, filesystem, environment,
  shell, clock, randomness, and raw-code effects are absent.
- An island names a package export that is resolved and authorized through the
  FM02 plugin/product boundary. The IR never carries module bytes, a local path,
  a URL, or an ambient capability grant.
- Validation snapshots the complete accepted value. Later caller mutation
  cannot change the validated document or its canonical bytes.
- Empty interactivity is valid and must keep the static path at zero JavaScript.

## 3. Top-level document

```typescript
export interface InteractivityDocument {
  readonly kind: "Interactivity";
  readonly version: 1;
  readonly state: readonly StateDeclaration[];
  readonly bindings: readonly Binding[];
  readonly handlers: readonly Handler[];
  readonly islands: readonly IslandDeclaration[];
}
```

`state`, `bindings`, `handlers`, and `islands` preserve authored order in the
validated value and canonical serialization. Names and IDs MUST be unique in
their respective arrays. A reference MUST resolve within the same document.

The empty document is:

```json
{"kind":"Interactivity","version":1,"state":[],"bindings":[],"handlers":[],"islands":[]}
```

## 4. Identifiers and references

`StateName`, `BindingId`, `HandlerId`, `IslandId`, custom-event names, and
authored element IDs use this grammar:

```text
[a-z][a-z0-9]*(?:-[a-z0-9]+)*
```

They contain 1–64 ASCII characters. Package names follow FM02's canonical
package-name grammar and are at most 214 characters. Export names use
`[A-Za-z_][A-Za-z0-9_]{0,63}`.

```typescript
export type NodeRef =
  | { readonly kind: "document" }
  | { readonly kind: "element"; readonly id: string };
```

`document` is valid for handlers and document-wide bindings. An island target
and fallback MUST be an `element` reference: progressively enhanced behavior
must point at an authored region that exists without JavaScript. FM-B060 owns
resolution against rendered content and rejects missing or ambiguous element
IDs before emitting a page.

## 5. State

```typescript
export type StateScope = "block" | "document" | "site" | "session";
export type StateType = "boolean" | "number" | "string" | "enum" | "json";
export type Persistence = "none" | "session" | "local";

export interface StateDeclaration {
  readonly name: string;
  readonly scope: StateScope;
  readonly type: StateType;
  readonly initial: JsonValue;
  readonly persist: Persistence;
  readonly owner?: NodeRef;
  readonly values?: readonly string[];
}
```

- `owner` is required and must be an element reference for `block` scope. It
  is forbidden for every other scope.
- `values` is required only for `enum`, contains 1–256 unique strings, and the
  initial value must be one of them.
- Boolean, number, and string initials must match their declared type. Numbers
  must be finite. `json` accepts any bounded `JsonValue`.
- `persist: "session"` and `persist: "local"` describe client persistence, not
  host storage capability. Backends that cannot provide it report degradation.

## 6. Values and predicates

```typescript
export type ValueExpr =
  | { readonly kind: "literal"; readonly value: JsonValue }
  | { readonly kind: "state"; readonly state: string };

export type Predicate =
  | { readonly kind: "truthy"; readonly value: ValueExpr }
  | { readonly kind: "equals"; readonly left: ValueExpr; readonly right: ValueExpr }
  | { readonly kind: "not"; readonly predicate: Predicate }
  | { readonly kind: "all"; readonly predicates: readonly Predicate[] }
  | { readonly kind: "any"; readonly predicates: readonly Predicate[] };
```

Every state reference must resolve. `all` and `any` contain 1–64 predicates.
Predicate nesting and total expression-node limits in §11 apply before any
backend evaluates them.

There is deliberately no arithmetic, property traversal, function call,
regular expression, dynamic key, or string-to-code operation in v1. Richer
computation belongs in a reviewed island.

## 7. Bindings

```typescript
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
```

`visible` changes presentation while preserving authored content as the
no-script fallback. `text` and form `value` replace only text/value slots; they
cannot inject markup. An extension name uses the identifier grammar and its
consumer must be declared by an installed plugin. Unknown extensions are
validation warnings at composition time and must not become ambient execution.

## 8. Triggers, effects, and handlers

```typescript
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
```

A handler contains 1–64 effects, executed in order. State and island references
must resolve. `toggle-state` requires a boolean declaration. A literal
`set-state` value must match the declaration; a state expression is checked for
compatible declared type.

`visible.threshold` is finite in `[0, 1]`. `timer.afterMs` is a safe integer in
`[0, 86_400_000]`. Navigation accepts only a portable root-relative route, a
same-document fragment, or an absolute HTTPS URL; scheme-relative URLs,
backslashes, credentials, and ASCII controls are rejected. `dispatch` creates
only a document-local declarative event. It does not invoke an operating-system
or plugin event bus.

## 9. Islands and progressive enhancement

```typescript
export type IslandActivation = "load" | "visible" | "interaction";

export interface IslandDeclaration {
  readonly id: string;
  readonly packageName: string;
  readonly export: string;
  readonly target: Extract<NodeRef, { readonly kind: "element" }>;
  readonly fallback: Extract<NodeRef, { readonly kind: "element" }>;
  readonly activation: IslandActivation;
  readonly config: JsonValue;
}
```

An island is a reference to a reviewed package export, not code. `target`
identifies the region the island may enhance. `fallback` identifies useful
authored content that remains present when scripts are blocked, fail to load,
or the backend does not implement islands. Target and fallback may be the same
element.

`interaction` activation requires at least one handler that references the
island through `run-island`. `visible` and `load` activation are selected by the
page when their target resolves. An unused declaration is rejected so the
canonical document cannot hide dormant executable dependencies.

FM-B060 must emit exactly the selected `IslandId` values in `usedIslands` and
the deploy route. A page with no selected declarations emits no island module,
loader, inline program, or empty runtime shell.

## 10. Canonical serialization

`canonicalInteractivityDocument(document)` returns UTF-8 JSON text with:

1. object keys sorted by Unicode scalar value;
2. array order preserved;
3. no insignificant whitespace;
4. finite JSON numbers in ECMAScript JSON number form;
5. no `undefined`, sparse arrays, accessors, symbols, non-plain objects, or
   cyclic references.

Validation constructs and deeply freezes a fresh snapshot before canonical
serialization. Canonical bytes therefore do not depend on property insertion
order, prototypes, getters, or mutation after validation.

## 11. Validation and resource limits

`validateInteractivityDocument(value, limits?)` either returns a deeply frozen
validated snapshot or throws `InteractivityError`. Limits are validated,
copied, and frozen before the input is inspected.

The defaults are:

| Limit | Default |
|---|---:|
| Canonical UTF-8 bytes | 1 MiB |
| State declarations | 256 |
| Bindings | 1,024 |
| Handlers | 1,024 |
| Islands | 256 |
| Effects per handler | 64 |
| Predicate operands | 64 |
| Expression depth | 32 |
| Total expression/value nodes | 8,192 |
| JSON depth | 32 |
| JSON nodes | 16,384 |
| General string bytes | 4,096 |

Every configured limit must be a positive safe integer and may not exceed the
hard implementation ceiling. The validator reads each limit property once,
rejects accessors, snapshots it, and never retains the caller's object.

Validation is fail-closed and deterministic. It rejects:

- a non-plain object, accessor, symbol key, unknown/missing field, or sparse
  array at any IR-owned position, and every Proxy before invoking a meta-trap;
- a cyclic graph or repeated object identity (the wire format is a tree);
- invalid identifiers, duplicate declarations, and dangling state/island
  references;
- type-incompatible initials or effects, non-finite numbers, unsafe timer
  bounds, and unsafe navigation targets;
- an island without a resolvable fallback contract or an unused declaration;
- any count, depth, string, JSON-node, expression-node, or canonical-byte
  limit violation.

Canonical UTF-8 bytes are counted exactly in a bounded descriptor walk before
a snapshot or output string is built. Strings with unpaired UTF-16
surrogates are rejected because they do not define a Unicode scalar sequence.

Errors carry a stable code and JSON-pointer-like path. Attacker-controlled path
segments are length-bounded and ASCII-escaped; diagnostics must never echo the
whole rejected document or raw terminal controls.

## 12. Capability and trust boundary

The declarative evaluator needs no FM01 capability. Persistence named in state
is client behavior and cannot access host storage. Navigation is data validated
against §8. Dispatch is document-local. No effect can fetch, read a file,
inspect environment variables, spawn a process, or execute source text.

Island code remains executable plugin code. Its package is discovered,
installed, granted, launched, and sandboxed through FM02/FM07. Interactivity IR
does not confer capabilities and cannot widen the island manifest's effective
grant set. Backends must not resolve a package path or import an island before
the product plugin boundary has authenticated the exact selected package.

## 13. Delivery plan

FM-B059 ships the types, bounded validator, canonical serializer, conformance
tests, package documentation, and this normative contract. FM-B060 composes
validated documents with rendering and AOT/deploy selection. FM-B061 adds the
live progressive-enhancement proof. FM-B013 closes only after all three merge.

## 14. Related specifications

- [FM00](FM00-forme-vision.md) — product vision and original design sketch
- [FM01](FM01-forme-kernel.md) — kinds, document containers, and usage records
- [FM02](FM02-forme-plugin-host.md) — executable extension trust boundary
- [FM04](FM04-forme-style-ir.md) — parallel backend-neutral Style IR
- [FM06](FM06-forme-aot-compiler.md) — per-page dependency selection
- [FM07](FM07-forme-cli-dev-server.md) — product plugin composition and preview
- [FM08](FM08-forme-deploy-runner.md) — deployment manifest and publication
