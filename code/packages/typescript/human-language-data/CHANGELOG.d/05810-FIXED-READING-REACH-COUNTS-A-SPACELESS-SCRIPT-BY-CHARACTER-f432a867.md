### Fixed — reading reach counts a spaceless script by character

- `passageWordCount` split every passage on whitespace. Japanese writes none,
  so `もうすこし、ゆっくりいってください` -- sixteen kana, four words -- counted as
  ONE token, and a six-line Japanese passage measured 6. That is
  indistinguishable from six single words.
- `passageLength` now counts a passage in the unit its own script makes
  available: by character for Han, hiragana, katakana and the CJK extensions;
  by whitespace-separated word everywhere else. A mixed line counts both, so
  `コーヒー 100` is five.
- This is also the unit Chinese's task shape asks for. Its part declares
  `unit: "items"`, and an item there is a character -- the old count reported
  words against a shape that never asked for one.
- Japanese's measured passage moves 6 -> 45 with no content change. Its floor of
  1/1 is unaffected, which is the point of recording floors as parts-in-reach
  rather than as a raw length.
- `passageWordCount` is kept as an alias so a caller that genuinely means words
  can say so. It is identical for every space-writing script.
- Closes HL-C371, which was filed rather than fixed because it would have
  changed the measurement for two tracks in the same change that first gave one
  of them a passage. Japanese has since landed, so the two are now separable.
