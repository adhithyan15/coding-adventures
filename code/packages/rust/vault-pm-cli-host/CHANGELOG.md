# Changelog

## Unreleased

- Add `ControllingTerminal::read_command_line`, a bounded echoed line reader for
  the foreground interactive shell. It uses the same controlling terminal every
  prompt uses, so a redirected standard input can supply neither a secret nor a
  command; it writes the fixed compile-time `vault-pm> ` prompt; and it reports
  a real end of input as `Ok(None)` rather than a failure so a session can end
  cleanly, while every other read failure stays closed. Line content is bounded
  at 1,024 bytes, must be valid UTF-8, and must contain no control characters.
- Factor the Unix controlling-terminal open into one helper shared by the
  secret, text, command-line, and reveal paths, and split the bounded line
  reader into an end-of-input-aware form. The secret reader's behaviour is
  unchanged: end of input still fails closed.

- Add a fixed hidden wipe-on-drop opaque-record payload prompt, collecting the
  whole canonical-CBOR payload as lowercase hexadecimal under the existing
  1,024-byte secret-line bound. It is hidden rather than echoed because an
  opaque record's schema is unknown, so no part of its payload can be shown to
  be non-secret.
- Add a fixed canonical URL-count prompt, repeated required URL input, and
  optional hidden wipe-on-drop login-notes input for complete login forms.
- Add fixed bounded TOTP label, issuer, algorithm, digits, and period prompts
  plus hidden wipe-on-drop Base32 seed input.
- Add fixed bounded database label, engine, host, port, database, and username
  prompts plus hidden wipe-on-drop password input.
- Add fixed bounded API-key label, service, scope, and expiry prompts plus a
  hidden wipe-on-drop token prompt.
- Add fixed bounded payment-card metadata prompts plus hidden PAN and CVV
  prompts, retaining controlling-terminal input and wipe-on-drop ownership.
- Add exact-`yes` controlling-terminal confirmation and direct quoted,
  control-escaped secret delivery without routing values through process
  stdout or stderr.
- Wipe the temporary escaped string and Windows UTF-16 console buffer after
  every terminal disclosure attempt.
- Add a bounded regular-file portable-artifact reader and reuse the fixed
  hidden import-passphrase prompt through CLI composition.
- Add fixed hidden portable-export passphrase and confirmation prompts with
  constant-time comparison.
- Add create-new durable encrypted-artifact persistence with Unix mode `0600`,
  no final-path replacement, and best-effort incomplete-file cleanup.
- Add fixed secure-note title and hidden body prompts, reusing the bounded,
  echo-restoring, wipe-on-drop controlling-terminal input boundary.

## 0.1.0

- Add fixed-prompt echoed UTF-8 login metadata collection with per-field
  bounds, control rejection, and optional username/URL handling.
- Add a fixed hidden login-password prompt and byte-accurate Windows UTF-8
  bounds.
- Add fixed-prompt, bounded secret collection from the controlling terminal on
  Unix and the attached console on Windows.
- Add echo restoration, oversized-line draining, zeroizing ownership, and
  constant-time new-passphrase confirmation.
- Add a stable, payload-free OS entropy adapter over `csprng`.
