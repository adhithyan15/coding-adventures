// Notes.mll — layout for the list-plus-editor notes view.
//
// Composition:
//
//   Column [ notes-root ]
//     Text [ notes-heading ]
//     Row [ notes-body ]
//       Column [ notes-list ]           (+ New note, one row per note)
//       If ( a note is selected )
//         Column [ notes-editor ]        (title, body, actions)
//       Else
//         Column [ notes-empty ]         ("select or start a note" hint)
//
// Selecting a row: keys, not indices — same reasoning as TaskApp's Board
// section (see TaskApp.mil's board-columns note). `If ( when: ( n[0] ==
// selectedNoteId ) )` compares the row's own id against the host-owned
// selected-note-id slot, the same cross-loop-variable-vs-slot comparison
// Grid.mll's `is-editing: ( r == editRow && c == editCol )` already
// proved works — but note the CAMELCASE form: inside a parenthesized
// expression a slot is referenced by its camelCase identifier
// (`selectedNoteId`), not the kebab-case `slot: selected-note-id` form
// used everywhere else. Writing the kebab-case name bare here compiles
// (nothing rejects it) but is silently wrong: the emitted JS parses
// `selected-note-id` as subtraction (`selected - note - id`), three
// undefined identifiers, not one — a real bug found live-testing this
// package, not a hypothetical.
//
// The legacy `Input` primitive, once, deliberately: UI29's kernel
// `HostInput` is single-line only (no `multiline` support was carried
// forward from `Input`/UI25 — see code/specs/task-app-notes-ui-v1.md).
// The note body genuinely needs multiple lines, so `notes-body-input`
// uses `Input ( multiline: true )` rather than a single-line `HostInput`.
// This is the only such use in this package.

layout Notes {
  Column [ notes-root ] {
    Text [ notes-heading ] ( content : slot: notes-title )
    Row [ notes-body ] {
      Column [ notes-list ] {
        HostButton [ notes-new ] ( label : "+ New note" , onClick : emit: onNewNote )
        For ( each: slot: note-rows , as: n , index: ni ) {
          If ( when: ( n[0] == selectedNoteId ) ) {
            HostButton [ notes-row-on ] ( label : ( n[1] ) , onClick : emit: onSelectNote )
          }
          Else {
            HostButton [ notes-row-off ] ( label : ( n[1] ) , onClick : emit: onSelectNote )
          }
        }
      }
      If ( when: slot: selected-note-id ) {
        Column [ notes-editor ] {
          HostInput [ notes-title-input ] (
            value : slot: title-value ,
            placeholder : "Title" ,
            onChange : emit: onTitleChange
          )
          Input [ notes-body-input ] (
            value : slot: body-value ,
            placeholder : "Write something…" ,
            multiline : true ,
            onChange : emit: onBodyChange
          )
          Row [ notes-actions ] {
            HostButton [ notes-save ] ( label : "Save" , onClick : emit: onSave )
            HostButton [ notes-delete ] ( label : "Delete" , onClick : emit: onDelete )
            HostButton [ notes-cancel ] ( label : "Cancel" , onClick : emit: onCancel )
          }
        }
      }
      Else {
        Column [ notes-empty ] {
          Text [ notes-empty-hint ] ( content : "Select a note, or start a new one." )
        }
      }
    }
  }
}
