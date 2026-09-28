### Fixed — the Compose release build broke on its own migration

Not predicted from the code: **#15057 went red on "Build the Compose Desktop
app"**, and reproducing it locally gave

```
error: no jar in .../binaries/main/app contains MosaicHost; cannot place the engine
```

The Compose arm locates the jar containing `MosaicHost.class` — Engram's *own*
Kotlin host — and places `engram_capi` beside it, because that host resolved the
engine from its jar's directory at runtime. The migration deletes that class, so
the `find` returns nothing and a perfectly good build fails.

Same shape as Qt's, one backend along: the release script asserting something
the architecture no longer has.

A migrated Compose needs no placement at all. `--runtime-library` hands Gradle
the runtime as a project resource and `createDistributable` carries it in —
measured, not assumed: the distribution holds
`engram_app.app/Contents/app/resources/libmosaic_app.dylib`.

**The verification is not skipped with the placement.** Dropping both would
trade a loud failure for a silent one, which is the trade this script exists to
refuse — so the migrated path asserts the runtime is in the distribution and
exports its six `mosaic_app_*` symbols.

Both paths were run end to end on a real Gradle build: with the Compose
`[host_assets]` line removed the way #15057 removes it, the build now succeeds
and reports the runtime shipped; with the line present, it still places
`engram_capi` beside the host jar exactly as before.

**Ordering:** #15057 needs this to go green, so it should land after this
change or rebase onto it.

