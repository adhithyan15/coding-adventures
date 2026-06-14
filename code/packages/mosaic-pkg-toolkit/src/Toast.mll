// Toast.mll — layout for the Toast component.
//
//   If (when: slot: open) {
//     Box [ toast ]
//       Column
//         Row [ toast-header ]
//           Text [ toast-title ] ( content: slot: title )
//           HostButton [ toast-close-btn ] ( label: "x", onClick: onClose )
//         Box [ toast-body ]
//           Text ( content: slot: message )
//   }
//
// Visibility is bound to the `open` slot via If, so toasts that
// aren't currently shown don't render. (No animation — that's
// out of scope per the spec §3.3 Tier 3 note.) The .msl positions
// the toast at the bottom-right with anchor margin; the host can
// override via the .msl cascade.

layout Toast {
  If ( when: slot: open ) {
    Box [ toast ] {
      Column [ toast-column ] {
        Row [ toast-header ] {
          Text [ toast-title ] (
            content : slot: title
          )
          HostButton [ toast-close-btn ] (
            label : "x" ,
            onClick : emit: onClose
          )
        }
        Box [ toast-body ] {
          Text [ toast-message ] (
            content : slot: message
          )
        }
      }
    }
  }
}
