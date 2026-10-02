# @coding-adventures/forme-interactivity-ir

The bounded, declarative Forme Interactivity IR defined by
[FM05](../../../specs/FM05-forme-interactivity-ir.md). It represents state,
bindings, handlers, and progressively enhanced islands as immutable data.

The package does not execute authored code, resolve packages, access the
network or filesystem, or grant capabilities. Island loading remains behind
the Forme plugin and product boundary.

## API

```ts
import {
  canonicalInteractivityDocument,
  validateInteractivityDocument,
} from "@coding-adventures/forme-interactivity-ir";

const document = validateInteractivityDocument({
  kind: "Interactivity",
  version: 1,
  state: [{
    name: "menu-open",
    scope: "document",
    type: "boolean",
    initial: false,
    persist: "none",
  }],
  bindings: [],
  handlers: [],
  islands: [],
});

const canonical = canonicalInteractivityDocument(document);
```

`validateInteractivityDocument` accepts hostile `unknown` input in Node,
returns a
fresh deeply frozen snapshot, and throws `InteractivityError` with a stable
code and path on failure. It rejects unknown fields, accessors, symbol keys,
sparse arrays, proxies, repeated identities and cycles, dangling references,
unsafe navigation, incompatible state effects, ill-formed Unicode, and every
configured resource-limit violation.

The optional limits object is also validated and snapshotted without invoking
getters. Defaults bound canonical bytes, collection sizes, expression and JSON
depth/node counts, effect and predicate fan-out, and individual string bytes.
The validator accounts for escaped string bytes while walking, so an oversized
document fails before it can accumulate or serialize an unbounded string set.

`canonicalInteractivityDocument` sorts object keys recursively while
preserving authored array order. Validated documents therefore produce stable
JSON suitable for hashing and cache keys.

`EMPTY_INTERACTIVITY` is the frozen zero-JavaScript document. A renderer can
use it without emitting a loader or empty runtime shell.

## Trust boundary

Interactivity effects are a closed set: state updates, safe navigation,
document-local dispatch, and references to reviewed islands. There is no raw
source, selector language, dynamic property traversal, module path, URL-based
module load, shell, environment, or ambient host handle in the IR.

The follow-up FM-B060 integration resolves element references against content
and emits only the islands selected by each page. FM-B061 supplies the live
progressive-enhancement proof.

## Development

```sh
npm install
npm run build
npm run test:coverage
```

The package depends on `@coding-adventures/forme-types` plus Node's built-in
non-trapping Proxy detector. It declares no host capabilities in
`required_capabilities.json`.
