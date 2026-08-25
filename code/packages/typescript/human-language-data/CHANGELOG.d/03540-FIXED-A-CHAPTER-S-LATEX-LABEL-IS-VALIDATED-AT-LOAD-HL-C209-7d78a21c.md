### Fixed - a chapter's LaTeX label is validated at load (HL-C209)

- `chapters.json` `label` was the one author-controlled field the book generator interpolated RAW into `\label{...}`; titles go through the LaTeX escaper, output paths through `safeOutput`, activity ids and script commands through their own regexes.
- A security review demonstrated a label of `ch:x}\immediate\write18{id}{` emitting a live control sequence into a generated `.tex`. Builds run plain `latexmk -xelatex` with no `-shell-escape`, so `\write18` is refused today — but `\input` and `\openout` are not, and a compiler flag is not a property this module can see or keep.
- `loadTrackChapters` now rejects any label outside `/^[A-Za-z0-9:_-]+$/`, which accepts all 900 committed labels and every convention in use.
- Pinned with a test that also proves the guard is falsifiable, and self-tested against the review's own hostile label before being trusted.


