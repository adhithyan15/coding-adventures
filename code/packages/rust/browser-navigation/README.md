# browser-navigation

Reusable, host-neutral browser navigation state.

The crate deliberately knows nothing about HTML, layout, painting, windows, or
storage. `NavigationHistory` owns Back/Forward/Home/Reload stack semantics.
Each entry has a stable session-local identifier so consumers can associate
state with repeated occurrences of the same URL without leaking document or
widget policy into this crate. Redirect replacement preserves that identity;
fresh navigation allocates a new one.
`VisitedLinks` owns session-scoped URL identity using `url-parser` canonical
forms, including scheme/host case folding, default-port removal, dot-segment
removal, percent-escape normalization, and fragment-insensitive resource
identity.

Keeping these types below `venture-browser-core` lets any browser shell reuse
the same behavior without depending on Venture's synchronous page pipeline.
