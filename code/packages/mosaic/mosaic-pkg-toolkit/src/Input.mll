// Input.mll — layout for the toolkit Input.
//
// Single HostInput wrapped with a `part_name: input` so the .msl can
// style the border, focus state, and padding consistently across
// every backend.
//
// `disabled` is bound to the real HostInput prop (UI58, #14786). It used
// to read `read-only : slot: disabled`, because until that landed there
// was no way to say *unavailable* -- only *not editable*. The two are
// genuinely different affordances:
//
//   read-only   takes keyboard focus, announces as editable-but-read-only,
//               and its text can be selected and copied.
//   disabled    is skipped by focus entirely and announces as unavailable.
//
// So the old binding produced an input that looked disabled -- the .msl
// dims it via `state disabled` -- and still accepted focus and a caret.
// Field.mll and InputGroup.mll carried the same approximation, and the
// other five toolkit controls never did, because HostButton, HostCheckbox,
// HostRadio and HostNumberInput always had a real `disabled`.

layout Input {
  HostInput [ input ] (
    value : slot: value ,
    placeholder : slot: placeholder ,
    disabled : slot: disabled ,
    onChange : emit: onChange ,
    onCommit : emit: onCommit
  )
}
