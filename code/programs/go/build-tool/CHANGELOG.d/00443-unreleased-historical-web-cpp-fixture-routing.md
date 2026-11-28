# Route historical-web vectors to the C++ package

The build-tool registry now selects `cpp/historical-web` when the shared HWEB-00
vectors change. The exact vector path is also used for fallback package hashing
and Kiln `repo_inputs`; focused graph tests include its bounded-json and
test-bootstrap dependency closure.
