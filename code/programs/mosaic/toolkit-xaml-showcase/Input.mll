// Input.mll — layout for the toolkit Input.
//
// Single HostInput wrapped with a `part_name: input` so the .msl can
// style the border, focus state, and padding consistently across
// every backend.

layout Input {
  HostInput [ input ] (
    value : slot: value ,
    placeholder : slot: placeholder ,
    read-only : slot: disabled ,
    onChange : emit: onChange ,
    onCommit : emit: onCommit
  )
}
