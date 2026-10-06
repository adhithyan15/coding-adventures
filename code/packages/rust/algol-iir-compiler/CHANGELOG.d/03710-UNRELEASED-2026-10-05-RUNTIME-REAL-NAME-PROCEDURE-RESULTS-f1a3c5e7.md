## 0.371.0 - 2026-10-05 - Runtime-real name-formal procedure results

- Preserve formatter provenance on a real procedure result when finite
  call-by-name specialisation binds every real scalar name formal to a proven
  runtime-real actual. This reuses the existing specialised sibling and does
  not add a runtime thunk ABI.
