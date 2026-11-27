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

Elixir's external `.exs` adapter is run by the fixture-local Python build
driver; fixture commands travel through an environment variable so Windows
shell parsing cannot change their arguments. The Haskell adapter is a separate
Cabal executable in the standalone project rooted at the Haskell package. Both
invoke pure library APIs; neither turns the package into a shipped command-line
program.

The OCaml and Swift adapters are separate fixture-adjacent projects. A shared
`build_ocaml_swift.py` driver runs each package's component tests, builds the
external adapter in a temporary directory, and invokes `replay_cli.py` for all
nine cases. Their project and source files are selected as exact build-tool
inputs; the reusable packages do not read fixture JSON or emit serialized
traces.
