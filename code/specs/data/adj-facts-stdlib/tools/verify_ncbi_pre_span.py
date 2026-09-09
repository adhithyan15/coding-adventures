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
separate them. Two rounds got this wrong in opposite directions -- round 7
credited uniqueness with power it does not have, and round 8 over-corrected by
naming line-anchoring as THE single separator. `controls()` runs the full
battery on a reconstructed first attempt and asserts exactly which arms must
fail, so this docstring reports a test that runs rather than a claim beside one.

THE CONTROLS ARE BUILT FROM THE PAGE, NOT FROM THE FIELD, and that is the whole
design. An earlier version sliced the control out of the value under test, so
any defect that broke a property broke the control too -- and because `main()`
weighed controls first, TWELVE OF THIRTEEN defect shapes, including the exact
round-1 defect this instrument was written to catch, announced "the instrument
is broken, this verdict proves nothing" instead of "the artifact is wrong".
That is the confusion the exit codes exist to prevent, in the more dangerous
direction: it invites a reader to discount a real regression. Reconstructing
the control from the page makes control failure mean what it says -- the page
moved, or this script is wrong -- independent of whatever the field contains.

EXIT CODES. 0 every property holds and every control behaved. 1 the ARTIFACT is
wrong -- a property failed. 2 no verdict (the page could not be fetched, or came
back truncated). 3 the INSTRUMENT is broken -- a control misbehaved, so nothing
this run says about the artifact can be trusted.
"""
import html
import os
import re
import sys
import urllib.error
import urllib.request

URL = "https://www.ncbi.nlm.nih.gov/Taxonomy/Utils/wprintgc.cgi"
LIBS = ["biology/genetic-code.adj", "biology/start-codon.adj"]
MAX_BYTES = 8 * 1024 * 1024
MIN_PAGE = 10000
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


def source_fields(path):
    """Every `source` field in a file, so a second table cannot go unnoticed."""
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    found = re.findall(r'^\s*source "(.*)"\s*$', text, re.MULTILINE)
    if len(found) != 1:
        raise SystemExit("!!! expected exactly one source field in " + path
                         + ", found " + str(len(found)))
    return unquote(found[0])


def fetch_flat(url):
    req = urllib.request.Request(url, headers={"User-Agent": "adj-facts-stdlib/verify"})
    with urllib.request.urlopen(req, timeout=60) as response:
        body = response.read(MAX_BYTES)
    if len(body) >= MAX_BYTES:
        # A read that hit the cap may have stopped mid-document. The docstring
        # promised exit 2 "or came back truncated" while only the 10k floor
        # could ever fire; this is the arm that makes that true.
        return None
    return html.unescape(re.sub(r"<[^>]+>", "", body.decode("utf-8", "replace")))


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


def canonical_block(page):
    """The five-line block AS THE PAGE HAS IT, sliced from the page itself.

    Independent of any `.adj` field, so the controls built from it keep working
    when a field is broken -- which is precisely when their verdict matters.
    """
    marker = page.find("AAs  = FFLL")
    if marker < 0:
        return None
    start = page.rfind("\n", 0, marker) + 1
    lines = page[start:].split("\n")[:5]
    if len(lines) != 5:
        return None
    return "\n".join(lines)


def controls(page, report):
    """Cases that MUST fail. Returns True only if every one behaved."""
    ok = True
    block = canonical_block(page)
    if block is None:
        report(False, "control: the canonical five-line block is not locatable "
                      "on the page -- the page changed shape")
        return False

    # The page's own block must pass everything. If it does not, this script's
    # idea of the block is wrong, not the repository's.
    for key, passed in properties(block, page).items():
        if not passed:
            report(False, "control: the PAGE'S OWN block fails " + key)
            ok = False
    if ok:
        report(True, "control: the page's own block passes all five arms")

    # A one-letter alteration must fail arms 1 and 2 -- demonstrated through
    # `properties`, not merely asserted, so arm 2 has a must-fail case too.
    altered = properties(block.replace("Starts", "Startz", 1), page)
    if altered["1 substring"] or altered["2 unique"]:
        report(False, "control: a one-letter alteration still passes "
                      "substring/uniqueness")
        ok = False
    else:
        report(True, "control: a one-letter alteration fails 1 substring and 2 unique")

    # The defective first attempt: line 1 flush, interior indents intact.
    if not block.startswith("    "):
        report(False, "control: the page's block no longer opens with four "
                      "spaces, so the first-attempt control cannot be built")
        return False
    first = properties(block[4:], page)
    expected = {"1 substring": True, "2 unique": True, "3 line-anchored": False,
                "4 column-aligned": False, "5 decodes": False}
    for key, want in expected.items():
        label = ("control: the first attempt "
                 + ("passes " if want else "FAILS ") + key)
        if first[key] != want:
            report(False, label + " -- but it " + ("failed" if want else "passed"))
            ok = False
        else:
            report(True, label)
    return ok


def verdict(page, root, quiet=False):
    """Route one run to an exit code. Split out of `main` so `--self-test` can
    drive it with a synthetic page and a synthetic library tree, and so the
    routing claim in the package README is checked rather than asserted."""
    def say(line):
        if not quiet:
            print(line)

    if page is None:
        say("COULD NOT FETCH " + URL)
        say("No verdict. This is not a pass and not a failure of the data.")
        return 2
    if len(page) < MIN_PAGE:
        say("!!! page came back only " + str(len(page)) + " characters")
        say("No verdict -- refusing to verify against a partial fetch.")
        return 2

    bad_controls = []

    def report(ok, label):
        say(("  ctrl  " if ok else "  CTRL  ") + label)
        if not ok:
            bad_controls.append(label)

    say("controls (built from the page, not from the fields)")
    controls(page, report)
    say("")

    bad_props = []
    for rel in LIBS:
        name = rel.split("/")[-1]
        say(name)
        try:
            value = source_fields(os.path.join(root, *rel.split("/")))
        except SystemExit as exc:
            say("  FAIL  " + name + ": " + str(exc))
            bad_props.append(name + ": unreadable source field")
            continue
        for key, ok in properties(value, page).items():
            say(("  PASS  " if ok else "  FAIL  ") + name + ": (" + key + ")")
            if not ok:
                bad_props.append(name + ": " + key)
        say("")

    if bad_controls:
        say("THE INSTRUMENT IS BROKEN: " + str(len(bad_controls))
            + " control(s) misbehaved, so this run's verdict proves nothing.")
        for entry in bad_controls:
            say("   " + entry)
        return 3
    if bad_props:
        say("THE ARTIFACT IS WRONG: " + str(len(bad_props))
            + " property check(s) failed.")
        for entry in bad_props:
            say("   " + entry)
        return 1
    say("every checked property holds, and every control behaved.")
    return 0


def self_test(page):
    """Prove the four exit codes are produced by the right causes.

    WHY THIS EXISTS. The package README claimed the routing "is checked by
    mutating a copy of the libraries six ways". After the controls were moved
    off the fields and onto the page, NO library mutation can reach exit 2 or 3
    -- so that check exercised two of four codes and could not touch the 1-vs-3
    edge it was cited for. A check that cannot fail on the thing it certifies
    is the defect class this whole installment is about, so the check moved
    into the script where anyone can run it:

        python .../verify_ncbi_pre_span.py --self-test

    Library mutations must reach 1, page mutations 3, fetch problems 2, and the
    untouched pair 0. Each case names WHICH input it perturbs, because the
    routing's whole point is telling those apart.
    """
    import shutil
    import tempfile

    ok = True
    results = []

    with tempfile.TemporaryDirectory() as tmp:
        root = os.path.join(tmp, "stdlib")
        for rel in LIBS:
            dest = os.path.join(root, *rel.split("/"))
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            shutil.copyfile(os.path.join(stdlib_dir(), *rel.split("/")), dest)
        target = os.path.join(root, *LIBS[0].split("/"))
        with open(target, encoding="utf-8") as handle:
            pristine = handle.read()

        def run(label, perturbs, want, text=None, use_page=page):
            with open(target, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(pristine if text is None else text)
            got = verdict(use_page, root, quiet=True)
            results.append((label, perturbs, want, got))
            return got == want

        cases = [
            ("nothing perturbed", "neither", 0, None, page),
            ("interior indent stripped", "library", 1,
             pristine.replace(BS + "n  ", BS + "n"), page),
            ("line 1 anchor stripped", "library", 1,
             pristine.replace('source "    ', 'source "', 1), page),
            ("one data letter changed", "library", 1,
             pristine.replace("FFLL", "FFLZ", 1), page),
            ("a second source field added", "library", 1,
             pristine.replace('    locator "', '    source "x"\n    locator "', 1), page),
            ("block marker gone from the page", "page", 3, None,
             page.replace("AAs  = FFLL", "AAs  = ZZZZ")),
            ("page's line-1 indent removed", "page", 3, None,
             page.replace("\n    AAs  = FFLL", "\nAAs  = FFLL", 1)),
            ("page truncated", "fetch", 2, None, page[:500]),
            ("page unreachable", "fetch", 2, None, None),
        ]
        for label, perturbs, want, text, use in cases:
            if not run(label, perturbs, want, text, use):
                ok = False

    print("SELF-TEST -- which input produces which exit code")
    print("  " + "perturbation".ljust(34) + "input".ljust(9)
          + "want".ljust(5) + "got")
    for label, perturbs, want, got in results:
        flag = "" if want == got else "   <-- MISROUTED"
        print("  " + label.ljust(34) + perturbs.ljust(9)
              + str(want).ljust(5) + str(got) + flag)
    reached = sorted({got for _, _, _, got in results})
    print()
    print("  exit codes reached: " + str(reached))
    if reached != [0, 1, 2, 3]:
        print("  *** the self-test does not exercise all four codes, so it "
              "cannot certify the routing")
        ok = False
    print("  " + ("PASS -- every code produced by its own cause"
                  if ok else "*** FAIL"))
    return 0 if ok else 4


def main():
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    want_self_test = "--self-test" in sys.argv[1:]
    try:
        page = fetch_flat(URL)
    except (urllib.error.URLError, TimeoutError) as exc:
        if want_self_test:
            print("--self-test needs the page once to build its cases: " + str(exc))
            return 2
        print("COULD NOT FETCH " + URL + ": " + str(exc))
        print("No verdict. This is not a pass and not a failure of the data.")
        return 2
    if want_self_test:
        return self_test(page)
    return verdict(page, stdlib_dir())


if __name__ == "__main__":
    sys.exit(main())
