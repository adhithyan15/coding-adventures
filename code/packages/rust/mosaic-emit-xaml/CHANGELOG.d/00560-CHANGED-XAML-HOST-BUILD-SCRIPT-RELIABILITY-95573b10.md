### Changed - XAML host build script reliability

Generated `build.ps1` drivers now resolve `dotnet` from PATH or the standard
`Program Files\dotnet\dotnet.exe` location before building, and fail with a
non-zero exit code when the tool is unavailable. The `-Run` path also reports a
missing executable or non-zero app exit instead of leaving the script looking
green after a failed launch.

The nested Windows Rust workspace config now uses the Rust-bundled `rust-lld`
linker, matching the repo root and avoiding accidental resolution of Git/MSYS
`link.exe` without requiring Visual Studio's `lld-link.exe` in local dev shells.

