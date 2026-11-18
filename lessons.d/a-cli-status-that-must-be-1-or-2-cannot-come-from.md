---
category: Rust
---

# A CLI status that must be 1 or 2 cannot come from main returning Err: the status is chosen by where the error arose, not by its code

`warrant-corpus-gen` exited 1 for a refusal and for a `--check` drift alike, while its oracle and VER04r use 1 = differs and 2 = refused (CV2-F2). Two traps: a `main` that returns `Err` always exits 1, so status 2 needs `std::process::exit` (declared `proc:signal`; `ExitCode` would read as `proc:exec` to the capability analyzer), and the `GenError` code `drift` is raised both by `--check` and by the bindings cross-check, which is a refusal, so mapping by code alone gets one of them wrong. Map at the call site (`Failure::Drift` only where the comparison runs), and pin every category with a binary-level test. A capability `target` may contain only `[a-zA-Z0-9_-./*]`.
