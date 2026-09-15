// PhotoPickerApp.mll — layout for the UI59 `files.open` reference app.
//
// A status line above a single button. `picking` is exposed on the
// interface (see PhotoPickerApp.mil) for a future host to disable the
// button mid-pick; this minimal layout doesn't wire that yet -- the
// status text alone ("Opening the photo picker...") already says so.

layout PhotoPickerApp {
  Column [ root ] {
    Box [ status-row ] {
      Text ( content: slot: status )
    }
    Box [ actions ] {
      HostButton [ pick-btn ] (
        label : "Pick a Photo" ,
        onClick : emit: onPickPhoto
      )
    }
  }
}
