// Radio.mll — layout for the Radio.
//
// Same If/Else swap pattern as Checkbox.mll. The .msl renders the
// box as a circle and the selected indicator as a filled dot
// rather than a checkmark.

layout Radio {
  Row [ radio ] {
    If ( when: slot: selected ) {
      HostButton [ radio-box-selected ] (
        label : "•" ,
        disabled : slot: disabled ,
        onClick : emit: onSelect
      )
    }
    Else {
      HostButton [ radio-box-unselected ] (
        label : "" ,
        disabled : slot: disabled ,
        onClick : emit: onSelect
      )
    }
    Text [ radio-label ] (
      content : slot: label
    )
  }
}
