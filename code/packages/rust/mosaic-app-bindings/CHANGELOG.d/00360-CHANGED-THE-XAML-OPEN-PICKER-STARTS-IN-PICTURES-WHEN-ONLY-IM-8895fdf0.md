### Changed — the XAML open picker starts in Pictures when only images are accepted (UI59 §2)

The XAML platform library's `files.open` picker always started in the
Documents library. When every type the app accepts is an image (`accept`
of only `image/*` types), it now starts in the Pictures library and shows
thumbnails, as photo-picker's own handler did before UI87 §7.4 retired it.
Any other accept list, or none, still starts in Documents as a list.

`MosaicPlatformEffects.OnlyImages(extensions)` decides it from the picker's
extensions, using the same MIME table the filter comes from; the headless
conformance harness checks it (images, svg, mixed, none, a document), and
passes 224 checks.
