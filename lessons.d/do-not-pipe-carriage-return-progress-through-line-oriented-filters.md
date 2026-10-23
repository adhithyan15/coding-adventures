---
category: Repo policy / workflow reminders
---

# Do not pipe carriage-return progress through line-oriented filters

The build tool redraws one progress line with carriage returns. Piping its
output through `awk` to keep only final report rows did not suppress those
updates because the stream was not newline-delimited; instead it accumulated a
huge logical line and produced noisy, truncated tool output. For noninteractive
validation, prefer a supported quiet or report-output option. If none exists,
run the tool normally and request a bounded final result, rather than treating
a newline-oriented filter as a progress renderer.
