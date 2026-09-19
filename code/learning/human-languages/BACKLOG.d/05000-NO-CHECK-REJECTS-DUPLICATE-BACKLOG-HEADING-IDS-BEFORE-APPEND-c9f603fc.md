## HL-C411 — no check rejects duplicate backlog heading ids before append-only merge

**Status: OPEN. Depends on the HL-C410 repair merged by #15593.** Two concurrent
branches both selected `04980`/`HL-C409` after reading the same repository state:

| immutable filename | current heading id | subject |
|---|---|---|
| `04980-HL-C409-FRONTMATTER-UNQUOTE-...` | `HL-C410` | `unquote()` does not decode escapes |
| `04980-HL-C409-MALAYALAM-A1-READING-...` | `HL-C409` | A1 reading stops seventeen words short |

#15593 resolved the ambiguous heading id by changing the first shard's heading
to `HL-C410` while preserving its append-only filename. The filenames still
share rank `04980` and retain the historical `HL-C409` text, but neither is an
error: the full basenames differ, and the doc-shard contract deliberately
permits parallel fragments to share a rank.

**The remaining defect is at author time.** Backlog ids are chosen by reading
the directory, so two open branches can independently choose the same next id.
Nothing rejects the duplicate until a person notices it. Both branches can pass
all local gates because each is unique relative to the base it started from.

**Why the filename was not renamed.** Markdown history shards are append-only.
A rename is a delete plus an add, and the books workflow correctly rejected the
first attempted repair:

```
##[error]Markdown history shards are append-only; restore these deleted files:
code/learning/human-languages/BACKLOG.d/04980-HL-C409-FRONTMATTER-...
```

That attempted rename had passed `npm run validate`, all twelve package gates,
the full suite, the strict Malayalam book compile and the LaTeX warning scan.
The package checks shard shape; the workflow also protects committed history.

**What closing this needs.** Add an author-time allocation or validation rule
that remains safe when curriculum PRs are concurrent. A check against one
branch's base is insufficient because neither branch can see the other. The id
must instead be derived from an independently unique value, reserved centrally,
or verified against active PRs before publication. Until then, re-read both main
and active human-language PRs before assigning the next `HL-C` id.
