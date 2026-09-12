# UI75: Shared details disclosure semantics

## Goal

Venture owns HTML `details` state above every native and generated host. Closed
disclosures lay out only their first direct `summary`; open disclosures lay out
the complete child list. A missing authored summary receives the deterministic
label `Details` so it remains visible and operable.

## Shared contract

- `html-to-layout` filters closed disclosure children before shared layout.
- `html-to-paint` emits transformed, clipped, scroll-aware summary hit regions
  with stable ID-derived keys and document-order indexes.
- `venture-browser-core` owns pointer, Enter/Space, and accessibility open,
  close, and toggle actions. Opening one non-empty named disclosure closes the
  other members of its group transactionally before one retained-page reflow.
- Controls and links nested in a summary keep activation precedence over the
  enclosing disclosure.
- Hosts forward page pointer and keyboard input to the shared reducer; they do
  not infer visibility, group membership, labels, or open state.

## Acceptance

The deterministic fixture covers closed, open, grouped, linked-summary, and
generated-summary cases. Package tests require the same paint-region contract
for every generated host, while core tests exercise pointer, keyboard, grouped,
and accessibility transitions.
