// Tabs.mll — layout for the Tabs (v0.8).
//
//   Column [ tabs ]
//     Row [ tabs-bar ]
//       For (each: slot: headers, as: header, index: i)
//         HostButton [ tabs-tab ] (label: header, onClick: emit: onSelect)
//     Text [ tabs-panel ] (content: slot: active-body)
//
// The Row of headers and the single Text body live in the same outer
// Column so the body always renders directly below the bar.
//
// Active-state styling: the .msl flat-styles all tabs the same in
// v0.8; the active highlight needs sub-part state syntax (same
// caveat as Nav/Breadcrumb).

layout Tabs {
  Column [ tabs ] {
    Row [ tabs-bar ] {
      For ( each: slot: headers , as: header , index: i ) {
        HostButton [ tabs-tab ] (
          label : header ,
          onClick : emit: onSelect
        )
      }
    }
    Text [ tabs-panel ] (
      content : slot: active-body
    )
  }
}
