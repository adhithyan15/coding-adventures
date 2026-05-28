// NumberInput.mll — layout for the toolkit NumberInput (v0.4).
//
// Single HostNumberInput wrapped with `part_name: number-input` so
// the .msl can style border, focus, and padding consistently
// across every backend.
//
// `min`/`max`/`step` are deliberately omitted from the toolkit
// wrapper's interface — they're per-use compile-time literals
// rather than runtime slots, so authors who need range constraints
// should compose `HostNumberInput` directly. A follow-up could
// add toolkit `min`/`max` if a recurring pattern emerges.

layout NumberInput {
  HostNumberInput [ number-input ] (
    value : slot: value ,
    placeholder : slot: placeholder ,
    disabled : slot: disabled ,
    onChange : emit: onChange
  )
}
