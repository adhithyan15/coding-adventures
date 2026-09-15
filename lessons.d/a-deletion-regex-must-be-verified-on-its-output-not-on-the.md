# A deletion regex must be verified on its OUTPUT, not on the token's absence

Removing nine cross-volume lesson ids (`TE-C29`, `AR-C27`, …) from prose, I checked the
only thing that was easy to check: that no id remained. Zero. Every generated artifact
regenerated, every `--check` passed, 392 tests green.

The regex had also eaten the words **around** each id. Thirteen sites, six of them
printed in the PDF and read aloud in the audio:

- `shares's spectacular PIE root?` — a recall question whose answer object vanished
- `treat it the same cautious way treated its own equivalent finding` — no subject
- `the same hedge already applied to Kannada's *aparāhna* in: thin evidence` — dangling
- `as Spanish fui, taught in), and war/waren` — dangling inside a parenthesis
- `a distinction this arc cares about, per's own finding` — `per's`

The absence of the token was never in question. What the sentence read like afterwards
was, and no automated check I had could see it. A security-review subagent reading the
prose caught all thirteen.

**Rule:** when a regex deletes a token from human-facing text, the deletion changes a
*sentence*, not a *string*. Diff the prose and read it. Grep the result for the
signatures of a bad splice — `'s` with no owner, `( `, ` )`, `in:`, `,,`, doubled
spaces, a verb with no subject. And prefer a replacement that names what the token
named (`TE-C29` → `Telugu's`) over one that deletes it, so the sentence keeps its
grammar by construction.

**Corollary, same session:** `git checkout -- <file>` to undo a *test* mutation threw
away that file's real, uncommitted fix too — it restores from the index, and nothing
was staged. Mutate a **copy** in the scratchpad, or stage the real work first.
