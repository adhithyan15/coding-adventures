### Added -- the XAML host answers effects (UI47 §5.4 step 4, last of five)

The fifth and final host template. `MosaicRuntimeHost` gains `EffectHandler`,
`CompleteEffect`, `DeferEffect` and the same bounded settle loop; the emitted
protocol version moves to `EFFECT_PROTOCOL_VERSION`. **Every native backend now
answers effects**, which is what step 5 was waiting on -- Engram's Anki import
would otherwise have worked on some backends and silently broken on the rest.

Three things specific to this host:

- **`NativeLibrary.GetExport` throws on a missing symbol**, so the seventh
  resolves through `TryGetExport` into a nullable delegate. Without that, a
  protocol-1 runtime would stop loading at all -- a worse failure than the one
  it prevents, since the application works right up until an effect arrives.
- **`JsonElement` is immutable**, so `WithEffects` and `ReportingError` rebuild
  the update through a dictionary rather than mutating one, the way the other
  four do.
- **The delegate had to be renamed.** A nested `delegate CompleteEffect` and a
  static method `CompleteEffect` cannot coexist in one class; the delegate is
  now `CompleteEffectNative`, matching Flutter's naming.

