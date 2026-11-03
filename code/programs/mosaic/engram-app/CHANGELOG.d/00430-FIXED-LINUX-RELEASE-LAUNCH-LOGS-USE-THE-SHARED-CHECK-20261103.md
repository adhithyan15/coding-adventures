### Fixed — the Linux release launches use the shared launch-log check

The Compose and Flutter Linux gates in `release-engram.yml` had their own
inline launch-log check. It wrapped the log in `::stop-commands::` correctly,
but `if grep -F "Mosaic Rust runtime unavailable" "$log"` then echoed any
matching line a second time, outside the quoted region. A line like
`::add-mask::... Mosaic Rust runtime unavailable` printed by the app would
have been read as a workflow command.

Both gates now call `code/scripts/mosaic-release-check-log.sh`, the one copy
of the rule. It shows the log as plain text, fails quietly (`grep -q`) on the
runtime-unavailable report, and fails if the log is missing. It uses a 128-bit
token, and a last line without a newline cannot swallow the resume marker.
TaskApp's Linux lanes and the macOS launcher use the same script.
`release-engram.yml` reruns when it or its test changes.
