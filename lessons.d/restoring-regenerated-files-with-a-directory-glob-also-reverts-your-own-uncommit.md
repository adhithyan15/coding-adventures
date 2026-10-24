---
category: Repo policy / workflow reminders
---

# Restoring regenerated files with a directory glob also reverts your own uncommitted edits; stage first or restore an explicit file list

**What went wrong.** To compare `grammar-tools generate-rust-compiled-grammars`
output before and after a grammar-tools change, I ran the sweep (which rewrites
`code/packages/rust/*/src/_grammar.rs`) and then cleaned up with
`git checkout -- code/packages/rust/*/src/`. The glob also matched
`grammar-tools/src`, `lexer/src` and `parser/src`, where the change under test
lived uncommitted and unstaged, and silently reverted all of it.

**Fix.** The edits had been made by scripts still in the session, so they were
replayed. The sweep is now cleaned up from the tool's own output list:

```bash
grep -o "OK -> .*" sweep.log | sed 's/OK -> //' > outputs.txt
while read f; do
  if git ls-files --error-unmatch "$f" >/dev/null 2>&1; then git checkout -- "$f"
  else rm -f "$f"; fi
done < outputs.txt
```

**Do differently.** Before running any generator that writes into the tree,
`git add` (or commit) your own work: `git checkout -- <path>` restores from the
index, so staged edits survive a careless restore. Restore generated files by
explicit path, never by a directory glob that can reach hand-written code.
