---
title: Capability-typed stages
date: 2026-05-08
excerpt: Why every Forme stage declares the capabilities it needs — and why the orchestrator refuses to grant them by default.
---

# Capability-typed stages

The most opinionated thing in the FM01 kernel isn't the type system —
it's the *capability system*. Every stage declares an array of strings
like `"storage:read"`, `"network:fetch:api.github.com"`, or
`"filesystem:write"` describing the dangerous things it intends to do.
The orchestrator then hands the stage a `StageContext` whose dangerous
APIs are wired to **denied wrappers** unless the matching capability
was declared.

```ts
import { defineStage } from "@coding-adventures/forme-stage";

export default defineStage({
  name: "@example/fetch-recent-prs",
  // ...
  capabilities: ["network:fetch:api.github.com"],
  async run(input, config, ctx) {
    // ctx.network.fetch is the REAL fetch, scoped to api.github.com
    // ctx.filesystem.writeFile throws CapabilityError — we didn't ask
    // ctx.shell.exec       throws CapabilityError — we didn't ask
  },
});
```

## Why this matters

Three properties fall out:

1. **Auditability.** Every package in the registry has a machine-
   readable manifest (`required_capabilities.json`) listing what it
   wants. A reviewer sees `["shell:*", "network:*"]` and asks the
   obvious question.
2. **Plug-in safety.** When the FM02 plugin host loads a third-party
   stage, it can refuse to grant capabilities the user didn't
   pre-approve. No more "this innocuous Markdown plugin is now
   exfiltrating your env vars."
3. **Reproducibility.** If a stage didn't declare `network:*`, it
   *cannot* hit the network. Your build is reproducible because the
   universe of side effects is bounded by the manifest.

## The pragmatic exception

Source and sink stages have a chicken-and-egg problem: `ctx.storage`
is supposed to be the orchestrator-provided `StorageApi`, but for
`forme-source-fs` to read disk, *something* has to be the
implementation. v0 resolves it by letting filesystem-adapter stages
declare the capability *and* bypass `ctx.filesystem` with direct
`node:fs/promises` calls — documented explicitly in each adapter's
README + `required_capabilities.json` + module header.

It's a real divergence and it's a real choice. The alternative —
bootstrapping a `FilesystemApi` implementation in the orchestrator
just to wire it back into source-fs — is more bureaucracy than it's
worth at v0 scale.

## What's next

A future post will walk through the capability *parser* and *matcher*
— how `network:fetch:*.github.com` matches `network:fetch:api.github.com`
without the substring-trap that bites every naive matcher
implementation.
