"""Verify the two NCBI `<pre>` citations against the live page.

WHY THIS FILE IS IN THE REPOSITORY. Installment 4h's headers in
`biology/genetic-code.adj` and `biology/start-codon.adj` claim their `source`
fields are byte-exact spans of NCBI's genetic-code page. Round 7 of security
review added a sentence to both headers crediting a verification script -- and
round 8 found that the script lived only in the author's scratchpad, so no
reader of this repository could locate or run the instrument the headers cite.
A citation to an unreachable instrument is the same defect class this
installment exists to remove, so the instrument moved into the tree.

It also answers, for these two values, the complaint recorded on #14444: that
the contiguity screens are working-tree instruments and every figure the
changelog publishes is reported rather than reproducible. This one is
reproducible:

    python code/specs/data/adj-facts-stdlib/tools/verify_ncbi_pre_span.py

It needs network access and nothing else -- no third-party packages, and none
of the author's scratchpad helpers.

WHAT IT CHECKS, and why each property is here rather than another.

  1. SUBSTRING.  The field is a byte-exact substring of the page with tags
     stripped and entities unescaped. Necessary, and NOT sufficient -- see (3).

  2. UNIQUE.  The span occurs exactly once, so "the page contains this text"
     names one place. Asserted for its own sake. It is NOT a discriminator:
     round 8 measured that the defective first attempt was also unique, and
     round 7 had wrongly claimed uniqueness separated the two.

  3. LINE-ANCHORED.  The character before the match is a newline. THIS is the
     property that separates the shipped value from the defective one. The
     first attempt at this repair restored eight of the twelve stripped spaces
     and left line 1 flush; the resulting value was still a byte-exact, unique
     substring -- its span simply began four characters into line 1, so the
     excerpt stopped standing on its own columns. A substring test passes it.

  4. COLUMN-ALIGNED.  All five `= ` data columns land at the same offset, which
     is what makes the block decodable by column at all.

  5. DECODES.  `atg` read by raw column index gives `M` on the `AAs` line (what
     `genetic-code.adj` reads) and `M` on the `Starts` line (what
     `start-codon.adj` reads). The pre-repair bytes gave `T` on `AAs`.

NEGATIVE CONTROLS. A screen that only looks for what it expects passes on a
file where nothing was fixed, so each arm is paired with a case that must FAIL:
a one-letter alteration must not be found, and the first attempt's span --
reconstructed here by removing line 1's four spaces -- must fail (3) while
still passing (1) and (2). If a control does not behave, this script says its
own verdict is meaningless and exits non-zero rather than reporting a pass.
"""
import html
import re
import sys
import urllib.request

URL = "https://www.ncbi.nlm.nih.gov/Taxonomy/Utils/wprintgc.cgi"
LIBS = ["biology/genetic-code.adj", "biology/start-codon.adj"]
BS = chr(92)


def stdlib_dir():
    """The adj-facts-stdlib directory, from this file's own location."""
    import os
    return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def unquote(s):
    """Undo the .adj string escaping (\\n, \\t, \\", \\\\)."""
    out, i = [], 0
    while i < len(s):
        if s[i] == BS and i + 1 < len(s):
            out.append({"n": "\n", "t": "\t"}.get(s[i + 1], s[i + 1]))
            i += 2
            continue
        out.append(s[i])
        i += 1
    return "".join(out)


def source_field(path):
    import io
    text = io.open(path, encoding="utf-8").read()
    m = re.search(r'^\s*source "(.*)"\s*$', text, re.M)
    if not m:
        raise SystemExit("!!! no source field in " + path)
    return unquote(m.group(1))


def fetch_flat(url):
    req = urllib.request.Request(url, headers={"User-Agent": "adj-facts-stdlib/verify"})
    with urllib.request.urlopen(req, timeout=60) as r:
        raw = r.read().decode("utf-8", "replace")
    return html.unescape(re.sub(r"<[^>]+>", "", raw))


def decode(value, line_index, codon):
    """Read `codon` off the given line by RAW COLUMN INDEX."""
    lines = value.split("\n")
    if len(lines) < 5:
        return "?"
    target, b1, b2, b3 = lines[line_index], lines[2], lines[3], lines[4]
    for n in range(min(len(target), len(b1), len(b2), len(b3))):
        if (b1[n] + b2[n] + b3[n]).lower() == codon:
            return target[n]
    return "?"


def main():
    import os
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    flat = fetch_flat(URL)
    if len(flat) < 10000:
        raise SystemExit("!!! page came back only " + str(len(flat))
                         + " characters -- refusing to verify against it")

    failures = []

    def check(ok, label, detail=""):
        print(("  PASS  " if ok else "  FAIL  ") + label
              + ("   " + detail if detail else ""))
        if not ok:
            failures.append(label)

    for rel in LIBS:
        path = os.path.join(stdlib_dir(), *rel.split("/"))
        v = source_field(path)
        name = rel.split("/")[-1]
        print(name)

        at = flat.find(v)
        check(at >= 0, name + ": (1) byte-exact substring of the page")
        if at < 0:
            continue
        check(flat.count(v) == 1, name + ": (2) occurs exactly once",
              str(flat.count(v)) + " occurrence(s) -- asserted, NOT a discriminator")
        check(at == 0 or flat[at - 1] == "\n",
              name + ": (3) match begins at a line boundary",
              "preceded by " + repr(flat[at - 1] if at else "<start>"))

        offsets = sorted({ln.find("= ") + 2 for ln in v.split("\n")})
        check(len(offsets) == 1, name + ": (4) all five data columns aligned",
              "offsets " + str(offsets))
        check(decode(v, 0, "atg") == "M", name + ": (5) atg -> M on the AAs line",
              "got " + decode(v, 0, "atg"))
        check(decode(v, 1, "atg") == "M", name + ": (5) atg -> M on the Starts line",
              "got " + decode(v, 1, "atg"))

        # --- negative controls -------------------------------------------
        altered = v.replace("Starts", "Startz", 1)
        check(altered not in flat,
              name + ": control -- a one-letter alteration is NOT found")

        if not v.startswith("    "):
            check(False, name + ": control -- value no longer starts with the "
                                "four-space indent, so the line-anchor control "
                                "cannot be built")
            continue
        first_attempt = v[4:]
        fa = flat.find(first_attempt)
        check(fa >= 0 and flat.count(first_attempt) == 1
              and fa > 0 and flat[fa - 1] != "\n",
              name + ": control -- the first attempt's span IS a unique "
                     "substring but is NOT line-anchored",
              "this is why (1) and (2) cannot discriminate and (3) can")
        print()

    if failures:
        print("VERIFICATION FAILED on " + str(len(failures)) + " check(s):")
        for f in failures:
            print("   " + f)
        return 1
    print("every checked property holds, and every control behaved.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
