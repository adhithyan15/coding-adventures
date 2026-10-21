## 2026-10-19 (A slot-bound number placeholder is the TextField title)

`HostNumberInput ( placeholder : slot: x )` was dropped (EM-14): the number
field read `placeholder` with a string-literal-only lookup, so
`mosaic-pkg-toolkit`'s `NumberInput` came out as `TextField("", value: …,
format: .number)`, and the degradation report said nothing. It now takes its
title from `swift_placeholder`, as `HostInput` does: a literal stays a
`LocalizedStringKey` literal, and a slot, `For` binding or expression is
`String(describing: …)`, which `TextField(_:value:format:)` takes through its
`StringProtocol` overload. A name that is not a Swift identifier is an
`UnsafeSlotName` error.
