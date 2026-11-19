# Bounded diagnostics for Flutter Android state-gate failures

## Purpose and scope

The Android emulator gate requires a freshly launched Flutter app to persist a
state file after its first environment event. When its `run-as ... test -s`
does not succeed by the existing deadline, the gate does not establish whether
the file was absent, empty, unreadable, or inaccessible. This diagnostic mode
adds evidence for the stages that can lead to that write; it does not change
the gate, repair behavior, or establish a cause by itself.

This is a temporary, opt-in investigation path for the generated Flutter phone
host and its Android gate. It does not change release builds, legacy host
behavior, state contents, test expectations, or timeouts.

## Enablement and output

Diagnostics are disabled unless the generated app is built with the explicit
`MOSAIC_NATIVE_DIAGNOSTICS=true` Dart define. The relevant CI build enables it
only for the Flutter phone artifacts used by the emulator gate. The host emits
fixed-format `MOSAIC_NATIVE_DIAG v=1 stage=… result=<enum> elapsed_us=<int>`
records through Dart `print`, which Flutter routes to Android logcat. It does
not print environment values, event or snapshot data, exception messages,
filesystem paths, or arbitrary platform output. Each fixed stage/result pair
is emitted once, so an initial skipped environment report does not hide a
later successful attempt. Each app process emits at most 24 records; the final
slot is reserved for `capture_clipped` if the limit is reached. Any inability
to emit a record is ignored and must not change the operation's result.

Required stages distinguish:

1. Dart `main()` entry before Flutter binding initialization, then the
   application-support-directory lookup attempt immediately before its
   existing await; the lookup result is nonempty, empty, or thrown. The
   attempt marker does not establish that the platform plugin replied;
2. whether the phone runtime was bundled (this does not claim the selected
   fallback route), Dart create-call
   attempt, raw ABI-call return/throw, and startup response
   accepted/rejected;
3. environment-report decisions (fixed not-ready/suppressed reason or send),
   Dart dispatch attempt, raw ABI-call return/throw, and answer decoded as
   success/error or rejected by a nonzero native status; these stages allow an
   initial skip and a later send;
4. persistence decisions in actual order: skipped-already-saved, disabled,
   or unavailable path; then the snapshot call's null/present/error result,
   parent-directory creation, temporary-file write, and replacement return.
   These markers describe calls/results only; they do not claim durable
   storage.

Instrumentation is observational. A Dart marker before an FFI call means
only that the Dart call site was reached; it cannot prove Rust entered the
ABI. The raw call return and decoded response are separate markers. A non-null
Flutter response is not a substitute for decoded native answer. An
environment report may be successfully dispatched while the public method
returns `null` because the app did not change its visible revision or warning.
A nonzero native status is recorded separately from a decode rejection; only a
zero-status buffer that cannot be decoded is a decode rejection.

## Gate failure evidence

On the existing Flutter Trestle missing-state failure only, the gate may print
a new bounded diagnostic summary containing:

- SHA-256 of the exact APK, its parsed application identifier, and SHA-256 of
  the generated project's resolved `pubspec.lock` when available;
- at most 24 recognized host diagnostic records, each line capped at 240
  characters;
- the state-path probe result as `absent`, `present_empty`,
  `present_nonempty`, or `probe_failed`.

The newly added summary is capped below 64 KiB in total; its raw log is capped
at 49,152 bytes before exact PID, envelope, and vocabulary filtering. The
existing 200-line failure log printed by `fail()` remains separate and is not
included in this summary cap.

The probe must use the app's existing `run-as` identity. A failed command,
timeout, or unreadable response is `probe_failed`; it must never be reported as
absence. It reports only a size class, not state contents. The failure-only
capture runs in one child process under a five-second timeout plus a
one-second kill-after bound, filters host markers to the current Trestle
PID/launch window, and reports whether log capture itself
was complete, clipped, timed out, or failed. APK and lock identifiers are
fixed-size hashes and parsed package metadata. Failure to gather diagnostics
cannot mask the original gate failure, change its status, or skip emulator
cleanup. Other apps' successful runs do not cause Trestle records to be
published.

The existing state assertion, launches, restore/quarantine checks, delays,
and timeouts remain unchanged. APK identity and the file probe are contextual
evidence, not proof that the installed process loaded a particular native
symbol or received an environment answer.

## Validation and evidence limits

Source/generator tests must verify disabled-by-default output, fixed
diagnostic vocabulary, absence of values, ordered persistence skip behavior,
and instrumentation around the Dart-call/ABI-return/answer boundaries. The
Android build command must pass the explicit Dart define. Script tests must
verify PID-scoped bounded failure-only capture and preservation of the
original failing status when the state, PID, log, hash, and APK-metadata
probes fail individually, including an outer-timeout control. One reviewed native
CI run may then capture the checkpoints for the already reproduced failure.
A passing run or missing marker is not by itself a causal explanation.

This investigation does not prove general FFI correctness, package
installability, storage durability, or native/source/Warrant behavior. Any
production fix requires a specific checkpoint-supported cause and a focused
regression, while preserving the original gate.
