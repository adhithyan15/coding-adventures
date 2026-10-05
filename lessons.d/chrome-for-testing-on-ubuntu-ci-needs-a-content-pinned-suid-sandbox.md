---
category: Supply chain & CI pinning
---

# Chrome for Testing on Ubuntu CI needs a content-pinned SUID sandbox helper when AppArmor blocks user namespaces

Ubuntu runners can allow Chromium to start locally yet reject its unprivileged
user-namespace sandbox in CI because AppArmor applies a stricter policy. A
launcher that only watches Chrome's log then reports a misleading timeout even
though Chrome exited immediately with a sandbox error.

Do not work around that failure with `--no-sandbox`, a globally relaxed sysctl,
or an unverified system package. For Chrome for Testing, download the exact
browser archive already selected by the workflow, verify the archive and the
embedded `chrome_sandbox` helper against committed SHA-256 digests, install the
helper root-owned with mode `4755`, and point `CHROME_DEVEL_SANDBOX` at it. Keep
the launcher bounded and make it surface early process exit plus the complete
startup log so the next policy failure is diagnostic rather than a timeout.

When upgrading the pinned Chrome build, recompute both digests from the official
artifact and update them together. This preserves the browser sandbox without
weakening the runner and makes the privileged helper a reviewable supply-chain
input.
