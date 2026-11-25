## Unreleased — microbenchmark pair fixture consumers

- Register the exact shared `microbenchmark-pairs-v1.json` fixture for Go and Python `microbenchmark` and `rank-statistics`. The file is exported as a file input for cache keys, without a recursive tree or broad path match.
- Add selection coverage for the four consumers, language filtering, missing-consumer failure, deleted-path selection, and deterministic near-miss paths.
