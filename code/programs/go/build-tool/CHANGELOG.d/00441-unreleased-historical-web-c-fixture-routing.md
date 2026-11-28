# Route historical-web vectors to the C package

The build-tool registry now selects `c/historical-web` when the shared HWEB-00
vectors change. The exact vector path is also used for fallback package hashing
and Kiln `repo_inputs`, with the C harness prerequisites represented in focused
graph tests.
