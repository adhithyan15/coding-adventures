// Alert.mll — layout for the Alert component.
//
// Box[alert] wraps a horizontal Row with the message Text on the
// leading edge and (when dismissible) a HostButton on the trailing
// edge. Pure kernel-primitive composition:
//
//   Box [alert]
//     Row
//       Text [message]
//       If (dismissible) HostButton [close-btn]
//
// The variant slot doesn't appear in the .mll — it routes through
// the part-level styling in Alert.{theme}.msl. Same convention as
// Button.mll (see Button.mll's commentary).

layout Alert {
  Box [ alert ] {
    Row [ alert-row ] {
      Text [ message ] (
        content : slot: message
      )
      If ( when: slot: dismissible ) {
        HostButton [ close-btn ] (
          label : "x" ,
          onClick : emit: onClose
        )
      }
    }
  }
}
