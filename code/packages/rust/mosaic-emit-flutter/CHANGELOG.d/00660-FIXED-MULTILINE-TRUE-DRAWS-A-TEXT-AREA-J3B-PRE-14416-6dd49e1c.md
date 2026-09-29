### Fixed — `multiline: true` draws a text area (J3b-pre, #14416)

Flutter never read `multiline`, so `Input [ … ] ( multiline: true )` rendered
as a single-line `TextField` on Flutter while every other native backend drew
a text area. This affected Trestle's note body and the note-type editor's
templates. The field now gets `keyboardType: TextInputType.multiline`,
`textInputAction: TextInputAction.newline`, `minLines: 8` (matching Compose)
and `maxLines: null`.

