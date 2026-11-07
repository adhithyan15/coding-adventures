### Harden local demo build scripts and cross-platform engine vendoring

- Added cargo bin directory detection (`~/.cargo/bin`) across local demo build and wasm scripts (`visicalc`, `visicalc-html`, `visicalc-webcomp`, `visicalc-deno`, `visicalc-xaml`, `visicalc-flutter`, `visicalc-qt`, `visicalc-swiftui`, `visicalc-compose`, `task-app`, `task-wasm`, and `engram-wasm`).
- Added Windows/MSYS/Cygwin dynamic library vendoring (`spreadsheet_capi.dll`) and static library vendoring (`spreadsheet_capi.lib`) in `visicalc-xaml`, `visicalc-flutter`, `visicalc-compose`, and `visicalc-qt` build/verify scripts.
- Added cross-platform Python binary discovery in `code/programs/typescript/visicalc-html/scripts/build.sh`.
- Updated `code/programs/dart/visicalc-flutter/.gitignore` to ignore the generated touch variant `lib/generated/formula_bar_touch.dart`.
- Refreshed auto-generated Mosaic component outputs for Flutter, SwiftUI, and Compose targets.
