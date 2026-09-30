### Added — every XAML app gets the platform library (UI87 §7.6)

- `MosaicPlatformEffects.cs` is written beside `MosaicRuntimeHost.cs` in every
  generated WinUI project, in the same namespace (the SDK's default glob
  compiles it, in the stub shell too).
- The runtime-backed `MainWindow.xaml.cs` installs
  `MosaicPlatformEffects.Install(this, appKinds: <new[] { ... } or null>)`
  right after `MosaicRuntimeHost.LoadRequired();` and the package's own
  `[host_effects]` handler, whose `kinds` become the router's app kinds. A
  window with no `LoadRequired` (the stub shell) and no declared handler is
  left as it is; a declared handler there is still refused.

