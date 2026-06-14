// Dialog.mll — layout for the Dialog component (v0.2.0).
//
// Decomposition (kernel primitives only):
//
//   HostDialog [dialog-shell]           ← native dialog primitive
//     open  : slot: open                  (visibility driven by host slot)
//     modal : true                        (compile-time keyword; UI29-1 §2.1)
//     title : slot: title                 (host primitive's title slot)
//     onClose : emit: onClose             (fires on Esc / backdrop / close)
//   {
//     Column [dialog-stack]             ← vertical stack inside the dialog
//       Box [dialog-message]            ← stylable message row
//         Text (content: slot: message)
//       Box [dialog-actions]            ← stylable actions row
//         HostButton                    ← explicit close button
//           label: slot: close-label
//           onTap: emit: onClose
//   }
//
// What changed from v0.1.0
// ------------------------
// v0.1.0 used `Box [dialog-root] { Column { Box [dialog-title] { ... } } }`
// — i.e. it built the dialog frame from scratch out of Boxes.  v0.2.0
// makes `HostDialog` the layout root and lets the platform render the
// native dialog element: `<dialog>` on React/HTML/WebComponent, `.sheet`
// on SwiftUI, `Popup` on Qt, `ContentDialog` on XAML.  Three things
// fall out:
//
//   1. The outer Box wrapper is gone — `HostDialog` IS the root.
//   2. The inner `dialog-title` Box is gone — `HostDialog`'s `title:`
//      slot renders the title natively (with the platform's typography
//      and accessibility metadata).
//   3. The userland composition shrinks while the rendered behavior
//      gets dramatically richer: modal blocking, focus trap,
//      Esc-to-close, top-layer rendering, screen-reader `dialog` role.
//
// What stayed the same
// --------------------
// The mosstyle parts vocabulary still works the way authors expect:
// `dialog-shell` (renamed from `dialog-root`) styles the dialog frame
// via the platform's dialog-element styling hook, `dialog-message`
// styles the body text row, and `dialog-actions` styles the button row.
//
// Why HostButton and not the dialog primitive's "default action" slot?
// -------------------------------------------------------------------
// HostDialog deliberately does not include built-in OK/Cancel buttons —
// kernel primitives carry the *structural* semantics (visibility,
// modality, dismiss policy) and leave action vocabulary to userland.
// Different products want different button text, different button
// counts, different positions; a kernel-level "default action" slot
// would constrain that.  Dialog v0.2.0 exposes a single close button
// because that's the package's contract; richer dialog packages
// (Confirm, Alert, Prompt) compose differently.

layout Dialog {
  HostDialog [ dialog-shell ] (
    open    : slot: open ,
    modal   : true ,
    title   : slot: title ,
    onClose : emit: onClose
  ) {
    Column [ dialog-stack ] {
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
