# Matching the other event is not a reason two handlers may do the same thing

2026-09-13.

Engram's note-type editor was not reset by the collection-level `SaveNoteType`,
where the editor-level `NoteTypeEditorSaveNoteType` did reset it, and a name
typed after a collection save was silently dropped. The fix looked obvious:
reset in both.

It was a data-loss regression, and the reasoning is what produced it.

- `NoteTypeEditorSaveNoteType` builds its note type from
  `note_type_from_editor_selection` — by construction it saves what the editor
  holds, so resetting afterwards is coherent.
- `SaveNoteType` resolves its target from the **payload** and never consults the
  editor at all.

So the two can name different note types, and an unconditional reset threw away
a draft of A because something saved B — for a brand-new model, the name,
stylesheet, every field rename and template body at once, `reset()` being
`*self = Self::default()`.

Before making two handlers behave alike, check they are talking about the same
object. Two events with matching names and adjacent match arms are not
necessarily two routes to one operation; here one was "save what is open" and
the other "upsert whatever you are given".

Caught in security review, which measured both trees rather than reading them.
The reachability was nil today — no shell emits the bare event — and that is not
a defence: the event is documented for host model editors and aliased to
`upsertNoteType`, which is what a sync or an import calls.
