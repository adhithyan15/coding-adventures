---
category: CI & GitHub Actions
---

# simctl launch --stderr does not capture an iOS app's stderr on CI's macOS runner, so gate on files the app writes

**What went wrong.** PR #16702's first push ran Journal through the new
`mosaic-ios-simulator-gate.sh`. That push launched the app with
`xcrun simctl launch --terminate-running-process --stderr=<file>` and grepped
the file for the host's "rejected persisted state" report. The run on
`macos-latest` showed the app was working: the refused `{}` was moved to
`mosaic-state.v1.json.corrupt` and fresh state was written. But the stderr
file never held the report, so the gate failed with "the host did not report
the state it refused" after a 23-minute macOS step.

**The fix.** Gate on what the app does to files in its data container, not
on its output. The quarantine file holding exactly `{}`, followed by fresh
state, already proves the host refused the seed. `--terminate-running-process`
stays, because it guarantees a cold start. Its cost is that a refusal whose
quarantine move fails silently is not caught. Trestle's iOS gate has the
same limit.

**Next time.** Do not make a simulator gate depend on capturing an iOS app's
stdout/stderr through `simctl launch` unless a CI run has already shown that
capture working. Prefer observable state, such as files under
`simctl get_app_container ... data` or the `launchctl` job list. Stream
`simctl spawn <udid> log` only when the app logs through os_log.

