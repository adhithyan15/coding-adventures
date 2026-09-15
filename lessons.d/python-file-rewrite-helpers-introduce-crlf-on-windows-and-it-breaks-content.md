# Python file-rewrite helpers introduce CRLF on Windows, and it breaks content hashes

**Context:** `human-language-data`, PR #10068. CI failed `check:books` with "generated
output is missing or stale" on two chapters, while the identical check passed locally and
`git status` was clean.

**What happened:** I edit files with small Python helpers — `s = io.open(p, encoding='utf8')
.read()` … `io.open(p, 'w', encoding='utf8').write(s)`. On Windows that **write translates
`\n` into `\r\n`**. The curriculum's book/narration hashes are computed over file content,
line endings included, so:

- **Locally** the lesson Markdown was CRLF, the generator read CRLF, the hash was computed
  from CRLF, and the check compared CRLF to CRLF. Self-consistent, therefore green.
- **In CI** git checks out LF (the repo stores LF), the generator produced LF output, and
  the committed hash — computed from CRLF — no longer matched. Stale.

The mismatch is invisible to `git status` and `git diff`, because git normalises on the way
in. It is only visible to something that hashes bytes.

**The tell I ignored:** git printed `warning: in the working copy of '<file>', CRLF will be
replaced by LF the next time Git touches it` on *every single write*, across several
commits. I read a wall of repeated warnings as noise. It was the defect announcing itself in
plain language.

**Fix (three parts):**
1. When rewriting a file from Python, do it in **binary**: read `'rb'`, write `'wb'`, and
   keep the bytes you did not intend to change. Or pass `newline=''` to `io.open` in text
   mode, which disables translation on both read and write.
2. If a content hash or generated artifact is involved, **normalise before hashing** —
   `raw.replace(b'\r\n', b'\n')` — rather than trusting the editor's defaults.
3. **A gate that passes locally and fails in CI on "stale generated output" is almost always
   line endings or path separators, not logic.** Check `file <path>` for "CRLF line
   terminators" and compare `git show HEAD:<path> | file -` against the working tree before
   investigating anything else.

**Generalisation:** any check that compares *bytes* rather than *parsed structure* will
diverge between a CRLF working tree and an LF repository. That includes content hashes,
signature files, and golden-output tests. Prefer hashing normalised content, and never let
a helper script decide line endings for a file it did not create.
