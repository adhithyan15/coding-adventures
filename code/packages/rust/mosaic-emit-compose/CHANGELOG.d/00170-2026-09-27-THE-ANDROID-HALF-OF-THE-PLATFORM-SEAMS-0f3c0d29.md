## 2026-09-27 (the Android half of the platform seams)

- **`ANDROID_PLATFORM_KT` (UI89 §3.4, §3.5).** The Android `MosaicPlatform.kt`: the same functions as the desktop one over Android's `DragEvent`. Android gives a drop target the `clipData` only with the drop, so the text also rides in `localState`, which every event of an in-app drag carries -- a target accepts, enters and hovers as on desktop. Android never tells a drag source its drag ended, so the transfer's `onCompleted` runs, once, when a target reports the end.
- **Components no longer import `DragAndDropTransferAction` or `DragAndDropTransferData`.** Both were unused since the transfer moved into `MosaicPlatform.kt`, and the first is desktop-only: the Android build failed on the import alone. A test now refuses desktop-only names in a shared component.
- **A fourth seam, `mosaicDragEnded(event)`.** Every drop target's `onEnded` now calls it. The desktop half does nothing (AWT reports the end through `onTransferCompleted`); the Android half runs the source's completion. A test pins that both halves define the same functions.

