## Unreleased — a list filmstrip is captioned as a list

- A writing lesson that teaches several letters at once ("ক — ণ — শ",
  "ن، ت، ث") used to caption its filmstrip "How ক — ণ — শ is written, stroke
  by stroke", as if the list were one thing. The caption now comes from
  human-language-data's `filmstripCaption` (re-exported by `figures.ts`), the
  same module the book captions the strip with: "How these letters are
  written, stroke by stroke: ক, ণ, শ", the letters separated by an English
  comma whatever the lesson used. A single letter or a word reads as before.
  `figures.test.ts` pins all three shapes.
