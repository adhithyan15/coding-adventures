// Tabs.mll — layout for the Tabs (v0.8).
//
//   Column [ tabs ]
//     Row [ tabs-bar ]
//       For (each: slot: headers, as: header, index: i)
//         If (when: i == activeIndex)
//           HostButton [ tabs-tab-active ] (label: header, onClick: emit: onSelect)
//         Else
//           HostButton [ tabs-tab ] (label: header, onClick: emit: onSelect)
//     Text [ tabs-panel ] (content: slot: active-body)
//
// The Row of headers and the single Text body live in the same outer
// Column so the body always renders directly below the bar.
//
// active-index remains the public kebab-case slot; predicates use
// activeIndex because emitters expose kebab slots as backend-safe
// camel/Pascal identifiers.

layout Tabs {
  Column [ tabs ] {
    Row [ tabs-bar ] {
      For ( each: slot: headers , as: header , index: i ) {
        If ( when: i == activeIndex ) {
          HostButton [ tabs-tab-active ] (
            label : header ,
            onClick : emit: onSelect
          )
        }
        Else {
          HostButton [ tabs-tab ] (
            label : header ,
            onClick : emit: onSelect
          )
        }
      }
    }
    Text [ tabs-panel ] (
      content : slot: active-body
    )
  }
}
