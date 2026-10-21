---
category: Mosaic compiler pipeline
---

# Deferred effect completion must refresh host status as well as properties

Native Save and cancellation refreshed workbook properties after their deferred
effect completed, but the generated window discarded ApplyRequiredProps' status.
The diagnostic therefore still described the earlier pending effect. Assign the
returned status during the same refresh, preserving genuine runtime warnings and
the existing exception path. Test deferred completion separately from direct
dispatch: a later viewport event can hide this stale-status bug by updating the
diagnostic through a different path.

The first test run also caught the layout-switching shell's exact-string rewrite
of this callback. Update that rewrite and its lifecycle assertion together with
the plain shell; a focused plain-window test does not exercise variant mounting.

The package-artifact builder also asserts the generated callback across policy
boundaries. Run its suite alongside emitter tests: emitter-only validation missed
this downstream exact-string assertion and failed macOS CI.
