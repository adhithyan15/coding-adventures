## Unreleased — the narration loader map leaves the eager chunk

`src/narration-sources.ts` holds one lazy loader per narration chapter file.
Its header says nothing may import it statically, but `src/main.ts` did, so
the whole map went into the entry chunk. That is about 2,500 path strings, one
per chapter per track. Six tracks' pre-A1 vocabulary tranches (about 300 new
chapters) pushed the entry chunk to 528,322 bytes, over the 500 kB
`check:bundle` budget.

`main.ts` now loads the module with a dynamic import when the first lesson is
read aloud. The entry chunk drops to 163,900 bytes, and it no longer grows when
a chapter is added. The largest eager chunk is now `handwriting-tools`, at
495,771 bytes, which this change leaves untouched.
