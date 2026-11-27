# Historical Unix fixture adapters

These adapters exist only to replay `file-read.json` from package `BUILD` and
`BUILD_windows` recipes. Adapters stay outside package APIs and distributions.

`build_native.py` runs the C, C++, Java, and Lua adapters. `build_dotnet.py`
builds external C# and F# adapters against their reusable library projects,
runs their component checks, and replays the same fixture. The replay helper
reads the canonical JSON and passes one command to the adapter at a time. The
libraries themselves do not read fixtures, print results, or start processes.

The Dart adapter is interpreted by Dart through `replay_cli.py`. Kotlin's
adapter is test-only Gradle source under this directory; it reads the canonical
fixture and invokes the reusable library API directly. Neither lane uses
`build_native.py` or ships a command-line executable.

Publishing separate user-facing command-line programs requires an explicit
capability review and hardware-key approval under D21 / 13-capability-security.
