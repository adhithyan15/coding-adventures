layout Picker {
  Column [ root ] {
    HostButton [ from-slot ] ( label: "Slot", selected: slot: on )
    HostButton [ raised ] ( label: "Literal", selected: true )
    For ( each: slot: flags , as: flag ) {
      HostButton [ from-binding ] ( label: "Binding", selected: flag )
    }
    For ( each: slot: items , as: item , index: i ) {
      HostButton [ option ] ( label : item , selected : ( i == selectedIndex ) , onClick : emit: onSelect )
    }
    HostButton [ plain ] ( label: "Plain" )
  }
}
