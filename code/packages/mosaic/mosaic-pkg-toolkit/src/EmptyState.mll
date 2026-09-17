// EmptyState.mll — layout for the EmptyState.
//
//   Column [ empty-state ]
//     Text [ empty-state-title ]      (heading)
//     If (message)       Text [ empty-state-message ]
//     If (action-label)  HostButton [ empty-state-action ]
//
// The message is gated too: an empty Text still takes a line of height on
// some backends, which would leave a gap under a title-only empty state.

layout EmptyState {
  Column [ empty-state ] {
    Text [ empty-state-title ] (
      content : slot: title ,
      a11y-role : heading
    )
    If ( when: slot: message ) {
      Text [ empty-state-message ] (
        content : slot: message
      )
    }
    If ( when: slot: action-label ) {
      HostButton [ empty-state-action ] (
        label : slot: action-label ,
        onClick : emit: onAction
      )
    }
  }
}
