### Fixed — `HostInput.disabled` bound a `Not()` method it never declared (#14793)

UI58 (#14786) wrote the polarity flip into the attribute by hand:

```rust
attrs.push_str(&format!(" IsEnabled=\"{{x:Bind Not({pascal}), Mode=OneWay}}\""));
```

`Not` is a method on the partial class, registered through `add_helper`.
Emitting the call without registering the method produced XAML binding to
something the code-behind never declares, so the generated C# did not compile.
It was also wrong inside a `For` template, where WinUI's typed DataTemplate
compiler rejects a bare function binding and a row-VM computed property is
required.

Both are handled by `disabled_slot_xbind_path`, which every other control that
flips this polarity already used — `HostButton`, `HostCheckbox`, `HostRadio`.
`HostInput` is now routed through it too.

The UI58 test asserted the emitted attribute string, which was **correct**:
the working implementation emits the identical XAML. The whole difference lives
in the code-behind, so emitter tests passed and the first thing to notice was a
WinUI shell build on CI (#14791). The test now asserts the helper as well, and
was falsified before being trusted — restoring the hand-rolled form fails it
naming the cause.

Checked for the same class on the other two backends where UI58 emits a helper
call: compose emits `_mosaicTruthy` unconditionally, and flutter emits it when
the tree contains the call, which the `disabled` lowering does. Only XAML
registers helpers on demand, and only `HostInput` skipped it.

