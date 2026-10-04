# Forme Authoring Publish

`@coding-adventures/forme-authoring-publish` is the capability-free FM09
coordinator that joins one exact durable authoring revision to FM08's reviewed
deployment boundary. The editor receives only closed target review data;
credentials, filesystem paths, network clients, and target configuration stay
inside the injected host adapter.

## Commit order

Publication deliberately crosses two commit points in this order:

```text
exact project snapshot -> product build -> FM08 parse + content preflight
  -> reviewed external target -> preparation retirement
  -> exact-revision authoring acknowledgement
```

The coordinator computes the manifest identity from canonical FM08 bytes and
accepts a target acknowledgement only when it names that identity. A known
pre-commit failure may be retried. A malformed or indeterminate target result,
post-deploy cleanup failure, or local acknowledgement failure poisons the
coordinator until the shell reloads and reconciles external state.

The target receives a manifest-restricted content store. Every `get` is
re-verified against the owned manifest entry, enumeration exposes only owned
digests, and retirement revokes later reads. Synchronous builder re-entry also
observes the action as active because adapter work starts only after the active
record is installed.

## Usage

```ts
import { createAuthoringPublisher } from "@coding-adventures/forme-authoring-publish";

const publisher = createAuthoringPublisher({
  builder: {
    async build(input, signal) {
      return host.buildExactProject(input.revision, input.project, signal);
    },
  },
  target: {
    review: {
      targetId: "github-pages",
      label: "GitHub Pages",
      destination: "example/site on gh-pages",
    },
    async publish(input, signal) {
      return host.publishReviewedTarget(input, signal);
    },
  },
});

console.log(publisher.target); // safe to render before explicit confirmation
const attempt = await publisher.publish(authoringSession);
await publisher.dispose();
```

The builder must run the real product pipeline and return a separately
releasable FM08 manifest/content preparation. Target rejection is a known
failure only under the adapter contract that rejection occurs before its
external commit point; uncertain post-commit work must resolve
`indeterminate`.

## Verification

`bash BUILD` compiles the package and runs adversarial exact-revision,
manifest/content, target acknowledgement, cancellation, cleanup, persistence,
and hostile-shape tests with at least 95% statement/line and 90% branch
coverage.
