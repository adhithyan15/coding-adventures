### Added — layout variants that would name one view twice are refused (UI48 §7.5)

- `discover_variants` refuses two `<Component>.<variant>.mll` files whose
  variant names differ only in letter case or in `-` / `_` (`touch` / `Touch`,
  `task-list` / `task_list` / `tasklist`), naming both files. SwiftUI and
  Compose would build one type name from both and fail with a redeclaration,
  and a case-insensitive filesystem would keep only one generated file.

