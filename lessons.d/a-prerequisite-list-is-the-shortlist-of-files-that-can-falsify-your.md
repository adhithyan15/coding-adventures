---
category: Repo policy / workflow reminders
---

# A prerequisite list is the shortlist of files that can falsify your lesson

Three tranches running, the security review has caught the same failure in my
authored prose: **a lesson asserting something the corpus itself contradicts.**

| tranche | what I wrote | what the corpus already said |
|---|---|---|
| 4a | *alrededor* "traces back to *rota*" | `ES-C347-alrededor`: *retrō*, **pulled sideways by** *rota* |
| 4b | "nobody told you *claro* was an adjective" | `ES-C272-si-claro`: "*Claro* is Latin **clarus**" |
| 4c | Roman soldiers were paid in salt | `ES-C361-sal`: "an eighteenth-century invention with no ancient source" |

After the second one I wrote down the rule: *before writing that the corpus has
not said something, read the lesson that would have said it.* Good rule. It did
not stop the third, because it does not say **which** lesson to read, and the
corpus is thirteen hundred files.

The third catch supplied the missing half. All three falsifiers were in the new
lesson's own **`prerequisites:` list**. `ES-C456-ensalada` declares
`prerequisites: [..., ES-C361-sal, ...]` — the file that debunks the salt story
is named in the frontmatter of the lesson repeating it. Same for
`ES-C457-medicamento`, which lists `ES-C394-medico`, the lesson that calls the
"healing was named after measuring" inference circular and forbids it.

That is not a coincidence, and it will hold anywhere a corpus has an explicit
dependency edge. **You cite a lesson as a prerequisite precisely because it
covers the same ground**, so it is the most likely thing to have already made —
or already refuted — the claim you are about to make.

So the check is small and bounded. Before asserting an etymology, a "nobody
told you", or any fact about what the reader has and has not met:

```bash
# read every prerequisite of the lesson you are writing, in full
python3 - <<'PY'
import re,sys
fm=open(sys.argv[1]).read().split('---')[1]
for p in re.search(r'^prerequisites:\s*\[(.*?)\]',fm,re.M|re.S).group(1).split(','):
    p=p.strip()
    if p: print(open(f'spanish/lessons/{p}.md').read())
PY
```

Three to five files, not thirteen hundred. Grepping the whole corpus for a
keyword would not have caught any of the three, because none of them phrases
the claim the way I did — `ES-C361-sal` says "eighteenth-century invention",
not "salary".

The general shape: **a dependency edge is a claim that two things overlap.
Whatever else that buys you, it tells you exactly where to look for a
contradiction.**
