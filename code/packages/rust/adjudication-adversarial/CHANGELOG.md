# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-11

### Added

ADJ05 adversarial verifier. Composes `render_node` +
`find_contradicting_reading` + `judge_plausibility` from
`llm-primitives` into a three-step loop per sampled leaf IR node.

- `check_adversarial(document_text, ir_doc, gateway, opts)
  -> Result<AdversarialResult, CheckError>`.
- `CheckOptions { style: RenderStyle, domain_hint: String }`.
- `AdversarialResult { violations, call_records }` + `pass()`.
- `AdversarialViolation { node_id, ir_rendered, adversary_reading,
  adversary_explanation, judge_reason }`.
- `CheckError::Primitive(PrimitiveError) | LeafMissingSpans |
  SpanOutOfBounds`.

The three outcomes ADJ05 distinguishes:

| Adversary | Judge | Result |
|---|---|---|
| `Concurs` | (not called) | no violation; 2 calls recorded |
| `Reading` | `IMPLAUSIBLE` | no violation; reading + judge in trail |
| `Reading` | `PLAUSIBLE` | `AdversarialViolation` recorded |

The "implausible reading still goes into the audit trail" behaviour
is explicit in ADJ05 — the framework wants every adversarial finding
visible to the reviewer, even when not promoted to a gating
violation.

11 tests cover: adversary Concurs short-circuits (2 calls, no
violation); implausible reading is logged but not gating; plausible
reading becomes AdversarialViolation with all fields populated;
no-attackable-nodes is no-op; missing Renderer / Adversary /
Plausibility clients each return typed Primitive(NoClientForRole);
span-out-of-bounds and leaf-missing-spans return typed errors;
multi-node trails are correctly interleaved (Concurs short-circuits
mid-loop too); Discarded nodes skipped.

### Notes

This crate is **stacked on PR #2765** (the
`find_contradicting_reading` primitive). When #2765 merges, this PR
rebases trivially against main.

Reference: [ADJ05](../../../specs/ADJ05-adversarial-verifier.md).
