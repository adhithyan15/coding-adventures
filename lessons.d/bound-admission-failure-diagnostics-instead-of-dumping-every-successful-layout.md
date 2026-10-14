---
category: Rust
---

# Bound admission-failure diagnostics instead of dumping every successful layout

An adversarial test correctly rejected80 invalid admissions, but formatting each
unexpected successful layout produced a huge diagnostic dump. The original red
receipt remains preserved. Classify outcomes as accepted, panicked or a concise
typed error, retaining field/value bits and case identity. Keep every assertion
and cell; bound the explanation rather than reducing coverage.
