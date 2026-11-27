# Historical Unix fixture adapters

These adapters exist only to replay `file-read.json` from package `BUILD` and
`BUILD_windows` recipes. Each build compiles or copies an adapter to a temporary
build location; none is part of a language package API or distribution.

`build_native.py` runs the C, C++, Java, and Lua adapters. `build_dotnet.py`
builds external C# and F# adapters against their reusable library projects,
runs their component checks, and replays the same fixture. The replay helper
reads the canonical JSON and passes one command to the adapter at a time. The
libraries themselves do not read fixtures, print results, or start processes.

Publishing separate user-facing command-line programs requires an explicit
capability review and hardware-key approval under D21 / 13-capability-security.
