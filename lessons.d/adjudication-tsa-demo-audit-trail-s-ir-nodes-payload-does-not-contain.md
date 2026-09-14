---
category: Testing & coverage
---

# `adjudication-tsa-demo` audit trail's `ir_nodes[].payload` does NOT contain term trees

— only `id`, `kind`, `modality`, `polarity`. A walker scanning the audit trail for `quantity(...)` compounds will get 100% false-negatives. The full term JSON is emitted as a SEPARATE block in stdout under the marker `--- LLM-extracted IR (raw decompose_text output) ---` (when `ADJ_DEMO_AUDIT=1`). When the LLM goes through clarification reprompts the term tree also ends up embedded as a string inside `dialogue[*].response.text` and `dialogue[*].question_text`; in the no-clarification case those don't exist and the marker block is the only source. Always parse BOTH blocks.
