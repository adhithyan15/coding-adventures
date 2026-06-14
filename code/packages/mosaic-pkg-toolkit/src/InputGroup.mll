// InputGroup.mll — layout for InputGroup (v0.6).
//
//   Row [ input-group ]
//     If (prefix non-empty) Text [ input-group-prefix ]
//     HostInput [ input-group-field ]
//     If (suffix non-empty) Text [ input-group-suffix ]
//
// The two addons live in the same Row as the field; the .msl flat-
// styles them with shared borders so the whole assembly reads as
// one connected control.
//
// v0.6 always emits both `If` branches around the addons. The
// "should I show this addon" decision uses the slot truthiness —
// an empty string collapses the branch. A follow-up may add a
// dedicated `has-prefix: bool` slot if the truthiness shortcut
// proves awkward.

layout InputGroup {
  Row [ input-group ] {
    If ( when: slot: prefix ) {
      Text [ input-group-prefix ] (
        content : slot: prefix
      )
    }
    HostInput [ input-group-field ] (
      value : slot: value ,
      placeholder : slot: placeholder ,
      read-only : slot: disabled ,
      onChange : emit: onChange ,
      onCommit : emit: onCommit
    )
    If ( when: slot: suffix ) {
      Text [ input-group-suffix ] (
        content : slot: suffix
      )
    }
  }
}
