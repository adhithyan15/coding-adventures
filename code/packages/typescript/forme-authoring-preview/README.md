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
    publish(snapshot) {
      // snapshot.revision and snapshot.buildId identify the exact output.
      devServer.publish(snapshot);
    },
    publishFailure(failure) {
      devServer.publishFailure({
        message: failure.diagnostics.map(item => item.message).join("\n"),
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
  released.
- Successful outputs pass through FM07's `snapshotFromOutputs`, including its
  collision and portable-path checks.
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
