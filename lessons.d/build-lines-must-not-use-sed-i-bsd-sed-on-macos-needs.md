---
category: Cross-platform & Windows BUILD_windows
---

# BUILD lines must not use sed -i; BSD sed on macOS needs a suffix argument

GNU sed accepts `sed -i 'expr' file`, but BSD sed (macOS) takes the first
argument after `-i` as the backup suffix. So `sed -i 's/^0 /9 /' file` on macOS
reads the expression as a suffix and fails with `sed: 1: "...": invalid command
code f`. The BUILD gate then fails only on the macOS leg, and Linux CI stays
green, so the bug looks platform-specific and is easy to miss locally.

Seen in PR #19577, `code/packages/rust/warrant-corpus-gen/BUILD` line 12, where
a sabotage check used `sed -i` to corrupt a generated chunk file.

Fix: write to a temporary file and move it back. Keep the temporary file
outside the directory the gate checks, so extras checks do not see it:

    sed 's/^0 /9 /' $d/chunks/eq_agrees.txt > $d/eq.tmp && mv $d/eq.tmp $d/chunks/eq_agrees.txt

Do not use `sed -i ''` either: that form is BSD-only and breaks GNU sed. The
redirect-and-move form runs the same on both. Before pushing a BUILD change,
grep the BUILD files for `sed -i` and for other flags that differ between GNU
and BSD (`sed -r`, `readlink -f`, `date -d`, `xargs -r`).
