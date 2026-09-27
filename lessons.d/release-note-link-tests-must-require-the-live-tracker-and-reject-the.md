---
category: CI & GitHub Actions
---

# Release-note link tests must require the live tracker and reject the retired one

TaskApp's release-note test asserted that a limitation linked issue #13522, but
the issue later closed and #13977 became the live signing tracker. The test kept
passing while every generated release sent readers to retired work. When a
stable note template names a tracking issue, assert both that the live tracker
is present and that its retired predecessor is absent; then review those links
as part of the immutable release audit.
