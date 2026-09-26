- **The live region is parented to `<body>`, not to `root`**, for the same
  reason: inside `root` the next render would destroy it and turn every later
  announcement into a silent no-op.

**Choosing pointer events is not by itself enough to make touch work.** A
direct-manipulation pointer receives *implicit pointer capture* on `pointerdown`,
so every later event in the gesture retargets to the element it began on:
`pointerenter` never fires for any other drop target, and `pointerup`'s target is
still the source card. The obvious implementation therefore resolves every touch
drop back to its source column and the card snaps back — the precise failure
pointer events were chosen to avoid. Hit testing goes through `elementFromPoint`
instead, which reports what is actually under the finger, and the draggable
carries `touch-action: none` so the browser does not claim the gesture as a
scroll. Capture is embraced rather than fought: it is what makes a release
*outside* the component or the window still deliver `pointerup` to us.

Other behaviours worth naming, each with a test:

