# gfm-parser

TypeScript gfm-parser package.

## Portable extension conformance

The package test suite dynamically reads all sixteen cases from the shared
[`gfm-extension-v1`](../../../specs/fixtures/gfm-extension-v1/README.md)
corpus. Every source is parsed through the public `parse()` API, and the
resulting complete Document AST and public `toHtml()` output are compared with
the language-neutral oracle. The suite also proves that independent source,
AST, and HTML corruption cannot pass unnoticed.

The fixture remains test-only: it is not bundled into the published parser and
fenced payloads remain inert strings. An exact corpus change schedules this
package and its ordinary dependent closure in the repository build plan.

## TE04 fenced-block compatibility

GFM fences now produce `fenced_block` with the entire normalized `info`, its first-token `name`, and literal payload. Info is decoded once, interior whitespace is retained, and absent or empty normalized info produces null hints; empty payloads remain valid. Indented code remains `code_block`.

See [TE04](../../../specs/TE04-gfm-extensions.md) for normalization,
source retention, compatibility and validation boundaries. Run package-local
`npm run build` when supplied and `npm test -- --coverage` after installing
local dependencies; BUILD fronts also install required sibling packages.

Fence metadata retains literal U+2028/U+2029 and opposite fence characters;
only the recognized opener delimiter is removed before normalization.
