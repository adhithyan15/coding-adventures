layout NestedRowConditional {
  Row [ composer ] {
    If ( when: slot: outer-visible ) {
      Text [ prompt ] ( content: "Task" )
      If ( when: slot: inner-focused ) {
        HostInput [ focused-input ] ( value: slot: value, placeholder: "What needs doing?" )
      }
      Else {
        HostInput [ unfocused-input ] ( value: slot: value, placeholder: "What needs doing?" )
      }
    }
    Else {
      Text [ unavailable ] ( content: "Unavailable" )
    }
  }
}
