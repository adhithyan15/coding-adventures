# FM05 — Forme Interactivity IR

> **Status:** Canonical location reserved; normative v1 contract pending.
> **Scope:** The backend-neutral behavior, event, state, binding, and island
> representation carried alongside Content IR and Style IR.
> **Delivery owner:** FM-B013 in the
> [Forme completion roadmap](FM00-forme-completion-roadmap.md).

## Implementation status

| Surface | Status | Evidence / next step |
|---|---|---|
| Kernel kind name | Placeholder implemented | FM01 and `forme-types` reserve the `Interactivity` kind. |
| Behavior/event/state schema | Pending | FM-B013 must define the normative, versioned schema and validator. |
| Per-page island tracking | Pending | FM-B013 must connect renderer usage to AOT selection. |
| Progressive-enhancement proof | Pending | FM-B013 must ship an interactive component with a no-JavaScript fallback. |
| Non-web degradation | Pending | FM-B017 owns explicit terminal/print/email behavior. |

## 1. Purpose and authority

This file is the canonical home of FM05. Earlier Forme specifications reserved
FM05 for Interactivity IR, but a deploy-runner draft later reused the number.
That deploy contract now lives at [FM08](FM08-forme-deploy-runner.md).

FM00 §3.3 remains the design sketch until FM-B013 replaces this location
document with the normative schema. Code must not infer a stable wire format
from the current `Interactivity` placeholder.

## 2. Stable boundary

Interactivity IR is parallel to Content IR and
[Style IR](FM04-forme-style-ir.md). It describes behavior without embedding a
browser framework, executable source, or privileged host handle in authored
content. Backends may compile supported behavior, preserve a progressively
enhanced fallback, or explicitly report a degradation.

The eventual contract must define:

1. versioned state, event, predicate, action, binding, and island records;
2. deterministic validation and canonical serialization;
3. per-page usage tracking for [FM06](FM06-forme-aot-compiler.md);
4. capability boundaries between declarative behavior and plugin execution;
5. fallback and accessibility semantics for unsupported backends.

## 3. Delivery gate

FM05 becomes code-ready only when FM-B013 lands a schema, validator, package
surface, security limits, conformance tests, and a live product proof. Until
then the implementation ledger above is authoritative about what is absent.

## 4. Related specifications

- [FM00](FM00-forme-vision.md) — product vision and the original IR sketch
- [FM01](FM01-forme-kernel.md) — kind and document-container contracts
- [FM02](FM02-forme-plugin-host.md) — untrusted extension boundary
- [FM04](FM04-forme-style-ir.md) — parallel backend-neutral Style IR
- [FM06](FM06-forme-aot-compiler.md) — per-page dependency selection
- [FM07](FM07-forme-cli-dev-server.md) — preview and author-facing runtime
