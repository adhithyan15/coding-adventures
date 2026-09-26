- Add the package-owned Qt `MosaicHost.cpp/.h` and Cairo-backed
  `venture-browser-qt` bridge, reusing the shared native host controller for
  live page rendering, navigation, scrolling, hover, link activation, and
  resize without duplicating Mosaic-authored chrome.
