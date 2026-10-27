// Notes.mll — layout for the list-plus-editor notes view.
//
// Composition (0.3.0 — task-app-notes-ui-v1.md addendum v1.3):
//
//   Column [ notes-root ]
//     Text [ notes-heading ]
//     Row [ notes-body ]
//       Column [ notes-list ]
//         HostButton [ notes-new ]                    "+ New note"
//         pkg::mosaic-pkg-toolkit::RecordList         one row per note
//       If ( a note is selected )
//         Column [ notes-editor ]
//           Text      [ notes-task-label ]            attach-to-task field,
//           HostInput [ notes-task-input ]            Trestle's own
//           pkg::mosaic-pkg-draft-editor::DraftEditor title, body, actions
//       Else
//         Column [ notes-empty ]                      "select or start a note"
//
// What Notes owns, and what it borrows
// ------------------------------------
// Notes used to draw its own row buttons, title and body fields and action
// row. Those are now the shared components every product uses (the editor
// convergence audit, mosaic-editor-convergence-audit-v1.md, probe C):
//
//   was (0.2.0)                         now (0.3.0)
//   ──────────────────────────────────  ─────────────────────────────────
//   notes-row-on / notes-row-off        RecordList  (record-list-*)
//   notes-title-input                   DraftEditor (draft-editor-title)
//   notes-body-input                    DraftEditor (draft-editor-body)
//   notes-save / -delete / -cancel      DraftEditor (draft-editor-save, …)
//
// What stays is Trestle's alone: "+ New note", the attach-to-task field and
// the empty-state hint. Every word on screen is a slot (see Notes.mil), so
// the package draws no English of its own.
//
// Every slot of RecordList and DraftEditor is bound here, even where the
// value is just passed through: an unbound slot of a mounted package
// component is a compile error (UI13 §1, E6 F1), not a zero value.
//
// Selecting a row: keys, not indices — same reasoning as TaskApp's Board
// section (see TaskApp.mil's board-columns note). RecordList compares each
// row's key (`row[0]`) with its `selected-key`, which is bound to Notes'
// `selected-note-id`, and reports a click as the row's index; the host maps
// that index back to an id through the list it just built.
//
// The body is DraftEditor's text area, `HostInput ( multiline : true )`:
// Enter is a line break and Ctrl+Enter (Cmd+Enter on macOS) raises `onSave`,
// exactly as the Save button does (UI25 §4, UI29 §2.1). Notes forwards that
// `onSave` to its own, so the chord saves a note as it did before.
//
// The task field sits ABOVE the editor, as Journal composes its tags and
// journal fields above its DraftEditor. Below would put it after the Save
// button that commits it. Its visible label is also its accessible name
// (`a11y-label`), the same rule DraftEditor applies to its own fields, so
// what a screen reader announces is what a sighted user reads.

layout Notes {
  Column [ notes-root ] {
    Text [ notes-heading ] ( content : slot: notes-title )
    Row [ notes-body ] {
      Column [ notes-list ] {
        HostButton [ notes-new ] ( label : slot: new-note-label , onClick : emit: onNewNote )
        pkg::mosaic-pkg-toolkit::RecordList (
          rows : slot: note-rows ,
          selected-key : slot: selected-note-id ,
          onSelect : emit: onSelectNote
        )
      }
      If ( when: slot: selected-note-id ) {
        Column [ notes-editor ] {
          Text [ notes-task-label ] ( content : slot: task-label )
          HostInput [ notes-task-input ] (
            value : slot: task-name-value ,
            placeholder : slot: task-placeholder ,
            a11y-label : slot: task-label ,
            onChange : emit: onTaskNameChange
          )
          pkg::mosaic-pkg-draft-editor::DraftEditor (
            title-label : slot: title-label ,
            title-value : slot: title-value ,
            title-placeholder : slot: title-placeholder ,
            body-label : slot: body-label ,
            body-value : slot: body-value ,
            body-placeholder : slot: body-placeholder ,
            save-label : slot: save-label ,
            delete-label : slot: delete-label ,
            cancel-label : slot: cancel-label ,
            onTitleChange : emit: onTitleChange ,
            onBodyChange : emit: onBodyChange ,
            onSave : emit: onSave ,
            onDelete : emit: onDelete ,
            onCancel : emit: onCancel
          )
        }
      }
      Else {
        Column [ notes-empty ] {
          Text [ notes-empty-hint ] ( content : slot: empty-hint )
        }
      }
    }
  }
}
