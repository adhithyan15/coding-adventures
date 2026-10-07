---
category: CI & GitHub Actions
---

# A workflow guard test that no CI job runs guards nothing, and the workflows drift right past it

`code/scripts/tests/test_apt_install.py` has asserted since #14212 that no
workflow calls `apt-get update` directly -- everything goes through
`code/scripts/ci/apt-install.sh`. Two deploy workflows (`deploy-mosaic-docs.yml`,
`deploy-task-app.yml`) were later added with exactly that construct, and the
test went red on `main` without anybody seeing it: it was not in the "Verify
repo-wide metadata contracts" command list in `ci.yml`, or anywhere else CI
runs. A green CI said nothing about it.

It surfaced only while fixing #12163 / #12181 (an apt mirror stall that burned
a full six-hour job because no apt step had `timeout-minutes`): running the
test locally failed before any change was made.

**Fix:** the test is now in the metadata-contracts list, which has no `if:` and
so runs on every pull request. The two deploy workflows go through the wrapper.

**Do differently:** when you add a test under `code/scripts/tests/` that
inspects `.github/workflows/`, add its `python3 -m unittest discover ... -p
'<file>'` line to the contracts job in the same change, and check that it is
there before trusting that it guards anything. For any test file,
`grep -n "<file>" .github/workflows/*.yml` coming back empty means nothing runs it.
