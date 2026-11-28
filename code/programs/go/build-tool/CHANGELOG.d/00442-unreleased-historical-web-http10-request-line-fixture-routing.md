# Route HTTP/1.0 request-line vectors to Python

The build-tool registry now selects `python/historical-web` for the exact,
independently versioned HTTP/1.0 request-line vectors. Python fallback hashing
and Kiln `repo_inputs` include all three exact historical-web vector files;
other language ports remain keyed only by the original HWEB-00 vectors.
