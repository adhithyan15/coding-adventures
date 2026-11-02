### Fix VisiCalc XAML project configuration and cross-platform build scripts

- Fixed XML comment syntax error on line 2 in `code/programs/csharp/visicalc-xaml/VisiCalc.csproj` (removed disallowed `--` within comment).
- Removed duplicate default `Page` items and added `UseRidGraph`, PRI packaging suppression, and runtime DLL flattening to `VisiCalc.csproj`.
- Added Windows `.exe` executable detection in `build.sh` across all VisiCalc demo targets (`visicalc-xaml`, `visicalc-swiftui`, `visicalc-flutter`, `visicalc-qt`, `visicalc-compose`, `visicalc-android`, `visicalc-html`, `visicalc-webcomp`).
- Added Windows/MSYS/Cygwin dynamic library path detection (`spreadsheet_capi.dll`) in `code/programs/csharp/visicalc-xaml/scripts/verify.sh`.
