## 0.3.0

- Add `Suite::run_recorded` and `Outcome`, returning every test's verdict as the runner reported it; `Suite::run` is now a thin wrapper (behaviour unchanged).
- Add `run::main_recorded`, which hands the verdicts to a `finish` closure before returning the exit status; `run::main` is unchanged.
- Add the `manifest` module: a dependency-free, deterministic JSON writer (`Json`, `output_path`, `write`) used to emit evidence manifests (VER04h).
