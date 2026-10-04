## 2026-10-04 (an Android drag no target wanted still reports its end)

- **`MosaicDragEndWatcher` (UI89 §3.5).** `ANDROID_PLATFORM_KT` gains `MosaicDragEndWatcher(content)`: one drop
  target over the whole window, interested in every Mosaic drag (its
  `localState` is Mosaic's), accepting none, whose `onEnded` calls
  `mosaicDragEnded` (UI89 §3.5). Android tells a drag's end only to the
  targets that were interested in it, so a drag no component wanted -- let go
  over empty space, or over targets of another kind -- never reached its
  source, which stayed mid-drag. The components' own targets still decide
  every drop, and the source's completion still runs once.
- The wrapper passes the window's constraints through
  (`propagateMinConstraints`), so content lays out as before.
- The platform parity test now expects Android's extra, activity-only
  function beside the four shared seams.
