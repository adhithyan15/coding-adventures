# Route historical-web vectors to the F# package

The build-tool registry now selects `fsharp/historical-web` when the shared
HWEB-00 vectors change. The exact vector path is also used for fallback package
hashing and Kiln `repo_inputs`; focused graph tests include the F# test-bootstrap
dependency and its C# test-bootstrap prerequisite.
