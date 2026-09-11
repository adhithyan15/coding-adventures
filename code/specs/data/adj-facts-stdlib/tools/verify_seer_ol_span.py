"""Verify `biology/mitosis-phase-order`'s citation against the page it names.

WHY THIS EXISTS. Issue #14444 records that this package's contiguity screens
were working-tree instruments: they ran once, in a scratch directory, and left
nothing anyone else could re-run. A provenance claim that only its author can
check is not much of a claim. This is the second answer to that (the first is
`verify_ncbi_pre_span.py`), for a different and sharper case.

WHAT IT CHECKS. `mitosis-phase-order` cites the NCI SEER "Cell Cycle" page for
the ORDER of the four phases of mitosis. The page asserts that order, but it
asserts it as an ORDERED LIST -- `<ol class="usa-list">` -- and not in any
sentence. The `source` field therefore holds the page's lead-in paragraph,
"The four phases of mitosis are", quoted contiguously and nothing more, with
the list named in the file's header as the origin of the ordering.

The field used to hold, instead:

    The four phases of mitosis are Prophase ... Metaphase ... Anaphase ... Telophase.

which occurs nowhere on the page. Three things were invented: the " ... "
separator (zero occurrences on the page), the terminal period (the paragraph
ends at "are"), and the bare word "Telophase" (the page's fourth item reads
"Telophase (divided into parts I and II)"). This script exists so that anyone
can confirm all of that, and so the repair cannot silently rot.

EXIT CODES, which are the point of the script -- a checker that cannot
distinguish "the artifact is wrong" from "I could not tell" is worse than no
checker, because it launders one into the other:

    0  every arm holds
    1  the ARTIFACT is wrong -- a shipped value is not on the page
    2  NO VERDICT -- the page could not be fetched, or has changed shape
    3  the INSTRUMENT is broken -- its own controls failed

Run `--self-test` to exercise the routing offline: it perturbs each input in a
way that must produce a given exit code, and fails if any of the four codes is
never reached. Controls are built FROM THE PAGE, never sliced from the value
under test; an earlier instrument in this effort cut its controls from the
artifact and consequently reported "instrument broken" for twelve of thirteen
real defects.
"""

import argparse
import html
import http.client
import os
import re
import shutil
import sys
import urllib.error
import urllib.request

URL = "https://training.seer.cancer.gov/disease/cancer/biology/cycle.html"
ADJ = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                   "..", "biology", "mitosis-phase-order.adj")

# The paragraph the artifact quotes. Not a sentence -- a lead-in to the list.
LEADIN = "The four phases of mitosis are"

# A DIFFERENT real span of the SAME page, shipped by the sibling library. It is
# the positive control: an instrument that cannot confirm a value known to be
# byte-exact is not measuring byte-exactness, whatever it says about anything
# else.
CONTROL = ("Chromatin is transformed into chromosomes composed of pairs of filaments "
           "called chromatids (each is a complete genetic copy of its chromosome).")

# What the four top-level list items must read, in document order.
ITEMS = ["Prophase", "Metaphase", "Anaphase", "Telophase (divided into parts I and II)"]

OK, ARTIFACT_WRONG, NO_VERDICT, INSTRUMENT_BROKEN = 0, 1, 2, 3


def strip_tags(raw):
    """Tags out, entities decoded, non-breaking spaces normalised."""
    t = re.sub(r"(?is)<(script|style)[^>]*>.*?</\1>", " ", raw)
    t = re.sub(r"(?s)<[^>]+>", "\n", t)
    return html.unescape(t).replace("\xa0", " ")


def fetch(url=URL, timeout=60):
    req = urllib.request.Request(
        url, headers={"User-Agent": "Mozilla/5.0 (adj-facts-stdlib verifier)"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read().decode("utf-8", "replace")


def shipped_source(path=ADJ):
    """The `source` value as the artifact ships it. Comment lines are skipped:
    the header deliberately QUOTES the old fabricated string while disclosing
    it, and a reader that cannot tell a disclosure from a claim would forbid
    the very disclosure this repair exists to make."""
    with open(path, encoding="utf-8") as fh:
        for ln in fh.read().split("\n"):
            if ln.lstrip().startswith("%"):
                continue
            m = re.match(r'\s*source\s+"(.*)"\s*$', ln)
            if m:
                return m.group(1)
    return None


def top_level_items(raw):
    """The four top-level <li> labels of the <ol> that follows the lead-in.

    Returns None if the page's shape is not what the artifact assumes -- that
    is a NO VERDICT, not a defect: a redesigned page tells us nothing about
    whether the recorded quotation was honest when it was recorded.
    """
    at = raw.find(LEADIN)
    if at < 0:
        return None
    rest = raw[at:]
    ol = rest.find("<ol")
    if ol < 0:
        return None
    # Top-level items are the ones indented one tab; nested sub-lists are
    # deeper. Matching on that indentation is what keeps the sub-list events
    # (which are also <li>) out of the answer.
    return [t.strip() for t in re.findall(r"\n\t<li>([^\n<]*)", rest[ol:])]


def check(raw, source):
    """(exit code, list of report lines). Pure: no I/O, so --self-test can
    drive it with perturbed inputs and assert the routing."""
    rep = []
    text = strip_tags(raw)
    coll = " ".join(text.split())

    def present(v):
        if raw.count(v):
            return "byte-exact in raw HTML"
        if text.count(v):
            return "byte-exact in stripped text"
        if coll.count(" ".join(v.split())):
            return "collapsed-whitespace match only"
        return None

    # --- controls first ----------------------------------------------------
    if not present(CONTROL):
        rep.append("CONTROL FAILED: a value known to be a real span of this page "
                   "was not found. The page may have changed, or the instrument "
                   "may be mis-reading it. No verdict is possible.")
        return NO_VERDICT, rep
    if present(CONTROL[:-2] + "X."):
        rep.append("CONTROL FAILED: a deliberately corrupted value was reported "
                   "present. The instrument is not measuring anything.")
        return INSTRUMENT_BROKEN, rep
    rep.append("controls   ok  (a real span confirms; its corruption does not)")

    # --- the page must still have the shape the artifact assumes ----------
    items = top_level_items(raw)
    if items is None:
        rep.append("NO VERDICT: the lead-in paragraph or its <ol> is no longer "
                   "on the page in the expected shape.")
        return NO_VERDICT, rep
    if items != ITEMS:
        rep.append("NO VERDICT: the ordered list's top-level items have changed.")
        rep.append("  expected " + repr(ITEMS))
        rep.append("  found    " + repr(items))
        return NO_VERDICT, rep
    rep.append("list       ok  four top-level <li> items, in order: "
               + ", ".join(items))

    # --- the artifact ------------------------------------------------------
    if source is None:
        rep.append("NO VERDICT: no `source` line was found in the .adj file.")
        return NO_VERDICT, rep
    where = present(source)
    if where is None:
        rep.append("ARTIFACT WRONG: the shipped `source` is not on the page.")
        rep.append("  " + repr(source))
        return ARTIFACT_WRONG, rep
    if source != LEADIN:
        rep.append("ARTIFACT WRONG: the shipped `source` is on the page, but it "
                   "is not the lead-in paragraph this file is supposed to quote.")
        rep.append("  shipped  " + repr(source))
        rep.append("  expected " + repr(LEADIN))
        return ARTIFACT_WRONG, rep
    rep.append("source     ok  " + repr(source) + "  (" + where + ")")

    # --- and the fabrications must stay gone -------------------------------
    if " ... " in raw or " ... " in text:
        rep.append("NO VERDICT: the page now contains ' ... ', which this check "
                   "assumed it never did.")
        return NO_VERDICT, rep
    rep.append("no ' ... ' anywhere on the page: the separator the old value "
               "carried was never the page's own")
    return OK, rep


def self_test():
    """Drive `check` with perturbed inputs and assert the exit-code routing.

    Fails if any of the four codes is unreachable -- a router with a dead
    branch has an untested claim in it.
    """
    page = ("<p>" + LEADIN + "</p>\n<ol class=\"usa-list\">\n"
            + "".join("\t<li>" + i + "\n\t\t<ol type=\"a\"><li>x</li></ol>\n\t</li>\n"
                      for i in ITEMS)
            + "</ol>\n<p>" + CONTROL + "</p>\n")
    cases = [
        ("unperturbed", page, LEADIN, OK),
        ("source not on the page", page, "A sentence the page does not carry.",
         ARTIFACT_WRONG),
        ("the original fabricated composite", page,
         LEADIN + " Prophase ... Metaphase ... Anaphase ... Telophase.", ARTIFACT_WRONG),
        ("a real span, but the wrong one", page, CONTROL, ARTIFACT_WRONG),
        ("no source line at all", page, None, NO_VERDICT),
        ("the list's items changed", page.replace("Anaphase", "Anaphase II"), LEADIN,
         NO_VERDICT),
        ("the lead-in gone", page.replace(LEADIN, "The phases are"), LEADIN, NO_VERDICT),
        ("the control span gone", page.replace(CONTROL, "gone"), LEADIN, NO_VERDICT),
    ]
    seen, bad = set(), 0
    for label, raw, src, want in cases:
        got, _ = check(raw, src)
        seen.add(got)
        mark = "ok  " if got == want else "FAIL"
        if got != want:
            bad += 1
        print("  " + mark + "  exit " + str(got) + " (want " + str(want) + ")  " + label)
    # INSTRUMENT_BROKEN needs a page where a corrupted value IS found, which
    # only happens if the page literally contains the corruption.
    got, _ = check(page + CONTROL[:-2] + "X.", LEADIN)
    seen.add(got)
    print("  " + ("ok  " if got == INSTRUMENT_BROKEN else "FAIL")
          + "  exit " + str(got) + " (want " + str(INSTRUMENT_BROKEN)
          + ")  a page carrying the corrupted control")
    if got != INSTRUMENT_BROKEN:
        bad += 1
    # main()'s read arms are not reachable from check(), so they get their
    # own arm: shell this script against files that cannot be read and
    # require exit 2. This is the case that would have caught round 1's
    # asymmetric fix, and it needs no network because the local read now
    # happens before the fetch.
    # Each case asserts the STDOUT its branch is supposed to print, not just
    # the exit code. Two reasons, both found by review:
    #   * CPython itself exits 2 for "can't open file", so an arm that only
    #     checked the number passed even when the subprocess never ran;
    #   * the empty-.adj case was reaching 2 through the FETCH arm, never
    #     touching the read arm it was supposed to cover.
    # A checker's own tests are the last place to assert a number and call
    # it a behaviour.
    import subprocess
    import tempfile
    tmp = tempfile.mkdtemp()
    # The fixture page must be the SYNTHETIC PAGE THIS SELF-TEST ALREADY BUILT,
    # not a stub. A stub lacks the control span, so the run stops at the control
    # gate and reaches exit 2 through "CONTROL FAILED" -- the right number for
    # the wrong reason, which is exactly the confusion these exit codes exist to
    # prevent. The stdout assertion below caught that when a stub was used here.
    page_file = os.path.join(tmp, "page.html")
    with open(page_file, "w", encoding="utf-8") as fh:
        fh.write(page)
    page_url = "file:///" + page_file.replace(os.sep, "/").lstrip("/")
    # The NON-ASCII case covers the guard that reconfigures the output streams.
    # Without it, a cp1252 pipe makes the error HANDLER raise UnicodeEncodeError
    # -- a ValueError -- which escapes; the catch-all turns that into 3, so the
    # case fails with `exit 3 (want 2)` rather than the exit 1 it would have
    # produced before the catch-all existed. Either way a read failure stops
    # reporting itself as a read failure. Review pointed out that guard was
    # asserted in a comment
    # and never exercised, which is the same criticism that produced this whole
    # arm; the child is therefore run with PYTHONIOENCODING=ascii so the guard
    # is the only thing standing between a non-ASCII path and that failure.
    cases = [
        ("a .adj that is not valid UTF-8", b"\xff\xfe source \"x\"\n",
         "not-a-url", "NO VERDICT: could not read", None),
        ("a .adj with no source line", b"% only a comment\n",
         page_url, "no `source` line was found", None),
        ("a missing .adj under a NON-ASCII path, on an ascii-only pipe",
         None, "not-a-url", "NO VERDICT: could not read", "naïve-été"),
    ]
    for label, blob, url, expect, subdir in cases:
        here = tmp if subdir is None else os.path.join(tmp, subdir)
        os.makedirs(here, exist_ok=True)
        p = os.path.join(here, re.sub(r"[^A-Za-z0-9]+", "_", label)[:40] + ".adj")
        if blob is not None:
            with open(p, "wb") as fh:
                fh.write(blob)
        env = dict(os.environ)
        if subdir is not None:
            # Force the failure mode the stream guard exists for: without it, a
            # non-ASCII path printed to an ascii-only pipe raises from the
            # handler and CPython exits 1.
            env["PYTHONIOENCODING"] = "ascii"
        proc = subprocess.run([sys.executable, os.path.abspath(__file__),
                               "--adj", p, "--url", url],
                              capture_output=True, check=False, env=env)
        text = proc.stdout.decode("utf-8", "replace")
        good = proc.returncode == NO_VERDICT and expect in text
        seen.add(proc.returncode)
        print("  " + ("ok  " if good else "FAIL") + "  exit "
              + str(proc.returncode) + " (want " + str(NO_VERDICT) + ") + said "
              + repr(expect) + "  " + label)
        if not good:
            print("        stdout: " + repr(text[:200]))
            bad += 1
    shutil.rmtree(tmp, ignore_errors=True)

    # The catch-all in main() has ONE testable property and one untestable one,
    # and it is worth being exact about which is which. Untestable: whether it
    # catches an exception nobody anticipated -- no test can provoke that by
    # construction, so it is defence-in-depth and nothing here asserts it.
    # Testable, and asserted below: that it does NOT swallow SystemExit.
    # argparse exits through SystemExit, so a catch-all that ate it would turn
    # `--help` into exit 3 and a bad flag into a wrong verdict.
    for args, want, label in (([], 0, "--help exits 0 through the catch-all"),
                              (["--no-such-flag"], 2, "a bad flag still exits 2")):
        proc = subprocess.run([sys.executable, os.path.abspath(__file__), "--help"]
                              if not args else
                              [sys.executable, os.path.abspath(__file__)] + args,
                              capture_output=True, check=False)
        ok = proc.returncode == want
        print("  " + ("ok  " if ok else "FAIL") + "  exit " + str(proc.returncode)
              + " (want " + str(want) + ")  " + label)
        if not ok:
            bad += 1

    missing = {OK, ARTIFACT_WRONG, NO_VERDICT, INSTRUMENT_BROKEN} - seen
    if missing:
        print("  FAIL  these exit codes were never reached: " + str(sorted(missing)))
        bad += 1
    print()
    print("self-test: " + ("PASSED" if not bad else str(bad) + " FAILURE(S)"))
    return 0 if not bad else INSTRUMENT_BROKEN


def main():
    # Reporting is itself a place this can fail. A piped child's stdout is
    # cp1252 on Windows, so a non-ASCII path or source would raise
    # UnicodeEncodeError FROM THE HANDLER and escape. (UnicodeEncodeError is
    # a ValueError: the same exception family as the read arms' bug, one line
    # outside the tuple widened to catch it.) Before the catch-all below
    # existed that escape reached CPython and exited 1 -- ARTIFACT WRONG for
    # a reporting failure. It would now exit 3, so this reconfigure is what
    # keeps a merely-unreadable input reporting 2 instead of 3.
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, OSError, ValueError):
            pass
    try:
        return _main()
    except SystemExit:
        raise
    except BaseException as exc:  # noqa: BLE001 -- see below
        # ANY unexpected exception is the instrument failing, not the
        # artifact being wrong. Letting one escape means CPython picks the
        # exit code, and CPython picks 1. Closing the class beats closing
        # instances of it one review round at a time.
        print("INSTRUMENT BROKEN: " + type(exc).__name__ + ": " + str(exc))
        return INSTRUMENT_BROKEN


def _main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--self-test", action="store_true",
                    help="exercise the exit-code routing offline and exit")
    ap.add_argument("--url", default=URL)
    ap.add_argument("--adj", default=ADJ)
    args = ap.parse_args()

    if args.self_test:
        return self_test()

    # EVERY failure to READ routes to NO VERDICT, never to ARTIFACT_WRONG.
    # Two paths used to break that promise, and both exited 1 -- which this
    # script defines as "the artifact is wrong":
    #   * a malformed --url raises ValueError from Request(), not URLError;
    #   * shipped_source() was called outside the try, so a missing .adj
    #     raised FileNotFoundError and escaped.
    # Reporting an unreadable input as a provenance defect is exactly what
    # the four exit codes exist to prevent.
    # THE LOCAL READ COMES FIRST, deliberately: it is cheap, it needs no
    # network, and putting it here is what lets --self-test shell this
    # script against a deliberately unreadable file and assert the routing,
    # rather than asserting it in a comment. Round 1 fixed this arm and
    # wrote `except OSError` while widening the fetch arm's tuple in the
    # same edit -- and UnicodeDecodeError is a ValueError, NOT an OSError,
    # so a BOM'd or truncated .adj still exited 1. Both arms now carry the
    # same tuple, because they are answering the same question: could this
    # input be READ at all?
    try:
        source = shipped_source(args.adj)
    except (OSError, ValueError) as exc:
        print("NO VERDICT: could not read " + args.adj)
        print("  " + type(exc).__name__ + ": " + str(exc))
        return NO_VERDICT

    try:
        raw = fetch(args.url)
    except (OSError, http.client.HTTPException, urllib.error.URLError,
            ValueError) as exc:
        print("NO VERDICT: could not fetch " + args.url)
        print("  " + type(exc).__name__ + ": " + str(exc))
        return NO_VERDICT

    code, report = check(raw, source)
    for line in report:
        print(line)
    print()
    print({OK: "PASS: the shipped citation is the page's own contiguous lead-in.",
           ARTIFACT_WRONG: "FAIL: the shipped citation does not match the page.",
           NO_VERDICT: "NO VERDICT: the page could not be read as expected.",
           INSTRUMENT_BROKEN: "INSTRUMENT BROKEN: this script's controls failed."}[code])
    return code


if __name__ == "__main__":
    sys.exit(main())
