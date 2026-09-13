---
category: Testing & coverage
---

# `decompose_text` silently falls back to a hand-built fixture when the LLM emits unterminated JSON

Logged as `decompose_text FAILED: ... EOF while parsing` then `fallback: hand-built TSA fixture` on stderr-mixed stdout. Any bench that infers "model emitted no quantities" from an empty term-tree will misclassify these as model failures when they're really gateway/output-budget failures. Capture the failure-mode line explicitly.
