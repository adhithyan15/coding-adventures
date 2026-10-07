---
category: Testing & coverage
---

# Classify native consumer failures against an unchanged main baseline

The full CV04a post-merge five-consumer run exposed ten failures in `lang-aot/tests/lang_matrix.rs`: nested ALGOL array captures return NativeAot BackendRefused, and a JVM case raises VerifyError for excessive stack. The existing production BUILD explicitly excludes the unfiltered red matrix and its proven-column guard; issue #12032 already tracks these ALGOL cases. Preserve the actual failed full-suite result rather than claiming a green package. Reproduce representative exact cells on clean main with a separate target/worktree, compare failures, and validate the unchanged production test selection before attributing the defect or considering a scoped repair. Do not skip assertions or quietly narrow an originally requested passing claim.

Clean main `ebee9d93ba02` without CV04a reproduced exact single-cell 474:NativeAot (BackendRefused) and473:Jvm (Stack size too large), each one assertion/exit101, matching the candidate. Production-selection evidence confirms all49 explicit other targets and59 selected matrix assertions passed; the unfiltered package result remains failed. Retain representative-repro limits and the owned #12032 link, rather than claiming every failed cell was independently repeated. The latest three changed consumers pass479 tests/docs, and all five strict all-target lint checks pass.
