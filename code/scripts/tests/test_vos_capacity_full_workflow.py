"""Structural contract of `.github/workflows/vos-capacity-full.yml` (VOS-028c3c-3a).

Why this test exists
====================

The workflow is a *dispatch-only* recording: no pull request ever runs it, so
CI never parses it, and a mistake in it shows up only at the first real
dispatch -- which costs about 220 runner-hours.  What can be checked without
running it is its SHAPE, and the shape is where the security and integrity
rules of VOS30 section 9.12.2 live:

    triggers      dispatch only (part b adds `schedule`); never a PR trigger
    permissions   top-level `contents: read`; every job its own; `issues: write`
                  only in `report`, and nothing else there
    pins          every action on a four-name allowlist, by 40-hex commit SHA
    injection     NO `${{ }}` expression inside any `run:` block: everything
                  reaches a script through the step's `env:`
    integrity     one binary shared by digest; the join is called with all five
                  `--expect-*` flags (VOS-D136) after an exact-set artifact check
    budget        a timeout on every job and every step; 330 / 320 minutes for
                  the shard; `max-parallel: 8`, `fail-fast: false`

Like the other workflow tests in this directory it reads the file as text (the
contracts job is standard-library only).  It understands just enough
indentation to split the file into jobs, steps and `run:` blocks:

    jobs:                       indent 0
      build:                    indent 2   a job
        timeout-minutes: 45     indent 4   a job key
        steps:
          - name: ...           indent 6   a step starts here
            timeout-minutes: 5  indent 8   a step key
            run: |              indent 8
              set -euo ...      indent 10  the script

The negative tests feed a mutated copy of the text to the same checker and
require that the matching rule fires, so the checker is known to be able to
fail.
"""

from __future__ import annotations

import re
import unittest
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
WORKFLOW = REPO / ".github" / "workflows" / "vos-capacity-full.yml"
CI = REPO / ".github" / "workflows" / "ci.yml"

# The four actions the workflow may use, each pinned by the SHA ci.yml uses.
ALLOWED_ACTIONS = {
    "actions/checkout": "3d3c42e5aac5ba805825da76410c181273ba90b1",
    "actions/upload-artifact": "cf430e030ddbb5b0abf93d22962f4752f3646cd9",
    "actions/download-artifact": "9000827ccba6bdab643e8b6fd33ac0654aef8333",
    "dtolnay/rust-toolchain": "4360b52568e2003a75bf9bc1d59f33a8e3fc893c",
}
# Triggers the final workflow may have.  Part a (this one) has no schedule yet;
# VOS-028c3c-3b flips PART_A to False and asserts exactly both.
ALLOWED_TRIGGERS = {"workflow_dispatch", "schedule"}
PART_A = True
REPOSITORY = "adhithyan15/coding-adventures"
JOBS_PART_A = ["gate", "build", "record-shard", "record-join", "report"]
BINARY_ARTIFACT = "vos-capacity-probe-binary"


# ── a tiny text model of the workflow ───────────────────────────────────────


def indent_of(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def strip_comments(text: str) -> str:
    """Drop whole-line comments (the header prose names forbidden keys on purpose)."""
    return "\n".join(
        line for line in text.splitlines() if not line.lstrip().startswith("#")
    )


@dataclass
class Step:
    lines: list[str]

    @property
    def text(self) -> str:
        return "\n".join(self.lines)

    def key(self, name: str) -> str | None:
        """Value of a step-level key (indent 8, or the `- ` line at indent 6)."""
        pat = re.compile(rf"^(?: {{6}}- | {{8}}){re.escape(name)}:\s*(.*?)\s*$")
        for line in self.lines:
            m = pat.match(line)
            if m:
                return m.group(1)
        return None

    @property
    def uses(self) -> str | None:
        v = self.key("uses")
        return v.split(" #")[0].strip().strip("\"'") if v else None

    def with_keys(self) -> dict[str, str]:
        out: dict[str, str] = {}
        inside = False
        for line in self.lines:
            if re.match(r"^ {8}with:\s*$", line):
                inside = True
                continue
            if inside:
                if line.strip() and indent_of(line) <= 8:
                    inside = False
                    continue
                m = re.match(r"^ {10}([\w-]+):\s*(.*?)\s*$", line)
                if m:
                    out[m.group(1)] = m.group(2)
        return out

    def env_keys(self) -> dict[str, str]:
        out: dict[str, str] = {}
        inside = False
        for line in self.lines:
            if re.match(r"^ {8}env:\s*$", line):
                inside = True
                continue
            if inside:
                if line.strip() and indent_of(line) <= 8:
                    inside = False
                    continue
                m = re.match(r"^ {10}([\w-]+):\s*(.*?)\s*$", line)
                if m:
                    out[m.group(1)] = m.group(2)
        return out

    def run_block(self) -> str | None:
        for i, line in enumerate(self.lines):
            m = re.match(r"^(?: {6}- | {8})run:\s*(.*)$", line)
            if not m:
                continue
            first = m.group(1).strip()
            if first and not re.fullmatch(r"[|>][+-]?", first):
                return first
            body = []
            for nxt in self.lines[i + 1 :]:
                if nxt.strip() and indent_of(nxt) <= 8:
                    break
                body.append(nxt)
            return "\n".join(body)
        return None


@dataclass
class Job:
    name: str
    lines: list[str]
    steps: list[Step] = field(default_factory=list)

    @property
    def text(self) -> str:
        return "\n".join(self.lines)

    def key(self, name: str) -> str | None:
        pat = re.compile(rf"^ {{4}}{re.escape(name)}:\s*(.*?)\s*$")
        for line in self.lines:
            m = pat.match(line)
            if m:
                return m.group(1)
        return None

    def block(self, name: str) -> list[str] | None:
        """The indent-6 lines under a job-level `name:` key, or None if absent."""
        for i, line in enumerate(self.lines):
            if re.match(rf"^ {{4}}{re.escape(name)}:\s*$", line):
                body = []
                for nxt in self.lines[i + 1 :]:
                    if nxt.strip() and indent_of(nxt) <= 4:
                        break
                    body.append(nxt)
                return body
        return None

    def permissions(self) -> dict[str, str] | None:
        body = self.block("permissions")
        if body is None:
            return None
        out = {}
        for line in body:
            m = re.match(r"^ {6}([\w-]+):\s*(\S+)\s*$", line)
            if m:
                out[m.group(1)] = m.group(2)
        return out


class Workflow:
    def __init__(self, text: str) -> None:
        self.text = text
        self.code = strip_comments(text)
        self.lines = self.code.splitlines()
        self.jobs = self._jobs()

    def _top_block(self, key: str) -> list[str] | None:
        for i, line in enumerate(self.lines):
            if re.match(rf"^{re.escape(key)}:\s*(.*)$", line):
                body = []
                for nxt in self.lines[i + 1 :]:
                    if nxt.strip() and indent_of(nxt) == 0:
                        break
                    body.append(nxt)
                return body
        return None

    def triggers(self) -> set[str]:
        body = self._top_block("on")
        if body is None:
            return set()
        return {m.group(1) for line in body if (m := re.match(r"^ {2}([\w-]+):", line))}

    def top_permissions(self) -> dict[str, str] | None:
        body = self._top_block("permissions")
        if body is None:
            return None
        out = {}
        for line in body:
            m = re.match(r"^ {2}([\w-]+):\s*(\S+)\s*$", line)
            if m:
                out[m.group(1)] = m.group(2)
        return out

    def _jobs(self) -> dict[str, Job]:
        body = self._top_block("jobs") or []
        jobs: dict[str, Job] = {}
        current: Job | None = None
        for line in body:
            m = re.match(r"^ {2}([\w-]+):\s*$", line)
            if m:
                current = Job(m.group(1), [])
                jobs[current.name] = current
                continue
            if current is not None:
                current.lines.append(line)
        for job in jobs.values():
            in_steps = False
            step: Step | None = None
            for line in job.lines:
                if re.match(r"^ {4}steps:\s*$", line):
                    in_steps = True
                    continue
                if not in_steps:
                    continue
                if line.strip() and indent_of(line) <= 4:
                    in_steps = False
                    continue
                if re.match(r"^ {6}- ", line):
                    step = Step([line])
                    job.steps.append(step)
                elif step is not None:
                    step.lines.append(line)
        return jobs

    def all_steps(self):
        for job in self.jobs.values():
            for step in job.steps:
                yield job, step


# ── the rules: each returns a list of problems (empty = holds) ──────────────


def rule_triggers(wf: Workflow) -> list[str]:
    got = wf.triggers()
    problems = []
    if not got <= ALLOWED_TRIGGERS:
        problems.append(f"triggers {sorted(got - ALLOWED_TRIGGERS)} are not allowed")
    if "workflow_dispatch" not in got:
        problems.append("workflow_dispatch is missing")
    if PART_A and got != {"workflow_dispatch"}:
        problems.append(f"part a is dispatch only, got {sorted(got)}")
    if not PART_A and got != ALLOWED_TRIGGERS:
        problems.append(f"part b has exactly dispatch and schedule, got {sorted(got)}")
    if not re.search(r"^ {6}mode:\s*$", wf.code, re.M) or not re.search(
        r"^ {8}options:\s*\[record\]\s*$", wf.code, re.M
    ):
        problems.append("inputs.mode must be a choice whose only option is [record]")
    if not re.search(r"^ {8}type:\s*choice\s*$", wf.code, re.M):
        problems.append("inputs.mode must be `type: choice`")
    return problems


def rule_permissions(wf: Workflow) -> list[str]:
    problems = []
    if wf.top_permissions() != {"contents": "read"}:
        problems.append(f"top-level permissions must be contents: read, got {wf.top_permissions()}")
    for name, job in wf.jobs.items():
        perms = job.permissions()
        if perms is None:
            problems.append(f"job {name} declares no permissions")
        elif name == "report":
            if perms != {"issues": "write"}:
                problems.append(f"report must have issues: write and nothing else, got {perms}")
        elif perms != {"contents": "read"}:
            problems.append(f"job {name} must have contents: read only, got {perms}")
    if len(re.findall(r"issues:\s*write", wf.code)) != 1:
        problems.append("`issues: write` must appear exactly once (in report)")
    report = wf.jobs.get("report")
    if report and any(s.uses and s.uses.startswith("actions/checkout") for s in report.steps):
        problems.append("report must not check out")
    return problems


def rule_jobs_present(wf: Workflow) -> list[str]:
    if list(wf.jobs) != JOBS_PART_A:
        return [f"jobs must be {JOBS_PART_A}, got {list(wf.jobs)}"]
    return []


def rule_timeouts(wf: Workflow) -> list[str]:
    problems = []
    for name, job in wf.jobs.items():
        if not re.fullmatch(r"\d+", job.key("timeout-minutes") or ""):
            problems.append(f"job {name} has no timeout-minutes")
        for step in job.steps:
            if not re.fullmatch(r"\d+", step.key("timeout-minutes") or ""):
                problems.append(f"job {name}: step {step.lines[0].strip()} has no timeout-minutes")
    shard = wf.jobs.get("record-shard")
    if shard:
        if shard.key("timeout-minutes") != "330":
            problems.append("record-shard timeout-minutes must be 330")
        probe = [s for s in shard.steps if "--tier full" in (s.run_block() or "")]
        if len(probe) != 1 or probe[0].key("timeout-minutes") != "320":
            problems.append("the probe step of record-shard must be exactly one step with timeout-minutes 320")
    return problems


def rule_no_unsafe_keys(wf: Workflow) -> list[str]:
    problems = []
    if re.search(r"continue-on-error", wf.code):
        problems.append("continue-on-error is forbidden")
    if re.search(r"actions/cache", wf.code):
        problems.append("actions/cache is forbidden")
    for key in ("pattern", "run-id", "github-token"):
        if re.search(rf"^\s*(?:-\s+)?{key}:", wf.code, re.M):
            problems.append(f"`{key}:` is forbidden (download-all sees only this run)")
    for trigger in ("pull_request", "pull_request_target", "workflow_run"):
        if re.search(rf"^\s+{trigger}:", wf.code, re.M):
            problems.append(f"{trigger} is forbidden")
    for name, job in wf.jobs.items():
        env = job.block("env")
        if env is not None:
            problems.append(f"job {name} has a job-level env (the runner context is rejected there)")
    return problems


def rule_uses_pinned(wf: Workflow) -> list[str]:
    problems = []
    for job, step in wf.all_steps():
        uses = step.uses
        if uses is None:
            continue
        m = re.fullmatch(r"([\w./-]+)@([0-9a-f]{40})", uses)
        if not m:
            problems.append(f"job {job.name}: `{uses}` is not pinned by a 40-hex SHA")
            continue
        if m.group(1) not in ALLOWED_ACTIONS:
            problems.append(f"`{m.group(1)}` is not on the allowlist")
        elif ALLOWED_ACTIONS[m.group(1)] != m.group(2):
            problems.append(f"`{m.group(1)}` is not at the SHA ci.yml uses")
        if not re.search(r"uses:\s*\S+@[0-9a-f]{40} # \S+", step.text):
            problems.append(f"`{uses}` has no version comment")
        if m.group(1) == "actions/checkout" and step.with_keys().get("persist-credentials") != "false":
            problems.append("actions/checkout needs persist-credentials: false")
    return problems


def rule_no_expressions_in_run(wf: Workflow) -> list[str]:
    problems = []
    for job, step in wf.all_steps():
        block = step.run_block()
        if block is not None and "${{" in block:
            problems.append(f"job {job.name}: a `${{{{ }}}}` expression is inside a run block")
        if block is not None:
            first = next((l.strip() for l in block.splitlines() if l.strip()), "")
            if first != "set -euo pipefail":
                problems.append(f"job {job.name}: a run block does not start with `set -euo pipefail`")
    return problems


def rule_runner_and_concurrency(wf: Workflow) -> list[str]:
    problems = []
    for name, job in wf.jobs.items():
        if job.key("runs-on") != "ubuntu-24.04":
            problems.append(f"job {name} must run on ubuntu-24.04")
    if not re.search(
        r"^concurrency:\s*\n {2}group: vos-capacity-full\s*\n {2}cancel-in-progress: false\s*$",
        wf.code,
        re.M,
    ):
        problems.append("concurrency must be group vos-capacity-full with cancel-in-progress: false")
    return problems


def rule_record_shard(wf: Workflow) -> list[str]:
    shard = wf.jobs.get("record-shard")
    if shard is None:
        return ["record-shard is missing"]
    problems = []
    cond = shard.key("if") or ""
    for needle in (
        "needs.gate.outputs.mode == 'record'",
        f"github.repository == '{REPOSITORY}'",
        "startsWith(github.ref, 'refs/heads/')",
    ):
        if needle not in cond:
            problems.append(f"record-shard `if:` lacks {needle}")
    block = shard.block("strategy") or []
    text = "\n".join(block)
    for needle in ("fail-fast: false", "max-parallel: 8"):
        if needle not in text:
            problems.append(f"record-shard strategy lacks {needle}")
    m = re.search(r"^ {8}k:\s*\[([^\]]*)\]\s*$", text, re.M)
    if not m or [int(x) for x in m.group(1).split(",")] != list(range(64)):
        problems.append("the matrix must be exactly k: [0..63]")
    body = "\n".join(s.run_block() or "" for s in shard.steps)
    for needle in ("--tier full", '--shard "$SHARD_K/64"', "--jobs 2", "--run-id", "--run-attempt", "--exe", "--out"):
        if needle not in body:
            problems.append(f"record-shard script lacks {needle}")
    if "test -x /usr/bin/time" not in body:
        problems.append("record-shard must preflight /usr/bin/time")
    if '"$GITHUB_WORKSPACE" = "$BUILD_WORKSPACE"' not in body:
        problems.append("record-shard must assert the workspace path equals the build job's")
    if 'sha256sum' not in body or '"$BUILD_SHA256"' not in body or "chmod +x" not in body:
        problems.append("record-shard must chmod +x and recheck the binary's digest")
    if " validate " not in body:
        problems.append("record-shard must validate its output")
    if not any(s.uses and s.uses.startswith("dtolnay/rust-toolchain") for s in shard.steps):
        problems.append("record-shard must install the toolchain (meta::gather runs rustc)")
    uploads = [s for s in shard.steps if s.uses and s.uses.startswith("actions/upload-artifact")]
    if len(uploads) != 1 or uploads[0].with_keys().get("name") != (
        "shard-${{ matrix.k }}-of-64-${{ github.run_id }}-${{ github.run_attempt }}"
    ):
        problems.append("the shard artifact must be named shard-<K>-of-64-<run_id>-<run_attempt>")
    return problems


def rule_build(wf: Workflow) -> list[str]:
    build = wf.jobs.get("build")
    if build is None:
        return ["build is missing"]
    problems = []
    checkouts = [s for s in build.steps if s.uses and s.uses.startswith("actions/checkout")]
    if len(checkouts) != 1 or checkouts[0].with_keys().get("fetch-depth") != "0":
        problems.append("build must check out once with fetch-depth: 0")
    cargo = [s for s in build.steps if "cargo build" in (s.run_block() or "")]
    if len(cargo) != 1 or "cargo build --locked --release" not in (cargo[0].run_block() or ""):
        problems.append("build must run `cargo build --locked --release`")
    elif cargo[0].env_keys().get("CARGO_TARGET_DIR") != "${{ runner.temp }}/target":
        problems.append("CARGO_TARGET_DIR must be step-level env `${{ runner.temp }}/target`")
    out = build.block("outputs") or []
    names = {m.group(1) for l in out if (m := re.match(r"^ {6}(\w+):", l))}
    if names != {"sha256", "workspace"}:
        problems.append(f"build outputs must be sha256 and workspace, got {sorted(names)}")
    body = "\n".join(s.run_block() or "" for s in build.steps)
    if "sha256sum" not in body or "GITHUB_WORKSPACE" not in body or "test -x /usr/bin/time" not in body:
        problems.append("build must hash the binary, record the workspace and preflight /usr/bin/time")
    up = [s for s in build.steps if s.uses and s.uses.startswith("actions/upload-artifact")]
    if len(up) != 1 or up[0].with_keys().get("name") != BINARY_ARTIFACT:
        problems.append("build must upload the binary under its exact artifact name")
    return problems


def rule_artifacts(wf: Workflow) -> list[str]:
    problems = []
    for job, step in wf.all_steps():
        if step.uses and step.uses.startswith("actions/upload-artifact"):
            w = step.with_keys()
            if w.get("if-no-files-found") != "error":
                problems.append(f"job {job.name}: an upload lacks if-no-files-found: error")
            days = w.get("retention-days", "")
            if not days.isdigit() or int(days) < 5:
                problems.append(f"job {job.name}: an upload needs retention-days >= 5")
    return problems


def rule_join(wf: Workflow) -> list[str]:
    join = wf.jobs.get("record-join")
    if join is None:
        return ["record-join is missing"]
    problems = []
    downloads = [s for s in join.steps if s.uses and s.uses.startswith("actions/download-artifact")]
    if len(downloads) != 1:
        problems.append("record-join must have exactly one download-artifact step")
    elif set(downloads[0].with_keys()) != {"path"}:
        problems.append("record-join's download must set only `path` (download all of this run)")
    scripts = [s.run_block() or "" for s in join.steps]
    joins = [b for b in scripts if " join " in b]
    if len(joins) != 1:
        problems.append("record-join must have exactly one join command")
    else:
        for flag in (
            "--expect-binary-sha256",
            "--expect-run-id",
            "--expect-run-attempt",
            "--expect-shards 64",
            "--expect-jobs 2",
        ):
            if flag not in joins[0]:
                problems.append(f"the join command lacks {flag}")
        if "--out" not in joins[0]:
            problems.append("the join command lacks --out")
    if not any("diff " in b and "expected" in b and "exit 1" in b for b in scripts):
        problems.append("record-join must fail on a missing or extra artifact (exact-set check)")
    if not any("GITHUB_STEP_SUMMARY" in b and "sha256sum" in b for b in scripts):
        problems.append("record-join must write the joined file's SHA-256 to the step summary")
    ups = [i for i, s in enumerate(join.steps) if s.uses and s.uses.startswith("actions/upload-artifact")]
    vals = [i for i, s in enumerate(join.steps) if " validate " in (s.run_block() or "")]
    if len(ups) != 1 or join.steps[ups[0]].with_keys().get("retention-days") != "90":
        problems.append("the joined file must be uploaded once with retention-days: 90")
    if len(vals) != 1 or (ups and vals and vals[0] < ups[0]):
        problems.append("the joined file must be validated AFTER its upload")
    if re.search(r"git (push|commit)|gh pr|--force", "\n".join(scripts)):
        problems.append("record-join must not push or commit")
    return problems


def rule_report(wf: Workflow) -> list[str]:
    report = wf.jobs.get("report")
    if report is None:
        return ["report is missing"]
    problems = []
    if (report.key("if") or "") != (
        "always() && (contains(needs.*.result, 'failure') || contains(needs.*.result, 'cancelled'))"
    ):
        problems.append("report `if:` must be the failure-or-cancelled expression")
    env = {}
    for step in report.steps:
        env.update(step.env_keys())
    if env.get("GH_TOKEN") != "${{ github.token }}":
        problems.append("report needs GH_TOKEN: ${{ github.token }} in step env")
    for need in ("RUN_ID", "RUN_URL", "MODE"):
        if need not in env:
            problems.append(f"report must pass {need} through env")
    body = "\n".join(s.run_block() or "" for s in report.steps)
    if "github-actions[bot]" not in body or "env.TITLE" not in body:
        problems.append("report must match the open issue by exact title and github-actions[bot]")
    if "gh issue comment" not in body or "gh issue create" not in body:
        problems.append("report must comment when found and create otherwise")
    return problems


RULES = [
    rule_jobs_present,
    rule_triggers,
    rule_permissions,
    rule_timeouts,
    rule_no_unsafe_keys,
    rule_uses_pinned,
    rule_no_expressions_in_run,
    rule_runner_and_concurrency,
    rule_record_shard,
    rule_build,
    rule_artifacts,
    rule_join,
    rule_report,
]


def violations(text: str) -> dict[str, list[str]]:
    wf = Workflow(text)
    return {r.__name__: p for r in RULES if (p := r(wf))}


# ── tests ───────────────────────────────────────────────────────────────────


class VosCapacityFullWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")

    def test_the_workflow_satisfies_every_rule(self) -> None:
        found = violations(self.text)
        self.assertEqual(found, {}, "\n".join(f"{k}: {v}" for k, v in found.items()))

    def test_parser_sees_the_expected_shape(self) -> None:
        # Guards the checker itself: an empty parse would make every rule pass.
        wf = Workflow(self.text)
        self.assertEqual(list(wf.jobs), JOBS_PART_A)
        self.assertEqual(wf.triggers(), {"workflow_dispatch"})
        self.assertGreaterEqual(sum(len(j.steps) for j in wf.jobs.values()), 23)
        self.assertGreaterEqual(
            sum(1 for _, s in wf.all_steps() if s.run_block() is not None), 12
        )

    def test_every_uses_is_on_the_allowlist_by_sha(self) -> None:
        # Independent of the parser: a plain regex over the raw text.
        for m in re.finditer(r"^\s*(?:-\s+)?uses:\s*(\S+)", self.text, re.M):
            name, _, ref = m.group(1).partition("@")
            self.assertIn(name, ALLOWED_ACTIONS, m.group(0))
            self.assertEqual(ref, ALLOWED_ACTIONS[name], m.group(0))

    def test_the_pins_are_the_ones_ci_uses(self) -> None:
        ci = CI.read_text(encoding="utf-8")
        for name, sha in ALLOWED_ACTIONS.items():
            self.assertIn(f"{name}@{sha}", ci, f"{name} is not pinned at {sha} in ci.yml")

    def test_toolchain_is_the_exact_pin(self) -> None:
        # toolchain_pins.py compares this value with mise.toml; here it only has to
        # be present, exact, and installed wherever the probe is built or run.
        for job in ("build", "record-shard"):
            steps = Workflow(self.text).jobs[job].steps
            tool = [s for s in steps if (s.uses or "").startswith("dtolnay/rust-toolchain")]
            self.assertEqual(len(tool), 1, job)
            self.assertEqual(tool[0].with_keys().get("toolchain"), '"1.100.0"')

    def test_this_test_is_run_by_ci(self) -> None:
        # A guard test that no CI job runs guards nothing (lessons.d).
        ci = CI.read_text(encoding="utf-8")
        self.assertIn(
            "python3 -m unittest discover -s code/scripts/tests "
            "-p 'test_vos_capacity_full_workflow.py'",
            ci,
        )


class CheckerCanFailTests(unittest.TestCase):
    """The same checker, fed a damaged copy, must say so."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")

    def mutated(self, old: str, new: str, count: int = 1) -> str:
        # Mutate the code, never the header prose (which quotes some of the same
        # commands), so the damaged copy really differs where the rules look.
        marker = "\nname: vos-capacity-full\n"
        head, sep, body = self.text.partition(marker)
        self.assertTrue(sep)
        self.assertGreaterEqual(body.count(old), 1, f"mutation anchor missing: {old!r}")
        return head + sep + body.replace(old, new, count)

    def assertFires(self, rule: str, text: str) -> None:
        found = violations(text)
        self.assertIn(rule, found, f"{rule} did not fire; got {found}")

    def test_an_extra_trigger(self) -> None:
        text = self.mutated("on:\n  workflow_dispatch:", "on:\n  pull_request:\n  workflow_dispatch:")
        self.assertFires("rule_triggers", text)
        self.assertFires("rule_no_unsafe_keys", text)

    def test_a_schedule_trigger_in_part_a(self) -> None:
        text = self.mutated("on:\n  workflow_dispatch:", "on:\n  schedule:\n    - cron: '41 1 * * 0'\n  workflow_dispatch:")
        self.assertFires("rule_triggers", text)

    def test_an_expression_inside_run(self) -> None:
        text = self.mutated(
            "cargo build --locked --release", "cargo build --locked --release ${{ github.ref_name }}"
        )
        self.assertFires("rule_no_expressions_in_run", text)

    def test_a_missing_expect_flag(self) -> None:
        text = self.mutated('            --expect-run-id "$RUN_ID" \\\n', "")
        self.assertFires("rule_join", text)

    def test_every_expect_flag_is_required(self) -> None:
        for flag in ("--expect-binary-sha256", "--expect-run-attempt", "--expect-shards 64", "--expect-jobs 2"):
            self.assertFires("rule_join", self.mutated(flag, "--other"))

    def test_an_unpinned_action(self) -> None:
        text = self.mutated(
            "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1", "actions/checkout@v7", 99
        )
        self.assertFires("rule_uses_pinned", text)

    def test_an_action_off_the_allowlist(self) -> None:
        text = self.mutated(
            "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9",
            "someone/else@cf430e030ddbb5b0abf93d22962f4752f3646cd9",
        )
        self.assertFires("rule_uses_pinned", text)

    def test_checkout_that_keeps_credentials(self) -> None:
        text = self.mutated("persist-credentials: false", "persist-credentials: true", 99)
        self.assertFires("rule_uses_pinned", text)

    def test_continue_on_error(self) -> None:
        text = self.mutated("    timeout-minutes: 45\n", "    timeout-minutes: 45\n    continue-on-error: true\n")
        self.assertFires("rule_no_unsafe_keys", text)

    def test_a_widened_report_permission(self) -> None:
        text = self.mutated("    permissions:\n      issues: write\n", "    permissions:\n      issues: write\n      contents: write\n")
        self.assertFires("rule_permissions", text)

    def test_issues_write_outside_report(self) -> None:
        text = self.mutated("      contents: read\n    outputs:\n      sha256", "      contents: read\n      issues: write\n    outputs:\n      sha256")
        self.assertFires("rule_permissions", text)

    def test_a_job_without_permissions(self) -> None:
        text = self.mutated("    timeout-minutes: 30\n    permissions:\n      contents: read\n", "    timeout-minutes: 30\n")
        self.assertFires("rule_permissions", text)

    def test_a_step_without_a_timeout(self) -> None:
        text = self.mutated("        timeout-minutes: 320\n", "")
        self.assertFires("rule_timeouts", text)

    def test_a_matrix_that_is_not_0_to_63(self) -> None:
        text = self.mutated(", 62, 63]", ", 62]")
        self.assertFires("rule_record_shard", text)

    def test_a_cache_or_a_wildcard_download(self) -> None:
        self.assertFires(
            "rule_no_unsafe_keys",
            self.mutated("          path: ${{ runner.temp }}/artifacts\n", "          path: ${{ runner.temp }}/artifacts\n          pattern: shard-*\n"),
        )
        self.assertFires(
            "rule_join",
            self.mutated("          path: ${{ runner.temp }}/artifacts\n", "          path: ${{ runner.temp }}/artifacts\n          run-id: 1\n"),
        )

    def test_a_job_level_runner_context(self) -> None:
        text = self.mutated("    permissions:\n      contents: read\n    outputs:\n      sha256", "    permissions:\n      contents: read\n    env:\n      X: ${{ runner.temp }}/x\n    outputs:\n      sha256")
        self.assertFires("rule_no_unsafe_keys", text)

    def test_a_cargo_build_without_locked(self) -> None:
        self.assertFires("rule_build", self.mutated("cargo build --locked --release", "cargo build --release"))

    def test_a_validate_before_the_upload(self) -> None:
        # Move the validate step (with its comment) ahead of the upload step.
        up = self.text.index("      - name: Upload the joined file")
        val_comment = self.text.index("      # After the upload on purpose")
        end = self.text.index("\n  # ── report")
        text = self.text[:up] + self.text[val_comment:end] + "\n" + self.text[up:val_comment] + self.text[end:]
        self.assertNotEqual(text, self.text)
        self.assertFires("rule_join", text)

    def test_a_missing_concurrency_group(self) -> None:
        self.assertFires("rule_runner_and_concurrency", self.mutated("cancel-in-progress: false", "cancel-in-progress: true"))

    def test_a_loosened_record_if(self) -> None:
        text = self.mutated(" && startsWith(github.ref, 'refs/heads/')", "")
        self.assertFires("rule_record_shard", text)


if __name__ == "__main__":
    unittest.main()
