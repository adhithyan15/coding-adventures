# Bounded diagnostics for Flutter Android state-gate failures

## Purpose and scope

The Android emulator gate requires a freshly launched Flutter app to persist a
state file after its first environment event. When that file is missing, the
current gate proves only that the app failed to create a nonempty file at the
expected app-private path before the existing deadline. This diagnostic mode
adds evidence for the stages that can lead to that write; it does not change
the gate, repair behavior, or establish a cause by itself.

This is a temporary, opt-in investigation path for the generated Flutter phone
host and its Android gate. It does not change release builds, legacy host
behavior, state contents, test expectations, or timeouts.

## Enablement and output

Diagnostics are disabled unless the generated app is built with the explicit
`MOSAIC_NATIVE_DIAGNOSTICS=true` Dart define. The relevant CI build enables it
only for the Flutter phone artifacts used by the emulator gate. The host emits
fixed-format records to stderr, with a fixed stage name, a small enumerated
result, and elapsed microseconds where a stage has a measured duration. It
does not print environment values, event or snapshot data, exception messages,
filesystem paths, or arbitrary platform output. Each app process emits at most
24 records. Any inability to emit a record is ignored and must not change the
operation's result.

Required stages distinguish:

1. application-support-directory lookup: nonempty, empty, or thrown;
2. selected runtime route (including the statically bundled `@Native` route),
   first create-call entered, and startup response accepted or failed;
3. first environment-report decision (sent or a fixed not-ready/suppressed
   reason), actual native dispatch entered, and decoded answer returned,
   rejected, or thrown;
4. first snapshot result (null, present, or failed), then state-path
   availability, parent creation, temporary-file write, and replacement
   result. These markers describe calls/results only; they do not claim durable
   storage.

Instrumentation is observational. It must surround the actual FFI dispatch
and decoded answer, not infer a successful answer from a shell callback or a
non-null Flutter response. An environment report may be successfully
dispatched while the public method returns `null` because the app did not
change its visible revision or warning.

## Gate failure evidence

On the existing Flutter Trestle missing-state failure only, the gate may print
a bounded diagnostic summary containing:

- SHA-256 of the exact APK, its parsed application identifier, and SHA-256 of
  the generated project's resolved `pubspec.lock` when available;
- at most 24 recognized host diagnostic records, each line capped at 240
  characters;
- the state-path probe result as `absent`, `present_empty`,
  `present_nonempty`, or `probe_failed`.

The probe must use the app's existing `run-as` identity. A failed command,
timeout, or unreadable response is `probe_failed`; it must never be reported as
absence. It reports only a size class, not state contents. Diagnostic capture
has a dedicated wall-clock and byte budget. Failure to gather diagnostics
cannot mask the original gate failure, change its status, or skip emulator
cleanup. Other apps' successful runs do not cause Trestle records to be
published.

The existing state assertion, launches, restore/quarantine checks, delays,
and timeouts remain unchanged. APK identity and the file probe are contextual
evidence, not proof that the installed process loaded a particular native
symbol or received an environment answer.

## Validation and evidence limits

Source/generator tests must verify disabled-by-default output, fixed
diagnostic vocabulary, absence of values, and instrumentation around real
call/answer boundaries. Script tests must verify the bounded failure-only
capture and preservation of the original failing status when each diagnostic
probe fails. One reviewed native CI run may then capture the checkpoints for
the already reproduced failure. A passing run or missing marker is not by
itself a causal explanation.

This investigation does not prove general FFI correctness, package
installability, storage durability, or native/source/Warrant behavior. Any
production fix requires a specific checkpoint-supported cause and a focused
regression, while preserving the original gate.
