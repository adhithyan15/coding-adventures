### Changed — a declared filmstrip on a word lesson lands on its first script-introducing block (HL-C443)

`filmstripBlockIndex` now picks, in order:

1. the first Writing block;
2. the first Script block;
3. for a declared target on a lesson with neither, the first block that
   introduces a `-SCRIPT-` atom.

Derived candidates still need a Writing or Script block.

This lets `FA-C03-chist` drop the image it had placed by hand. The Persian
book still prints چ, and the lesson Markdown (which the app and narration
read) no longer references a filmstrip. That keeps filmstrips book-only, which
is what lets the app leave them out of its eager bundle.
