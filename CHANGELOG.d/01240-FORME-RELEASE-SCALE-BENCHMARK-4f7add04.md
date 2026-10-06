### Forme release-scale benchmark

- Split the final release-quality milestone into independently reviewable
  scale, browser-quality, public-API migration, and supported-platform gates.
- Exercise the live thirteen-stage blog DAG over exactly 1,000 generated pages
  through both HTML and terminal backends, then edit one page and prove that
  the persistent parser cache reuses the other 999 entries.
- Record clean and incremental elapsed times without an absolute host-speed
  threshold, and enforce the portable benchmark contract on Unix and Windows
  package build fronts.
- Fix the live named-input scheduler and static-site emitter after the scale
  corpus exposed circular sibling-stream backpressure at exactly two 64-value
  windows; a multi-window regression now pins concurrent fan-in progress.
- Bound concurrent input operations, retained page bytes and usage metadata,
  and benchmark child-process lifetime at the newly introduced boundaries.
- Isolate benchmark inputs, caches, reports, and emitted files in a fresh
  private project-temporary directory, then atomically replace the retained
  summary without following a pre-existing symlink.
