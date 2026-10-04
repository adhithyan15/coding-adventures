# Forme Authoring Preview

`@coding-adventures/forme-authoring-preview` is the capability-free FM09
coordinator between a persisted authoring session and the real FM03/FM07 watch
pipeline. It does not render Content IR itself. Instead, a host materializes an
immutable project snapshot, returns the real typed pipeline for that isolated
input, and the coordinator executes the pipeline through `Orchestrator.watch`.

## Why this boundary exists

An editor changes faster than a build. If revision B arrives while revision A
is compiling, A must never replace B's preview merely because A finishes first.
The coordinator therefore attaches the durable storage revision to every
attempt, coalesces edits, stops superseded watch sessions, and publishes only a
still-current success. A failed or cancelled build does not touch the last good
artifact set.

The package has no declared capabilities. Filesystem isolation, configuration
loading, plugin discovery, cache placement, loopback serving, and cleanup are
host responsibilities supplied through narrow injected methods.

## Usage

```ts
import { createAuthoringPreview } from "@coding-adventures/forme-authoring-preview";

const preview = createAuthoringPreview({
  orchestrator,
  debounceMs: 100,
  materializer: {
    async prepare(input, signal) {
      // Copy input.project into a new isolated location, build the exact
      // pipeline for it, and make release idempotent.
      return host.preparePipeline(input, signal);
    },
  },
  publisher: {
    publish(snapshot, commitIfCurrent) {
      // snapshot.revision and snapshot.buildId identify the exact output.
      commitIfCurrent(() => { devServer.publish(snapshot); });
    },
    publishFailure(failure, commitIfCurrent) {
      commitIfCurrent(() => {
        devServer.publishFailure({
          message: failure.diagnostics.map(item => item.message).join("\n"),
        });
      });
    },
  },
});

const attempt = await preview.request(authoringSession);
console.log(attempt.outcome, attempt.revision);

await preview.dispose();
```

## Lifecycle guarantees

- The project handed to the materializer is a fresh validated, deeply frozen
  copy paired with the persisted `storageRevision`.
- At most the latest debounced request starts after an edit burst.
- Superseded preparation receives an aborted signal; superseded builds are
  stopped through the real watch session before their isolated input is
  released. Release is the host's final retirement boundary if stop fails.
- Successful outputs pass through FM07's `snapshotFromOutputs`, including its
  collision and portable-path checks, after a descriptor-only snapshot enforces
  file-count, portable case-fold/prefix collision, path, per-file, and
  aggregate-byte limits and copies all bytes through typed-array intrinsics.
- Async publishers may prepare work before committing, but every visible
  mutation must occur inside the supplied one-shot synchronous commit guard.
  A successful guarded mutation records the exact visible last-good revision
  immediately, even if the publisher then hangs and is superseded.
  The guard rejects stale generations and closes when the publisher returns or
  its abort signal fires, so a non-cooperative stale publisher cannot block the
  next revision.
- Diagnostics are capped at 64 closed records. Arbitrary fields, controls,
  bidi formatting, adapter error strings, and over-limit text do not escape.
- `state.lastGoodRevision` is distinct from `state.activeRevision`, so a shell
  can honestly say that stale output remains visible while the current draft
  is building or failed.
- Public attempts and state snapshots are frozen. Disposal is idempotent and
  later requests are rejected.

## Verification

The test suite covers success, exact revision attribution, pre-build
coalescing, preparation and active-build cancellation, malformed artifacts,
last-good retention, bounded hostile diagnostics, adapter failure redaction,
cleanup, and disposal. Coverage exceeds 95% statements and lines and 90%
branches.
