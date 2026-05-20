// Dialog.mll — layout for the Dialog component.
//
// Decomposition (kernel primitives only):
//
//   Box [dialog-root]                ← stylable outer frame
//     Column [dialog-stack]          ← vertical stack: title, message, actions
//       Box [dialog-title]           ← stylable title row (its own `part`)
//         Text (content: slot: title)
//       Box [dialog-message]         ← stylable message row
//         Text (content: slot: message)
//       Box [dialog-actions]         ← stylable actions row
//         HostButton                 ← kernel-canonical clickable
//           label: slot: close-label
//           onTap : emit: onClose
//
// Why only Box / Column / Text / HostButton?
// ------------------------------------------
// These four primitives are the lowest-common-denominator set: every
// Mosaic backend on main today (React, SwiftUI, Qt, WebComponent, HTML,
// XAML) lowers them.  `If` is only present in HTML, WebComponent, and
// XAML right now; `For` is only present in HTML, WebComponent, and (just
// landing) React; `HostTable` is only present in React/SwiftUI/Qt/WebComp/
// HTML/XAML but adds complexity Dialog does not need.  Sticking to the
// LCD set makes Dialog the cross-backend smoke test that proves *every*
// emitter can ingest a userland package end-to-end.
//
// Why nested Boxes for the parts?
// -------------------------------
// mosstyle attaches style rules to *parts* (named via the `[part-name]`
// bracket annotation).  We want the title, message, and actions row to
// each be individually stylable — different padding, font, alignment —
// so each gets its own Box wrapper that owns a part name.  The inner
// Text / HostButton primitives stay style-free; they pick up colour and
// font from their parent Box via CSS inheritance (or its backend
// equivalent).

layout Dialog {
  Box [ dialog-root ] {
    Column [ dialog-stack ] {
      Box [ dialog-title ] {
        Text ( content: slot: title )
      }
      Box [ dialog-message ] {
        Text ( content: slot: message )
      }
      Box [ dialog-actions ] {
        HostButton (
          label: slot: close-label ,
          onTap: emit: onClose
        )
      }
    }
  }
}
