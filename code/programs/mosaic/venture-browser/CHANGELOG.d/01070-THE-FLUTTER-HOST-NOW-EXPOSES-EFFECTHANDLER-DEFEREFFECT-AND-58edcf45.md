- The Flutter host now exposes `effectHandler`, `deferEffect` and
  `completeEffect`. Every generated Flutter project installs Mosaic's platform
  library (UI87 §7.7), which reads those three members, and this host replaces
  the generated one, so `flutter analyze lib` failed on all four uses.
  Venture's shared session answers its own effects in the native library and
  no Await effect reaches Dart. So the host behaves like the generated host
  with no runtime: the handler is stored and never called, `deferEffect` is
  false, and `completeEffect` is null.

