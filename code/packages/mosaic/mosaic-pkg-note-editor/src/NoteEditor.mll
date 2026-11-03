// NoteEditor.mll - target-neutral focused note editor layout.
//
// Composition (0.2.0 — E4 of mosaic-editor-convergence-audit-v1.md):
//
//   Column [ note-editor ]
//     Text [ editor-title ]
//     Row  [ note-metadata ]       note id, note type, deck
//     Row  [ note-choice-row ]     note-type and deck option lists
//     Row  [ editor-body ]
//       Column [ field-list-column ]      one button per field
//       Box    [ focused-field-column ]
//         pkg::mosaic-pkg-draft-editor::DraftEditor   the focused field
//         Text      [ tags-label ]
//         HostInput [ tags-input ]                    a11y-label: tags-label
//     Row  [ editor-actions ]      Save note / Delete / Cancel
//
// What NoteEditor owns, and what it borrows
// -----------------------------------------
// NoteEditor is an Anki note FORM that contains an editor: the pickers, the
// tags field and the action row are Engram's alone, and stay here. The one
// text surface in it, the focused field, is now the shared editor every
// product uses (the audit's probe B):
//
//   was (0.1.0)                           now (0.2.0)
//   ────────────────────────────────────  ──────────────────────────────────
//   Text      [ selected-field-label ]    DraftEditor (draft-editor-body-label)
//   HostInput [ selected-field-input ]    DraftEditor (draft-editor-body)
//     single-line, no commit binding        multi-line; Ctrl/Cmd+Enter saves
//
// DraftEditor is mounted BODY-ONLY. Its title and its three actions are all
// label-gated, so binding their labels to "" removes them: NoteEditor keeps
// its own Save note / Delete / Cancel row, whose Delete label is stateful
// ("Discard draft", "Delete note", "Confirm delete") and whose buttons are
// styled as Engram's. The empty strings are not English, so the package
// still draws no words of its own; every word on screen is a NoteEditor
// slot bound by the host.
//
// Every DraftEditor slot is bound, the empty ones included: an unbound slot
// of a mounted package component is a compile error (UI13 §1, E6 F1), not
// a zero value.
//
// What the user notices
// ---------------------
// 1. The field is multi-line. Enter is a line break, which is what an Anki
//    field holds (Anki fields are routinely several lines). It used to do
//    nothing.
// 2. Ctrl+Enter (Cmd+Enter on macOS) saves the note: DraftEditor's body
//    raises its `onSave` on that chord (UI25 §4, UI29 §2.1), and NoteEditor
//    forwards it to `onSaveNote`, the same event as the Save note button.
//    That is Anki's own Add-dialog shortcut. DraftEditor raises `onSave` on
//    the chord even with no Save button drawn, so the forward is all it
//    takes.
// 3. The field's visible label is also its accessible name. DraftEditor
//    binds the body's `a11y-label` to `body-label`, which is this package's
//    `selected-field-label`. So the host must never send "" there, or the
//    field has no name; Engram sends "Field" while no field is selected.
// 4. The tags field gains the same treatment: its `a11y-label` is the
//    visible `tags-label`, so a screen reader no longer relies on the
//    placeholder for a name.

layout NoteEditor {
  Column [ note-editor ] {
    Text [ editor-title ] (
      content : slot: editor-label
    )
    Row [ note-metadata ] {
      Column [ note-id-meta ] {
        Text [ note-id-label ] (
          content : slot: note-id-label
        )
        Text [ note-id-value ] (
          content : slot: note-id-value
        )
      }
      Box [ note-type-meta ] {
        Text [ note-type-label ] (
          content : slot: note-type-label
        )
        Text [ note-type-value ] (
          content : slot: note-type-value
        )
      }
      Box [ deck-meta ] {
        Text [ deck-label ] (
          content : slot: deck-label
        )
        Text [ deck-value ] (
          content : slot: deck-value
        )
      }
    }
    Row [ note-choice-row ] {
      Column [ note-editor-note-type-list-column ] {
        Text [ note-editor-note-type-options-label ] (
          content : slot: note-type-options-label
        )
        Column [ note-editor-note-type-list ] {
          For ( each: slot: note-type-names , as: note-type , index: note-type-index ) {
            HostButton [ note-editor-note-type-option ] (
              label : note-type ,
              onClick : emit: onSelectNoteType
            )
          }
        }
      }
      Column [ note-editor-deck-list-column ] {
        Text [ note-editor-deck-options-label ] (
          content : slot: deck-options-label
        )
        Column [ note-editor-deck-list ] {
          For ( each: slot: deck-names , as: deck , index: deck-index ) {
            HostButton [ note-editor-deck-option ] (
              label : deck ,
              onClick : emit: onSelectDeck
            )
          }
        }
      }
    }
    Row [ editor-body ] {
      Column [ field-list-column ] {
        Text [ fields-label ] (
          content : slot: fields-label
        )
        Column [ note-field-list ] {
          For ( each: slot: field-labels , as: field , index: field-index ) {
            HostButton [ note-field-option ] (
              label : field ,
              onClick : emit: onSelectField
            )
          }
        }
      }
      Box [ focused-field-column ] {
        pkg::mosaic-pkg-draft-editor::DraftEditor (
          title-label : "" ,
          title-value : "" ,
          title-placeholder : "" ,
          body-label : slot: selected-field-label ,
          body-value : slot: selected-field-value ,
          body-placeholder : slot: selected-field-placeholder ,
          save-label : "" ,
          delete-label : "" ,
          cancel-label : "" ,
          onBodyChange : emit: onFieldValueChange ,
          onSave : emit: onSaveNote
        )
        Text [ tags-label ] (
          content : slot: tags-label
        )
        HostInput [ tags-input ] (
          value : slot: tags-value ,
          placeholder : slot: tags-placeholder ,
          a11y-label : slot: tags-label ,
          disabled : false ,
          onChange : emit: onTagsChange
        )
      }
    }
    Row [ editor-actions ] {
      HostButton [ save-button ] (
        label : slot: save-label ,
        onClick : emit: onSaveNote
      )
      HostButton [ delete-button ] (
        label : slot: delete-label ,
        onClick : emit: onDeleteNote
      )
      HostButton [ cancel-button ] (
        label : slot: cancel-label ,
        onClick : emit: onCancel
      )
    }
  }
}
