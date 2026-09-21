---
category: Repo policy / workflow reminders
---

# Two different values for what looks like one field may be two different fields; read the whole record before calling it drift

I was one edit away from filing "two different release commits are pinned for
the same oracle release" as a provenance defect. Two GitHub issues recorded the
pinned Closure release as commit `56007b2869ef…`; my working notes carried
`10ca677aff38…` for the same release tag. Same release, two hashes — obviously
drift, and drift in a pinned oracle is serious.

`tests/oracle/manifest.json` records **both**, deliberately, under two different
keys:

```json
"release":         { "commit_sha": "56007b2869ef…" },   // what the tag points at
"audited_source":  { "commit_sha": "10ca677aff38…" }    // the master commit read
                                                        // during the source audit
```

They answer different questions: which commit produced the artifact, and which
commit's source was read when auditing it. The issues quoted the first, which is
correct. My notes had the second, recorded as if it were the first. The
repository was right and my summary of it was wrong.

**This is the second time.** The first was calling a Prolog lexer's
`ANON_VAR = /_(?![A-Za-z0-9_])/` a latent defect, when the lookahead is
deliberate cross-language design — the canonical grammar is shared with a Python
lexer whose `re` supports it, and a documented patch function strips it for the
Rust backend. Same shape both times: a value that looks wrong in isolation, is
correct in the context I had not finished reading, and a report that would have
sent someone to "fix" working code.

**What to do.** When something looks like drift or a defect in code you did not
write, read the whole record that contains it before writing the report —
the full struct, both branches of the conditional, the generator as well as the
generated file. In particular:

- Two values that disagree may be two fields, not one field twice. Check the key,
  not just the value.
- If the report would be public and actionable, the cost of being wrong is
  someone else's time plus a correction; the cost of reading thirty more lines
  is thirty lines.
- A "defect" in deliberate design usually has a comment or a sibling call site
  explaining it. Not finding one is evidence you have not found the right file
  yet, not evidence that none exists.

Correcting a claim you have already published is cheap and worth doing
immediately; not publishing it is cheaper.
