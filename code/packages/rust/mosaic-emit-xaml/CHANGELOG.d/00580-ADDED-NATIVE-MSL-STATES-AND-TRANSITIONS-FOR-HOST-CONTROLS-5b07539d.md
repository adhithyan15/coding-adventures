### Added - Native MSL states and transitions for Host controls

`HostInput`, `HostButton`, `HostCheckbox`, `HostRadio`, `HostLink`, and
`HostNumberInput` now consume structured MSL state and transition IR.
Top-level `state-when-*` predicates become one-way WinUI `StateTrigger`
bindings, state properties become `VisualState` setters, and MSL durations
and easing curves become native `VisualTransition` values.

Each transitioned property is emitted in a separate `VisualStateGroup`.
This preserves MSL's property-scoped motion contract instead of letting one
transition duration animate every property changed by a state. Part-level
transitions apply in both directions, while a state-local transition
overrides the curve on entry. Multiple active states retain React/SwiftUI
precedence: the last `state-when-*` declaration wins. Stateful components use
a transparent first-child `Grid`, which is the placement WinUI requires for
automatic `StateTrigger` evaluation.

Supported easing lowerings are `linear`, `ease`, `ease-in`, `ease-out`, and
`ease-in-out`. WinUI XAML has no arbitrary cubic-bezier
`EasingFunctionBase`, so `cubic-bezier(...)` currently uses the closest
native `CubicEase` curve; an exact Composition-API lowering remains a
follow-up. Template-local Host controls inside `For` keep their VisualStates
inside the DataTemplate namescope so their triggers and targets remain
row-local.

