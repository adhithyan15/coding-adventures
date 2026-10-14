---
category: Testing & coverage
---

# Use explicit UTF-8 and saved helpers for Windows native validation

Validator75181 exited1 at unchanged source96b26233 after the complete18-package
nonLANG suite passed928 tests/60targets including real WinUI acceptance.
Python Path.read_text() selected cp1252 for the LANG BUILD file and failed on
byte0x90 before any LANG command. The original receipt and log are immutable;
this is a harness failure, not a successful complete native run.

Continuation23389 pins the same source, target and lock hash, verifies the
same-head completed row/log hash and retains it without rerunning the unchanged
suite. It reads BUILD explicitly as UTF-8 and executes every remaining original
production/native command. Missing decoding declarations must be corrected in
the harness; never omit BUILD commands or reinterpret the failure as acceptance.

A first inline PowerShell/Python preparation attempt also failed parsing before
creating the continuation. A saved Python helper replaced it. Use apply_patch
and saved UTF-8 Python helpers for multiline code instead of mixed shell quoting.

The completed continuation retains its original failed receipt and same-head
row proof. Miri remains DEFERRED/PENDING.

Documentation checkpoint23482 later failed lesson validation because the draft
title did not derive the filename created by the lesson helper. Retitle the
lesson to match the helper's actual slug and rerun validation; keep the title
and filename contract intact. This checkpoint failure happened before commit
or issue publication and is separate from the completed native continuation.
