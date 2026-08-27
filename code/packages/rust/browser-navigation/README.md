# browser-navigation

Reusable, host-neutral browser navigation state.

The crate deliberately knows nothing about HTML, layout, painting, windows, or
storage. `NavigationHistory` owns Back/Forward/Home/Reload stack semantics.
`VisitedLinks` owns session-scoped URL identity using `url-parser` canonical
forms, including scheme/host case folding, default-port removal, dot-segment
removal, percent-escape normalization, and fragment-insensitive resource
identity.

Keeping these types below `venture-browser-core` lets any browser shell reuse
the same behavior without depending on Venture's synchronous page pipeline.
