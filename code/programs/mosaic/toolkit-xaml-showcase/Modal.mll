// Modal.mll — layout for the Modal.
//
//   HostDialog [ modal-shell ] (
//     open: slot: open ,
//     title: slot: title ,
//     onClose: emit: onClose ,
//     modal: true
//   ) {
//     Column [ modal-stack ]
//       Box [ modal-body ]
//         Text ( content: slot: message )
//       Box [ modal-actions ]
//         HostButton [ modal-close-btn ] (
//           label: slot: close-label ,
//           onClick: emit: onClose
//         )
//   }
//
// HostDialog at the layout root means the XAML backend hoists this
// to a <ContentDialog> root (Fix A1 from the demo catalog). Other
// backends produce their native dialog idioms (React → <dialog>,
// SwiftUI → .sheet, Qt → Popup).

layout Modal {
  HostDialog [ modal-shell ] (
    open : slot: open ,
    title : slot: title ,
    onClose : emit: onClose ,
    modal : true
  ) {
    Column [ modal-stack ] {
      Box [ modal-body ] {
        Text [ modal-body-text ] (
          content : slot: message
        )
      }
      Box [ modal-actions ] {
        HostButton [ modal-close-btn ] (
          label : slot: close-label ,
          onClick : emit: onClose
        )
      }
    }
  }
}
