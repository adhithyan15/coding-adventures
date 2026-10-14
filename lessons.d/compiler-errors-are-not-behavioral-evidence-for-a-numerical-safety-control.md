---
category: Testing & coverage
---

# Compiler errors are not behavioral evidence for a numerical safety control

The first spacing control run at sourcee5b9163a exited before any test executed:
the test used `ParseError.column`, but the actual type exposes `col`. Rust E0609
is a compilation failure, not evidence that finite spacing creates invalid
geometry. Preserve that source-specific receipt and its zero-target count.

Read the exact error type, correct only the test field and retain the full error
message/location assertions. Run the control again on a new committed source;
only a real executed assertion can establish the numerical failure. Do not
reinterpret a compile error, zero-test filter or active process as a behavioral
result or native acceptance.
