#!/usr/bin/env bash
# =============================================================================
# Capture the pinned Closure oracle's refusals (CCR-081)
# =============================================================================
#
# Regenerates `code/programs/rust/closurec/tests/oracle/upstream-refusals-
# <release>.json`, the evidence artifact behind the `upstream_refuses`
# fixture-set disposition in the oracle manifest.
#
# WHY THIS SCRIPT EXISTS
#
# A fixture set's disposition is a claim about where its expected output came
# from.  `upstream_golden` says "upstream produced these bytes".  For twelve
# fixtures that claim was false in a specific and unfixable way: the pinned
# oracle does not merely disagree with the golden, it REFUSES THE INPUT —
# nonzero exit, empty stdout, a named diagnostic.  There is no upstream output
# for those goldens to have come from.
#
# The honest disposition for them is `upstream_refuses`, and the manifest
# validator demands an evidence file for it.  That file must be CAPTURED, not
# written by hand.  This is the lesson of CCR-079 (#15866): a ledger that the
# gate trusts, whose values a human typed, is not evidence — it is an assertion
# wearing evidence's clothes.  Four review rounds on the ladder gate each found
# the same defect one level up because every fix added another self-declared
# field.  So: this script runs the oracle and writes down what happened.
#
# WHAT IS CAPTURED, AND WHAT IS NOT
#
# The predicate recorded here is deliberately narrow and mechanically
# re-checkable:
#
#     the pinned oracle, run with THIS FIXTURE'S OWN recorded invocation,
#     exits nonzero and writes no stdout, with the named diagnostic
#
# It is NOT the broader claim "no invocation of upstream could ever produce
# this golden".  That broader claim is true for several of these fixtures — the
# ES-module ones in particular, whose goldens preserve `import`/`export` syntax
# that upstream rewrites under every resolver setting tried — but it quantifies
# over a search that was done by hand and cannot be re-run by a script.  An
# unfalsifiable claim does not belong in a file the test suite trusts.  It
# belongs in prose on the issue, and that is where it lives (#15868).
#
# The practical consequence: a fixture whose refusal is merely a missing flag
# (`simple-importmeta` needs `--chunk_output_type=ES_MODULES`) is captured here
# too, because it does refuse as invoked.  Completing its invocation is real
# work that changes its flags.txt and its golden; until someone does that, the
# recorded fact is the refusal.
#
# USAGE
#
#     export CLOSURE_ORACLE_JAR=/path/to/closure-compiler-v20260915.jar
#     code/scripts/capture-upstream-refusals.sh
#
# The JAR's sha256 is checked against the pin before anything runs.  The script
# fails loudly if any listed fixture has started SUCCEEDING — that is good news
# and means the fixture should leave the set, but it must be a deliberate edit
# rather than a silently shrinking evidence file.
#
# Wallclock: ~30s for twelve JVM starts.

set -euo pipefail

RELEASE="v20260915"
JAR_SHA256="9c8af06056aa06f968b5a457540a85869c7ba2861c211c56d8d4ef6c35ddf36d"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PKG_ROOT="$REPO_ROOT/code/programs/rust/closurec"
OUT="$PKG_ROOT/tests/oracle/upstream-refusals-$RELEASE.json"

if [[ -z "${CLOSURE_ORACLE_JAR:-}" ]]; then
  echo "error: set CLOSURE_ORACLE_JAR to the pinned closure-compiler-$RELEASE.jar" >&2
  exit 2
fi
if [[ ! -f "$CLOSURE_ORACLE_JAR" ]]; then
  echo "error: no such file: $CLOSURE_ORACLE_JAR" >&2
  exit 2
fi

actual_sha="$(sha256sum "$CLOSURE_ORACLE_JAR" | cut -d' ' -f1)"
if [[ "$actual_sha" != "$JAR_SHA256" ]]; then
  echo "error: oracle JAR sha256 mismatch" >&2
  echo "  expected $JAR_SHA256" >&2
  echo "  actual   $actual_sha" >&2
  exit 2
fi
echo "oracle JAR sha256 verified against the $RELEASE pin"

cd "$PKG_ROOT"
CLOSURE_ORACLE_JAR="$CLOSURE_ORACLE_JAR" RELEASE="$RELEASE" OUT="$OUT" python3 - <<'PY'
import json, os, re, subprocess, hashlib

# The fixtures this set covers.  Sorted, because the manifest validator
# requires fixture lists to be strictly sorted and this file should match.
FIXTURES = [
    "simple-export",
    "simple-import",
    "simple-importexpr",
    "simple-importmeta",
    "simple-newtarget",
    "simple-private-field",
    "simple-private-generator-method",
    "simple-private-getter",
    "simple-private-method",
    "simple-super",
    "simple-try-catch",
    "simple-with",
]

jar = os.environ["CLOSURE_ORACLE_JAR"]
release = os.environ["RELEASE"]
out_path = os.environ["OUT"]

sha = hashlib.sha256(open(jar, "rb").read()).hexdigest()
java_version = subprocess.run(["java", "-version"], capture_output=True, text=True).stderr.split('"')[1]

# The manifest's `closure-flags-file-v1` command: one argv element per nonblank,
# noncomment line of the fixture's flags.txt, run from the package root with
# these -D flags so the diagnostics are locale- and encoding-stable.
DFLAGS = [
    "-Duser.language=en",
    "-Duser.country=US",
    "-Duser.timezone=UTC",
    "-Dfile.encoding=UTF-8",
]

report = {
    "schema_version": 1,
    "description": (
        "Captured refusals of the pinned Closure oracle. Each entry is the result of running the "
        "manifest's `closure-flags-file-v1` command for that fixture - its own flags.txt, verbatim, "
        "from the package root - against the pinned JAR. This records only that upstream refuses the "
        "input AS INVOKED; it is not a claim that no invocation could produce the fixture's golden. "
        "Regenerate with code/scripts/capture-upstream-refusals.sh."
    ),
    "release": release,
    "oracle_jar_sha256": sha,
    "java_version": java_version,
    "command": "closure-flags-file-v1",
    "refusals": {},
}

unexpected = []
for fixture in FIXTURES:
    flags_path = f"tests/diff/{fixture}/flags.txt"
    flags = [
        line.strip()
        for line in open(flags_path)
        if line.strip() and not line.strip().startswith("#")
    ]
    proc = subprocess.run(
        ["java", *DFLAGS, "-jar", jar, *flags], capture_output=True, text=True
    )
    match = re.search(r"ERROR - \[(JSC_[A-Z_]+)\] (.*)", proc.stderr)

    # A fixture that now compiles, or that fails without a diagnostic, is not a
    # refusal and must not be recorded as one.  Collect every such case and
    # report them together rather than dying on the first.
    if proc.returncode == 0 or proc.stdout != "" or match is None:
        unexpected.append(
            f"  {fixture}: exit={proc.returncode} stdout_bytes={len(proc.stdout)} "
            f"diagnostic={'yes' if match else 'NONE'}"
        )
        continue

    report["refusals"][fixture] = {
        "exit_status": proc.returncode,
        "stdout_bytes": len(proc.stdout),
        "diagnostic": match.group(1),
        "message": match.group(2).strip(),
    }

if unexpected:
    raise SystemExit(
        "error: these fixtures did not refuse as expected:\n"
        + "\n".join(unexpected)
        + "\n\nIf upstream now accepts one of them that is good news, but it means the fixture\n"
          "should leave the `non-minify-upstream-refuses` set and get a real golden. Edit the\n"
          "FIXTURES list in this script and the manifest together; do not let the evidence file\n"
          "shrink silently."
    )

with open(out_path, "w") as handle:
    json.dump(report, handle, indent=2)
    handle.write("\n")

print(f"wrote {len(report['refusals'])} refusals to {out_path}")
for name, entry in report["refusals"].items():
    print(f"  {name:32s} {entry['diagnostic']}")
PY
