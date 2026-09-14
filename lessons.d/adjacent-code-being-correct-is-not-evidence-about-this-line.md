# Adjacent code being correct is not evidence about this line

Three times in one day, in unrelated files: `doc-shard.ts` kept a sanitization gap after
the CLI beside it was fixed; `shard.ts` kept a bare `catch → "missing"` while
`shard-cli.ts` had it right; and the `book.pdf` symlink guard was defended in review with
"same `[ -L ]` idiom as the figure guard twenty lines above." The first two were real
misses. The third was a real miss too — the idiom was right and the *coverage* was wrong,
which the analogy could not have revealed, because the sibling guarded a genuinely
complete set (`figures/*.pdf`) and this one did not.

**A sibling proves the idiom compiles, never that the coverage is complete.** When
tempted to write "same pattern as X, so it is fine", the honest move is to enumerate what
this line must cover and check the list — the sibling's list is the sibling's.

### The fourth instance, and the sharper form: copying a guard at half strength

A week later, a fifth: the assessment-artifact generator (`assessment-artifact-cli.ts`)
needed the same symlink protection `book-cli.ts` already carries for the generated
`.tex`. `book-cli.ts` carries **two** checks, and says why:

1. `lstat` the last component and refuse a non-regular-file, and
2. `realpath` the whole path and re-assert containment, *because* — in its own words —
   "`lstat` vets only the LAST component… resolving the whole chain and re-asserting
   containment is the only check that covers every component at once."

The new code cited `book-cli.ts` as its model and took only check 1. A committed
`core -> /somewhere/else` then passed: the `lstat` lands on a real directory *inside the
linked-to tree*, `isDirectory()` returns true, and every generated shard is written
outside the repository root. Security review caught it; a second round was needed.

This is not the same mistake as the three above, and the difference is the useful part.
Those were *appeals* — "same pattern as X, so it is fine". This was a **transcription**:
the sibling was opened, read, used as a template, and truncated. The comment explaining
the dropped half was on screen at the time.

**A guard copied from a sibling must be copied whole, and the sibling's comments are part
of the guard.** A comment that says "this check alone is not enough, which is why the next
one exists" is not commentary on the code — it is the specification for the line you are
about to not write. Before shipping a borrowed guard, diff it against its source and
account for every check the original has that yours does not.

Corollary, and the reason this keeps recurring across five instances in unrelated files:
**correct code sitting next to incorrect code does not make the neighbour correct.**
Proximity, shared idiom, and a shared author are all zero evidence. The only evidence is
enumerating what this call site must cover and checking the list.
