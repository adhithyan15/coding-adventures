// HelloDialog.mll — layout for the minimal "open a dialog" demo.
//
// One HostDialog wrapping a Column with the message and a Close button.
// The host project drives `open` via a DependencyProperty + a small
// ShowAsync()/Hide() bridge in code-behind.

layout HelloDialog {
  HostDialog [ shell ] (
    open  : slot: open ,
    modal : true ,
    title : slot: title ,
    onClose : emit: onClose
  ) {
    Column [ stack ] {
      Box [ message ] {
        Text ( content: slot: message )
      }
      Box [ actions ] {
        HostButton [ close-btn ] (
          label : "Close" ,
          onClick : emit: onClose
        )
      }
    }
  }
}
