---
category: CI & GitHub Actions
---

# A new -count repeat of an existing real-kernel test exposes its latent flakes on the older strace

**What went wrong.** The D50 PR added a repeat step to the kiln-sandbox job
(`-count=20` for the new restarted-spawn test, `-count=5` for the PRE-EXISTING
`TestSandboxStraceSignals` and `TestSandboxStraceDyingThreads`, on ubuntu-24.04 and
ubuntu-26.04). `TestSandboxStraceDyingThreads` had passed every single run for
weeks, but on 24.04 (strace 6.8) one of five repeats failed: strace 6.8 printed a
thread killed mid-call as ONE line, `newfstatat(AT_FDCWD<...>, "data.txt",
<unfinished ...>) = ?`. The parser correctly refuses it (`unparsable-line`, fail
closed), but the test's "is this a dying shape" check only knew the split spellings
(an unfinished line, then a resumed half), so it called the refusal unexpected.
Strace 6.19 on 26.04 never prints that form, so a single run on either image mostly
passed.

**Fix.** The test recognises the one-line form as a dying shape (test-only; the
parser is unchanged and still refuses it).

**Do differently.** Repeating a kernel-timing test is how latent spellings are
found; expect the first repeat of an old test, on every image in the matrix, to find
one, and budget for it. The tolerance list of such a test is a property of the
strace versions in CI, not of the code: when a new image or strace version joins, or
a repeat count rises, re-derive it. Give each repeat its own `-timeout` (go's default
is 10 minutes for the whole run) and make "the rule under test fired" a warning
only when its trigger is a race you cannot force.
