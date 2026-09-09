"""Verify the two NCBI `<pre>` citations against the live page.

WHY THIS FILE IS IN THE REPOSITORY. Installment 4h's headers in
`biology/genetic-code.adj` and `biology/start-codon.adj` claim their `source`
fields are byte-exact spans of NCBI's genetic-code page. Round 7 of security
review added a sentence to both headers crediting a verification script -- and
round 8 found the script lived only in the author's scratchpad, so no reader of
this repository could locate or run the instrument the headers cite. A citation
to an unreachable instrument is the same defect class this installment exists
to remove, so the instrument moved into the tree.

It also answers, for these two values, the complaint recorded on #14444: that
the contiguity screens are working-tree instruments and every figure the
changelog publishes is reported rather than reproducible. This one is
reproducible, needs network access and nothing else -- no third-party packages:

    python code/specs/data/adj-facts-stdlib/tools/verify_ncbi_pre_span.py

WHAT IT CHECKS.

  1. SUBSTRING       the field is a byte-exact substring of the page with tags
                     stripped and entities unescaped.
  2. UNIQUE          the span occurs exactly once, so "the page contains this
                     text" names one place.
  3. LINE-ANCHORED   the character before the match is a newline.
  4. COLUMN-ALIGNED  the block has five lines and all five `= ` data columns
                     land at the same offset.
  5. DECODES         `atg` read by raw column index gives `M` on the `AAs` line
                     (what `genetic-code.adj` reads) and on the `Starts` line
                     (what `start-codon.adj` reads).

WHICH OF THESE ACTUALLY DISCRIMINATE, measured rather than asserted. The
defective first attempt at 4h's repair restored eight of the twelve stripped
spaces and left line 1 flush. Running the whole battery on it:

    variant         substring  unique  anchored  columns   AAs atg  Starts atg
    shipped         True       1       True      [11]      M        M
    first attempt   True       1       False     [7, 11]   T        M

So (1) and (2) hold for BOTH and discriminate nothing; (3), (4) and (5) each
separate them. Two earlier rounds got this wrong in opposite directions --
round 7 credited uniqueness with discriminating power it does not have, and
round 8 over-corrected by naming line-anchoring as THE single separator when
column alignment and the `AAs` decode separate them too. The `controls()`
function below runs the full battery on the reconstructed first attempt and
requires exactly the properties above to fail, so this docstring cannot drift
from what the code demonstrates.

CONTROLS. A screen that only looks for what it expects passes on a file where
nothing was fixed. So: a one-letter alteration must not be found anywhere, and
the reconstructed first attempt must pass (1) and (2) while failing (3), (4)
and (5). If any control misbehaves the script reports that its verdict proves
nothing and exits non-zero -- distinctly from a property failure, because
"the artifact is wrong" and "the instrument is broken" are different news.
"""
import html
import os
import re
import sys
import urllib.error
import urllib.request

URL = "https://www.ncbi.nlm.nih.gov/Taxonomy/Utils/wprintgc.cgi"
LIBS = ["biology/genetic-code.adj", "biology/start-codon.adj"]
BS = chr(92)


def stdlib_dir():
    """The adj-facts-stdlib directory, from this file's own location."""
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
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    m = re.search(r'^\s*source "(.*)"\s*$', text, re.MULTILINE)
    if not m:
        raise SystemExit("!!! no source field in " + path)
    return unquote(m.group(1))


def fetch_flat(url):
    req = urllib.request.Request(url, headers={"User-Agent": "adj-facts-stdlib/verify"})
    with urllib.request.urlopen(req, timeout=60) as response:
        raw = response.read().decode("utf-8", "replace")
    return html.unescape(re.sub(r"<[^>]+>", "", raw))


def decode(value, line_index, codon):
    """Read `codon` off the given line by RAW COLUMN INDEX."""
    lines = value.split("\n")
    if len(lines) < 5:
        return "?"
    target, base1, base2, base3 = lines[line_index], lines[2], lines[3], lines[4]
    for n in range(min(len(target), len(base1), len(base2), len(base3))):
        if (base1[n] + base2[n] + base3[n]).lower() == codon:
            return target[n]
    return "?"


def properties(value, page):
    """The five checks, as booleans, for any candidate value."""
    at = page.find(value)
    lines = value.split("\n")
    # `str.find` returns -1 on a miss, which would silently contribute a bogus
    # offset of 1; require five lines that each actually carry a `= ` column
    # before comparing, or (4) can print a PASS for a one-line value.
    shaped = len(lines) == 5 and all("= " in ln for ln in lines)
    offsets = sorted({ln.find("= ") + 2 for ln in lines}) if shaped else []
    return {
        "1 substring": at >= 0,
        "2 unique": page.count(value) == 1,
        "3 line-anchored": at >= 0 and (at == 0 or page[at - 1] == "\n"),
        "4 column-aligned": shaped and len(offsets) == 1,
        "5 decodes": decode(value, 0, "atg") == "M" and decode(value, 1, "atg") == "M",
    }


def controls(value, page, report):
    """Every arm gets a case that must fail. Returns True if all behaved."""
    ok = True

    altered = value.replace("Starts", "Startz", 1)
    if altered in page:
        report(False, "control: a one-letter alteration must NOT be found")
        ok = False
    else:
        report(True, "control: a one-letter alteration is not found")

    if not value.startswith("    "):
        report(False, "control: value no longer opens with the four-space "
                      "indent, so the first-attempt control cannot be built")
        return False

    # The defective first attempt: line 1 flush, interior indents intact.
    first = properties(value[4:], page)
    expected = {"1 substring": True, "2 unique": True, "3 line-anchored": False,
                "4 column-aligned": False, "5 decodes": False}
    for key, want in expected.items():
        got = first[key]
        label = ("control: the first attempt " + ("passes " if want else "FAILS ")
                 + key)
        if got != want:
            report(False, label + "  -- but it " + ("failed" if want else "passed"))
            ok = False
        else:
            report(True, label)
    return ok


def main():
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    try:
        page = fetch_flat(URL)
    except (urllib.error.URLError, TimeoutError) as exc:
        print("COULD NOT FETCH " + URL + ": " + str(exc))
        print("No verdict. This is not a pass and not a failure of the data.")
        return 2
    if len(page) < 10000:
        print("!!! page came back only " + str(len(page)) + " characters")
        print("No verdict -- refusing to verify against a partial fetch.")
        return 2

    bad_props, bad_controls = [], []

    for rel in LIBS:
        name = rel.split("/")[-1]
        print(name)
        value = source_field(os.path.join(stdlib_dir(), *rel.split("/")))

        for key, ok in properties(value, page).items():
            print(("  PASS  " if ok else "  FAIL  ") + name + ": (" + key + ")")
            if not ok:
                bad_props.append(name + ": " + key)

        def report(ok, label, _name=name):
            print(("  ctrl  " if ok else "  CTRL  ") + _name + ": " + label)
            if not ok:
                bad_controls.append(_name + ": " + label)

        controls(value, page, report)
        print()

    if bad_controls:
        print("THE INSTRUMENT IS BROKEN: " + str(len(bad_controls))
              + " control(s) misbehaved, so this run's verdict proves nothing.")
        for entry in bad_controls:
            print("   " + entry)
        return 3
    if bad_props:
        print("VERIFICATION FAILED on " + str(len(bad_props)) + " property check(s):")
        for entry in bad_props:
            print("   " + entry)
        return 1
    print("every checked property holds, and every control behaved.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
