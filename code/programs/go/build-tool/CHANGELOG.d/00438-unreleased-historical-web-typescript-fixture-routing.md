# Route historical-web vectors to the TypeScript package

The build-tool registry now selects `typescript/historical-web` when the shared
HWEB-00 vectors change. The exact vector path is also used for fallback package
hashing and Kiln `repo_inputs`, with lookalike and sibling paths excluded.
