---
category: CI & GitHub Actions
---

# A workflow contract test can pin incidental step inputs, so changing a CI checkout means grepping code/scripts for the old value

PR #19226 changed the `build` job checkout from `fetch-depth: 0` to `fetch-depth: 1`
and the "Repo-wide metadata contracts" job failed with
`ocaml_toolchain_lock.ContractError: generic CI checkout must fetch history without
persisting credentials` (`code/scripts/ocaml_toolchain_lock.py`), plus
`test_cli_validate_repository_passes`.

Cause: the contract pinned the whole `with:` mapping of the build checkout to
`{"fetch-depth": "0", "persist-credentials": "false"}`. The security intent was the pinned
checkout SHA and `persist-credentials: false`; `fetch-depth: 0` was only the value that
happened to be there when the pin was written (#14393).

Fix: the contract now accepts `{"fetch-depth": "1", "persist-credentials": "false"}`.

Next time: before editing `ci.yml`, grep `code/scripts` (and `code/specs`) for the exact
key and value you are changing, and run the contracts job's unittest and validate
commands locally.
