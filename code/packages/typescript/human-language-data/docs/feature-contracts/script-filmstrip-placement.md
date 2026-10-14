# Script filmstrips: lesson placement and book rendering

WHERE on the page a strip goes is `filmstripBlockIndex` in
`strip-placement.ts`: the first Writing block, else the first Script block,
else the first block whose writing stage shows a model, never a dictation or
a composition. That module has no imports, so the language-ladder app runs
the same function on the same parse and shows each strip in the section the
book prints it in.
Generated book chapters rewrite the lesson's `.svg` image
destination to `.pdf`; the books workflow creates that PDF with `rsvg-convert`
before XeLaTeX runs.
