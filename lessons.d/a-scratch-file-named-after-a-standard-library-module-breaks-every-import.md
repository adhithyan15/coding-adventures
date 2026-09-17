---
category: Python
---

# A scratch file named after a standard library module breaks every import that reaches it

Two instruments that read a PDF died on the same traceback, and no line of it mentioned the thing
that was wrong:

    from pypdf import PdfReader
      -> pypdf/_crypt_providers/_cryptography.py: import secrets
        -> scratchpad/secrets.py, line 20
          FileNotFoundError: lessons.d/a-suite-that-captures-stdout-...md

A scratch file called `secrets.py` sat in the directory those scripts lived in. **Running a script by
its path puts that script's own directory first on `sys.path`**, so the `import secrets` buried inside
`pypdf` resolved to my file instead of the standard library's. Mine then ran its module-level code — a
secret scanner with hardcoded relative paths — and failed on a path that does not exist from that
working directory. That is the run-a-script-by-path case, which is the one measured here; other launch
modes were not tested.

**Measured in that one scratchpad:**

    stdlib names shadowed by files there     1     (secrets.py)
    scripts importing pypdf                  5
    of those, killed outright                4
    of those, silently degraded              1
    scripts importing urllib or requests    78
    of those, reaching secrets at all        0
    total .py files in the directory       524
    stdlib names shadowed after the rename   0

**The rows do not share a method, and writing "measured" without saying how is the same shortcut this
shard is about.** The four import counts are AST-derived. The two shadow-name rows and the file total
are directory listings tested against `sys.stdlib_module_names`, with no parsing at all — which is
why the two files that fail to parse still sit inside the 524 while appearing in no AST count. The
reach row is a clean-subprocess `sys.modules` probe. A line-anchored regex returns 74 for the
urllib-or-requests row, missing four files that import comma-form (`import re, html, sys,
urllib.request`), and a shard arguing for measurement discipline should not carry one figure from the
weaker method.

**Two of these rows are outcomes rather than structure, so they have a side.** Rows 1, 3 and 4 are
pre-rename and all three are zero afterwards: "killed" and "degraded" exist only while the shadow
does. Measured in two isolated temp directories rather than by re-breaking the live one — with a
shadowing `secrets.py` present the unguarded script exits 1 and the guarded one still reaches its
end; with none present both exit 0. The rest are structural counts the rename does not touch, and the
last row is post-rename.

**That network row is a negative control, not another casualty.** Measured in a clean interpreter:
after `import urllib.request`, and after `import requests`, `secrets` is absent from `sys.modules`;
after `import pypdf` it is present. The controls fired — a bare interpreter reports it absent, one
that imported `secrets` reports it present. And none of those 78 scripts imports `pypdf`, so the
overlap is zero. Blast radius is not "scripts that do I/O", it is "scripts that transitively reach
the shadowed name", and nothing but measurement separates the two.

The rename was the entire fix — `secrets.py` to `scan_for_secrets_in_lessons.py`. A `pypdf` script
that had failed twice then ran clean: seven spans, each occurring exactly once, zero misbehaving
experiments.

**The failure is invisible in three ways at once.** The traceback's last frame names *my* file, which
looks like the culprit but is only the victim of the name. The shadowed module is imported by a
dependency, so nothing I wrote mentions it. And the blast radius is every script in the directory
that *transitively* reaches the shadowed name — which cannot be known by reading any one of them.

**And the fifth script is the sharpest case, because it did not fail.** Checked by AST rather than by
eye: four of the five import `pypdf` unguarded at module top level and die. The fifth wraps its
import in `try:` / `except Exception`, so it caught the `FileNotFoundError`, printed a failure line,
and carried on — swallowing the evidence entirely. One badly named scratch file disabled four
instruments outright and silently degraded a fifth, whenever any of them was run from that directory;
run from anywhere else, all five are fine. I found out only because one of the four refused to run.

**Checking for it is one line, and it should use the interpreter's own list rather than a remembered
set of module names:**

    [f for f in os.listdir(".") if f.endswith(".py")
     and f[:-3] in sys.stdlib_module_names]

`sys.stdlib_module_names` is authoritative and versioned; a hand-kept list of "names to avoid" is the
same species of defect as the file it is trying to catch. Confirm the check can fire before trusting
a zero: `"os" in sys.stdlib_module_names` is True, `"zzz"` is False.

**The check has a blind spot, and it is the same defect one level up.** Measured on Python 3.10.11
(303 names): `test` is importable — `import test` reaches CPython's own test package — but is
deliberately absent from `sys.stdlib_module_names`. So `test.py` shadows a real module and **this
one-liner will not flag it**. Membership is also version-dependent: `parser` was removed from the
standard library in 3.10, so here it is neither listed nor importable and `parser.py` shadows
nothing. State the version you checked against; a list of names is a claim with an expiry date.

**Names a scratch file is likely to want, which are stdlib modules here:** `secrets`, `types`,
`token`, `code`, `stat`, `copy`, `queue`, `select`, `signal`, `string`, `time` — all eleven verified
present in `sys.stdlib_module_names` on 3.10.11. That is what they are, not a ranking of what is
likeliest; this session observed exactly one collision.

Related: [[a-checking-probe-that-cries-wolf-trains-you-to-discount-it]] — that shard is about probes
that report a defect which is not there; this is the other failure mode, an instrument that does not
report at all. A crashing probe is at least honest, but only if you read the traceback past its last
frame.

Related: [[an-instrument-that-enforces-a-convention-nobody-wrote-down-manufactures-defects]] — there
the instrument accused correct prose; here the environment disabled correct instruments. Both are
cases where the tooling, not the subject, produced the finding.
