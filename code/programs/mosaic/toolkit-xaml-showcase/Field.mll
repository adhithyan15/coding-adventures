// Field.mll — layout for the Field component.
//
//   Column [ field ]
//     Text [ field-label ] ( content: slot: label )
//     HostInput [ field-input ] (
//       value: slot: value ,
//       placeholder: slot: placeholder ,
//       read-only: slot: disabled ,
//       onChange: emit: onChange ,
//       onCommit: emit: onCommit ,
//     )
//     If ( when: slot: error ) {
//       Text [ field-error ] ( content: slot: error )
//     } Else {
//       Text [ field-help ] ( content: slot: help )
//     }
//
// Both the help and error texts share the bottom slot via If/Else.
// When `error: slot: e` evaluates truthy (non-empty per UI29 expr
// truthiness), the error renders; otherwise the help renders. The
// .msl styles the two differently — field-error gets the danger
// color, field-help gets the neutral muted color.

layout Field {
  Column [ field ] {
    Text [ field-label ] (
      content : slot: label
    )
    HostInput [ field-input ] (
      value : slot: value ,
      placeholder : slot: placeholder ,
      read-only : slot: disabled ,
      onChange : emit: onChange ,
      onCommit : emit: onCommit
    )
    If ( when: slot: error ) {
      Text [ field-error ] (
        content : slot: error
      )
    }
    Else {
      Text [ field-help ] (
        content : slot: help
      )
    }
  }
}
