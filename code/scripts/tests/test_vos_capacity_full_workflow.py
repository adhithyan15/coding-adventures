"""Structural contract of `.github/workflows/vos-capacity-full.yml` (VOS-028c3c-3a, 3b-2).

Why this test exists
====================

The workflow runs only on dispatch and on one weekly schedule: no pull request
ever runs it, so CI never parses it, and a mistake in it shows up only at the
first real run -- which costs about 220 runner-hours.  What can be checked without
running it is its SHAPE, and the shape is where the security and integrity
rules of VOS30 section 9.12.2 live:

    triggers      exactly dispatch (modes record and verify) and ONE weekly cron
                  (`41 1 * * 0`); never a PR or workflow_run trigger
    permissions   top-level `contents: read`; every job its own; `issues: write`
                  only in `report`, and nothing else there; no `actions: read`
    pins          every action on a four-name allowlist, by 40-hex commit SHA
    injection     NO `${{ }}` expression inside any `run:` block: everything
                  reaches a script through the step's `env:`
    integrity     one binary shared by digest; the join is called with all five
                  `--expect-*` flags (VOS-D136) after an exact-set artifact check
    budget        a timeout on every job and every step; 330 / 320 minutes for
                  a shard; `max-parallel: 8`, `fail-fast: false`
    verify        verify-shard runs `check --full --at-checkout` and may only
                  fail on exit 2 or exit 1 with no file; the retry is a separate
                  job; the triage validates every exit code; the fan-in joins
                  with five --expect-* flags and runs `decision-diff` against the
                  file in ITS OWN checkout; `verify-verdict` is the verdict

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
# The only job-level keys the workflow may use.  Anything else (`uses` and
# `secrets` for a reusable workflow, `container`, `services`, `environment`,
# `env`, `defaults`, `continue-on-error`, ...) widens what a job can reach.
ALLOWED_JOB_KEYS = {
    "name", "needs", "if", "runs-on", "timeout-minutes", "permissions",
    "outputs", "strategy", "steps",
}
# `needs:` and `if:` of every job, exactly (None = the key must be absent).
JOB_CONDITIONS = {
    "gate": (None, None),
    "build": ("gate", "needs.gate.outputs.run == 'true'"),
    "record-shard": (
        "[gate, build]",
        "needs.gate.outputs.mode == 'record' && github.repository == "
        "'adhithyan15/coding-adventures' && startsWith(github.ref, 'refs/heads/')",
    ),
    "record-join": ("[gate, build, record-shard]", "needs.gate.outputs.mode == 'record'"),
    "verify-shard": (
        "[gate, build]",
        "needs.gate.outputs.mode == 'verify' && github.repository == "
        "'adhithyan15/coding-adventures' && startsWith(github.ref, 'refs/heads/')",
    ),
    "verify-triage": (
        "[gate, verify-shard]",
        "${{ !cancelled() && needs.gate.outputs.mode == 'verify' }}",
    ),
    "verify-retry": (
        "[gate, build, verify-triage]",
        "needs.gate.outputs.mode == 'verify' && needs.verify-triage.outputs.retry != '[]' && "
        "github.repository == 'adhithyan15/coding-adventures' && startsWith(github.ref, 'refs/heads/')",
    ),
    "verify-fan-in": (
        "[gate, build, verify-shard, verify-triage, verify-retry]",
        "${{ !cancelled() && needs.gate.outputs.mode == 'verify' && needs.build.result == 'success' && "
        "needs.verify-shard.result == 'success' && needs.verify-triage.result == 'success' && (needs.verify-retry.result == 'success' || "
        "needs.verify-retry.result == 'skipped') }}",
    ),
    "verify-verdict": (
        "[gate, verify-fan-in]",
        "always() && needs.gate.outputs.mode == 'verify'",
    ),
    "report": (
        "[gate, build, record-shard, record-join, verify-shard, verify-triage, verify-retry, "
        "verify-fan-in, verify-verdict]",
        "always() && (contains(needs.*.result, 'failure') || contains(needs.*.result, 'cancelled') "
        "|| needs.verify-fan-in.outputs.flakes != '')",
    ),
}
# Exactly these two triggers (VOS30 9.12.2 item 4); one weekly cron in a free slot
# (ci.yml uses 00:00 daily and 06, 12, 18:00), owner decision "Weekly only".
ALLOWED_TRIGGERS = {"workflow_dispatch", "schedule"}
CRON = "41 1 * * 0"
REPOSITORY = "adhithyan15/coding-adventures"
JOBS = [
    "gate", "build", "record-shard", "record-join",
    "verify-shard", "verify-triage", "verify-retry", "verify-fan-in", "verify-verdict",
    "report",
]
RESULTS_FILE = "code/specs/data/vos30-capacity-results.json"
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
    def __init__(self, text: str, keep_comments: bool = False) -> None:
        self.text = text
        # keep_comments=True is the RAW view: GitHub expands `${{ }}` even inside
        # a `#` line of a `run: |` block, so that rule must see those lines.
        self.code = text if keep_comments else strip_comments(text)
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
    if got != ALLOWED_TRIGGERS:
        problems.append(f"triggers must be exactly {sorted(ALLOWED_TRIGGERS)}, got {sorted(got)}")
    crons = re.findall(r"^\s*-?\s*cron:\s*(.*?)\s*$", wf.code, re.M)
    if [c.strip("\"'") for c in crons] != [CRON]:
        problems.append(f"there must be exactly one cron, {CRON!r}, got {crons}")
    if not re.search(r"^ {6}mode:\s*$", wf.code, re.M) or not re.search(
        r"^ {8}options:\s*\[record, verify\]\s*$", wf.code, re.M
    ):
        problems.append("inputs.mode must be a choice whose only options are [record, verify]")
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
    # No write permission anywhere but that one, and no `actions:` permission at all
    # (the dropped gate was the only thing that needed `actions: read`).
    if len(re.findall(r"^\s+[\w-]+:\s*write\s*$", wf.code, re.M)) != 1:
        problems.append("the only write permission is report's `issues: write`")
    if re.search(r"^\s+actions:\s*\S+", wf.code, re.M):
        problems.append("no `actions:` permission is allowed (there is no Actions-API lookup)")
    if re.search(r"write-all|read-all", wf.code):
        problems.append("`write-all` and `read-all` are forbidden")
    report = wf.jobs.get("report")
    if report and any(s.uses and s.uses.startswith("actions/checkout") for s in report.steps):
        problems.append("report must not check out")
    return problems


def rule_jobs_present(wf: Workflow) -> list[str]:
    if list(wf.jobs) != JOBS:
        return [f"jobs must be {JOBS}, got {list(wf.jobs)}"]
    return []


def rule_timeouts(wf: Workflow) -> list[str]:
    problems = []
    for name, job in wf.jobs.items():
        if not re.fullmatch(r"\d+", job.key("timeout-minutes") or ""):
            problems.append(f"job {name} has no timeout-minutes")
        for step in job.steps:
            if not re.fullmatch(r"\d+", step.key("timeout-minutes") or ""):
                problems.append(f"job {name}: step {step.lines[0].strip()} has no timeout-minutes")
    for name, marker in (
        ("record-shard", "--tier full"),
        ("verify-shard", "check --full"),
        ("verify-retry", "check --full"),
    ):
        shard = wf.jobs.get(name)
        if shard:
            if shard.key("timeout-minutes") != "330":
                problems.append(f"{name} timeout-minutes must be 330")
            probe = [s for s in shard.steps if marker in (s.run_block() or "")]
            if len(probe) != 1 or probe[0].key("timeout-minutes") != "320":
                problems.append(f"the probe step of {name} must be exactly one step with timeout-minutes 320")
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


def rule_job_keys_and_conditions(wf: Workflow) -> list[str]:
    problems = []
    for name, job in wf.jobs.items():
        keys = {m.group(1) for l in job.lines if (m := re.match(r"^ {4}([\w-]+):", l))}
        extra = keys - ALLOWED_JOB_KEYS
        if extra:
            problems.append(f"job {name} has job-level keys off the allowlist: {sorted(extra)}")
        if name in JOB_CONDITIONS:
            needs, cond = JOB_CONDITIONS[name]
            if job.key("needs") != needs:
                problems.append(f"job {name} `needs:` must be {needs!r}, got {job.key('needs')!r}")
            if job.key("if") != cond:
                problems.append(f"job {name} `if:` must be {cond!r}, got {job.key('if')!r}")
    return problems


def rule_gate(wf: Workflow) -> list[str]:
    gate = wf.jobs.get("gate")
    if gate is None:
        return ["gate is missing"]
    body = "\n".join(s.run_block() or "" for s in gate.steps)
    problems = []
    for needle in (
        f'"$GITHUB_REPOSITORY" != "{REPOSITORY}"',
        'case "$GITHUB_REF" in',
        "refs/heads/*) ;;",
    ):
        if needle not in body:
            problems.append(f"the gate script lacks {needle!r} (a tag or fork dispatch must fail loudly)")
    if body.count("exit 1") < 4:
        problems.append("the gate must fail (exit 1) on a wrong repository, a non-branch ref, a wrong mode and a wrong event")
    # Record mode must never come from `schedule`: the schedule arm sets verify and
    # nothing else, and a dispatch can choose only record or verify.
    if not re.search(r"schedule\)\s*\n\s*mode=verify\s*\n\s*;;", body):
        problems.append("the gate's schedule arm must set mode=verify and nothing else")
    if "record | verify) mode=\"$INPUT_MODE\" ;;" not in body:
        problems.append("a dispatch may choose only record or verify")
    if 'case "$GITHUB_EVENT_NAME" in' not in body:
        problems.append("the gate must take the mode from the event name")
    outputs = "\n".join(gate.block("outputs") or [])
    if "steps.mode.outputs.mode" not in outputs:
        problems.append("the gate must output mode")
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
    # RAW text: comment lines inside a `run: |` block are still expanded by GitHub.
    raw = Workflow(wf.text, keep_comments=True)
    for job, step in raw.all_steps():
        block = step.run_block()
        if block is not None and "${{" in block:
            problems.append(f"job {job.name}: a `${{{{ }}}}` expression is inside a run block")
    for job, step in wf.all_steps():
        block = step.run_block()
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
    summaries = [b for b in scripts if "GITHUB_STEP_SUMMARY" in b and "sha256sum" in b]
    if not summaries:
        problems.append("record-join must write the joined file's SHA-256 to the step summary")
    elif not all(v in summaries[0] for v in ("$GITHUB_SHA", "$GITHUB_REF", "$GITHUB_RUN_ID", "$GITHUB_RUN_ATTEMPT")):
        problems.append("the summary step must also record the commit, ref, run id and run attempt (provenance)")
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
    if (report.key("if") or "") != JOB_CONDITIONS["report"][1]:
        problems.append("report `if:` must be the failure, cancelled or flakes expression")
    env = {}
    for step in report.steps:
        env.update(step.env_keys())
    if env.get("GH_TOKEN") != "${{ github.token }}":
        problems.append("report needs GH_TOKEN: ${{ github.token }} in step env")
    for need in ("RUN_ID", "RUN_URL", "MODE", "RETRIED", "FLAKES", "RESULT_VERDICT", "RESULT_FAN_IN"):
        if need not in env:
            problems.append(f"report must pass {need} through env")
    body = "\n".join(s.run_block() or "" for s in report.steps)
    # Shard index lists crossed job outputs: they are used only after a pattern check,
    # and nothing else a verify produces (row values) reaches the report.
    if body.count("grep -Eq") < 2:
        problems.append("report must pattern-check the shard index lists before using them")
    for needle in ("$retried", "$flakes"):
        if needle not in body:
            problems.append(f"report must print the validated index list {needle}")
    for forbidden in ("needs.verify-shard.outputs", "rerun.json", "download-artifact"):
        if forbidden in report.text:
            problems.append(f"report must not read verify data ({forbidden})")
    if "github-actions[bot]" not in body or "env.TITLE" not in body:
        problems.append("report must match the open issue by exact title and github-actions[bot]")
    if "gh issue comment" not in body or "gh issue create" not in body:
        problems.append("report must comment when found and create otherwise")
    return problems


def _check_step(job: Job, marker: str = "check --full") -> list[Step]:
    return [st for st in job.steps if marker in (st.run_block() or "")]


def _verify_leg(wf: Workflow, name: str, tail: str) -> list[str]:
    """What verify-shard and verify-retry share: the same leg, once, at the checkout."""
    job = wf.jobs.get(name)
    if job is None:
        return [f"{name} is missing"]
    problems = []
    strategy = "\n".join(job.block("strategy") or [])
    for needle in ("fail-fast: false", "max-parallel: 8"):
        if needle not in strategy:
            problems.append(f"{name} strategy lacks {needle}")
    body = "\n".join(st.run_block() or "" for st in job.steps)
    steps = _check_step(job)
    if len(steps) != 1:
        problems.append(f"{name} must run `check --full` in exactly one step (no retry loop inside a shard)")
        return problems
    run = steps[0].run_block() or ""
    if run.count("check --full") != 1 or re.search(r"\b(for|while|until)\b", run):
        problems.append(f"{name}: the check runs once, with no loop around it")
    for needle in (
        '"$exe" check --full --at-checkout',
        '--shard "$SHARD_K/64"',
        '--rerun-out "$out/rerun.json"',
        '--run-id "$RUN_ID"',
        '--run-attempt "$RUN_ATTEMPT"',
        '--exe "$exe"',
        "--jobs 2",
        f'"$GITHUB_WORKSPACE/{RESULTS_FILE}"',
    ):
        if needle not in run:
            problems.append(f"{name}: the check command lacks {needle}")
    if "--tier full" in body:
        problems.append(f"{name} must not run `run --tier full`")
    # exit 0 or 1 is a success of the job, anything else (or no file) a failure.
    if "0 | 1) ;;" not in run or "exit 1" not in run or '[ ! -s "$out/rerun.json" ]' not in run:
        problems.append(f"{name}: exit 2, or exit 1 with no re-run file, must fail the job")
    if 'printf \'%s\\n\' "$rc" > "$out/exit-code"' not in run:
        problems.append(f"{name}: the exit code must be written to an exit-code file")
    if '|| rc=$?' not in run:
        problems.append(f"{name}: the check's exit status must be captured, not allowed to end the step")
    for needle in ("test -x /usr/bin/time", '"$GITHUB_WORKSPACE" = "$BUILD_WORKSPACE"', "chmod +x", '"$BUILD_SHA256"'):
        if needle not in body:
            problems.append(f"{name} must keep record-shard's workspace and digest steps ({needle})")
    if not any(st.uses and st.uses.startswith("dtolnay/rust-toolchain") for st in job.steps):
        problems.append(f"{name} must install the toolchain")
    checkouts = [st for st in job.steps if st.uses and st.uses.startswith("actions/checkout")]
    if len(checkouts) != 1 or checkouts[0].with_keys().get("fetch-depth") != "0":
        problems.append(f"{name} must check out once with fetch-depth: 0")
    uploads = [st for st in job.steps if st.uses and st.uses.startswith("actions/upload-artifact")]
    want = "verify-shard-${{ matrix.k }}-of-64-${{ github.run_id }}-${{ github.run_attempt }}" + tail
    if len(uploads) != 1 or uploads[0].with_keys().get("name") != want:
        problems.append(f"{name}'s artifact must be named {want}")
    elif "exit-code" not in uploads[0].text or "rerun.json" not in uploads[0].text:
        problems.append(f"{name} must upload the re-run file and the exit-code file")
    return problems


def rule_verify_shard(wf: Workflow) -> list[str]:
    job = wf.jobs.get("verify-shard")
    problems = _verify_leg(wf, "verify-shard", "")
    if job is None:
        return problems
    text = "\n".join(job.block("strategy") or [])
    m = re.search(r"^ {8}k:\s*\[([^\]]*)\]\s*$", text, re.M)
    if not m or [int(x) for x in m.group(1).split(",")] != list(range(64)):
        problems.append("the verify-shard matrix must be exactly k: [0..63]")
    if "--at-checkout" not in "\n".join(st.run_block() or "" for st in job.steps):
        problems.append("verify-shard must use --at-checkout")
    return problems


def rule_verify_triage(wf: Workflow) -> list[str]:
    job = wf.jobs.get("verify-triage")
    if job is None:
        return ["verify-triage is missing"]
    problems = []
    downloads = [st for st in job.steps if st.uses and st.uses.startswith("actions/download-artifact")]
    if len(downloads) != 1 or set(downloads[0].with_keys()) != {"path"}:
        problems.append("verify-triage must download all of this run's artifacts with only `path`")
    body = "\n".join(st.run_block() or "" for st in job.steps)
    # Exit codes come from artifacts: each must be a lone 0 or 1, else the job fails.
    for needle in ('case "$v" in', "0) ;;", "1) retry=", "*) echo", "exit 1", "wc -c", "tr -d"):
        if needle not in body:
            problems.append(f"verify-triage must validate every exit code strictly (lacks {needle!r})")
    if 'if ! diff "$RUNNER_TEMP/expected.txt" "$RUNNER_TEMP/found.txt"; then' not in body:
        problems.append("verify-triage must require exactly the 64 shard artifacts and the binary")
    if 'echo "retry=[$retry]" >> "$GITHUB_OUTPUT"' not in body:
        problems.append("verify-triage must output the JSON list of shards to retry")
    # A value read from an artifact must never be evaluated as shell.
    if re.search(r"\b(eval|source)\b|\$\(\s*cat\b|bash -c|sh -c", body):
        problems.append("verify-triage must not evaluate artifact content")
    return problems


def rule_verify_retry(wf: Workflow) -> list[str]:
    job = wf.jobs.get("verify-retry")
    problems = _verify_leg(wf, "verify-retry", "-retry")
    if job is None:
        return problems
    strategy = "\n".join(job.block("strategy") or [])
    if "k: ${{ fromJSON(needs.verify-triage.outputs.retry) }}" not in strategy:
        problems.append("the verify-retry matrix must be fromJSON of the triage's list")
    # The retry is a separate job: verify-shard never retries inside itself.
    shard = wf.jobs.get("verify-shard")
    if shard and len(_check_step(shard)) != 1:
        problems.append("verify-shard must not contain the retry")
    return problems


def rule_verify_fan_in(wf: Workflow) -> list[str]:
    job = wf.jobs.get("verify-fan-in")
    if job is None:
        return ["verify-fan-in is missing"]
    problems = []
    if "verify-shard" not in (job.key("needs") or "") or "needs.verify-shard.result == 'success'" not in (job.key("if") or ""):
        problems.append("verify-fan-in must need verify-shard and require its success")
    checkouts = [st for st in job.steps if st.uses and st.uses.startswith("actions/checkout")]
    if len(checkouts) != 1:
        problems.append("verify-fan-in must have its own checkout (the committed file is read from it)")
    downloads = [st for st in job.steps if st.uses and st.uses.startswith("actions/download-artifact")]
    if len(downloads) != 1 or set(downloads[0].with_keys()) != {"path"}:
        problems.append("verify-fan-in must download all of this run's artifacts with only `path`")
    scripts = [st.run_block() or "" for st in job.steps]
    body = "\n".join(scripts)
    # Exact set: the binary, the 64 shards and exactly the listed retries.
    sets = [b for b in scripts if 'if ! diff "$RUNNER_TEMP/expected.txt" "$RUNNER_TEMP/found.txt"; then' in b]
    if len(sets) != 1 or "exit 1" not in sets[0] or "RETRY_LIST" not in sets[0] or (
        'echo "verify-shard-${k}-of-64-${RUN_ID}-${RUN_ATTEMPT}-retry"' not in sets[0]
    ):
        problems.append("verify-fan-in must require exactly the binary, 64 shards and the listed retries")
    elif "jq -e" not in sets[0]:
        problems.append("verify-fan-in must re-validate the retry list before using it")
    # A retried shard's file replaces its first file.
    if "-retry/rerun.json" not in body or 'grep -qx "$k"' not in body:
        problems.append("verify-fan-in must let a shard's retry file replace its first file")
    joins = [b for b in scripts if " join " in b]
    if len(joins) != 1:
        problems.append("verify-fan-in must have exactly one join command")
    else:
        for flag in (
            '--expect-binary-sha256 "$BUILD_SHA256"',
            '--expect-run-id "$RUN_ID"',
            '--expect-run-attempt "$RUN_ATTEMPT"',
            "--expect-shards 64",
            "--expect-jobs 2",
            "--out",
        ):
            if flag not in joins[0]:
                problems.append(f"the verify join command lacks {flag}")
    diffs = [(i, st) for i, st in enumerate(job.steps) if " decision-diff " in (st.run_block() or "")]
    if len(diffs) != 1:
        problems.append("verify-fan-in must have exactly one decision-diff command")
    else:
        idx, step = diffs[0]
        run = step.run_block() or ""
        if "--expect-shards 64" not in run:
            problems.append("decision-diff must carry --expect-shards 64")
        if "--rerun " not in run:
            problems.append("decision-diff must name the joined file with --rerun")
        m = re.search(r'--recorded\s+"([^"]+)"', run)
        if not m:
            problems.append("decision-diff needs --recorded PATH")
        else:
            rec = m.group(1)
            # The committed file of THIS job's checkout, never anything downloaded.
            if rec != f"$GITHUB_WORKSPACE/{RESULTS_FILE}" or "RUNNER_TEMP" in rec or "artifacts" in rec:
                problems.append(f"decision-diff --recorded must be the checkout's {RESULTS_FILE}, got {rec}")
        ups = [i for i, st in enumerate(job.steps) if st.uses and st.uses.startswith("actions/upload-artifact")]
        if len(ups) != 1 or job.steps[ups[0]].with_keys().get("retention-days") != "90":
            problems.append("the joined verify file must be uploaded once with retention-days: 90")
        elif ups[0] > idx:
            problems.append("the joined verify file must be uploaded before decision-diff (a red verify keeps its file)")
    summaries = [b for b in scripts if "GITHUB_STEP_SUMMARY" in b and "sha256sum" in b]
    if not summaries:
        problems.append("verify-fan-in must write the joined verify file's SHA-256 to the step summary")
    elif not all(v in summaries[0] for v in ("$GITHUB_SHA", "$GITHUB_REF", "$GITHUB_RUN_ID", "$sum")):
        problems.append("the verify summary must also record the commit, ref and run id")
    if "$(git rev-parse HEAD)" not in body or '"$GITHUB_SHA"' not in body:
        problems.append("verify-fan-in must assert its checkout is the run's commit")
    if "flakes=" not in body or "flakes:" not in "\n".join(job.block("outputs") or []):
        problems.append("verify-fan-in must output the flake indices")
    if re.search(r"git (push|commit)|gh pr|--force", body):
        problems.append("verify-fan-in must not push or commit")
    return problems


def rule_checkout_is_the_run_commit(wf: Workflow) -> list[str]:
    """Every job that checks out must assert HEAD is the run's commit."""
    problems = []
    for name, job in wf.jobs.items():
        if not any(st.uses and st.uses.startswith("actions/checkout") for st in job.steps):
            continue
        if not any('test "$(git rev-parse HEAD)" = "$GITHUB_SHA"' in (st.run_block() or "") for st in job.steps):
            problems.append(f"job {name} checks out but never asserts HEAD equals GITHUB_SHA")
    return problems


def rule_verify_verdict(wf: Workflow) -> list[str]:
    job = wf.jobs.get("verify-verdict")
    if job is None:
        return ["verify-verdict is missing"]
    problems = []
    body = "\n".join(st.run_block() or "" for st in job.steps)
    env = {}
    for st in job.steps:
        env.update(st.env_keys())
    if env.get("FAN_IN_RESULT") != "${{ needs.verify-fan-in.result }}":
        problems.append("verify-verdict must read the fan-in's result through env")
    if '[ "$FAN_IN_RESULT" != "success" ]' not in body or "exit 1" not in body:
        problems.append("verify-verdict must fail unless the fan-in succeeded")
    return problems


RULES = [
    rule_jobs_present,
    rule_triggers,
    rule_permissions,
    rule_job_keys_and_conditions,
    rule_gate,
    rule_timeouts,
    rule_no_unsafe_keys,
    rule_uses_pinned,
    rule_no_expressions_in_run,
    rule_runner_and_concurrency,
    rule_record_shard,
    rule_build,
    rule_artifacts,
    rule_join,
    rule_verify_shard,
    rule_verify_triage,
    rule_verify_retry,
    rule_verify_fan_in,
    rule_verify_verdict,
    rule_checkout_is_the_run_commit,
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
        self.assertEqual(list(wf.jobs), JOBS)
        self.assertEqual(wf.triggers(), {"workflow_dispatch", "schedule"})
        self.assertGreaterEqual(sum(len(j.steps) for j in wf.jobs.values()), 45)
        self.assertGreaterEqual(
            sum(1 for _, s in wf.all_steps() if s.run_block() is not None), 25
        )
        self.assertEqual(len(wf.jobs["verify-fan-in"].steps), 10)

    def test_every_uses_is_on_the_allowlist_by_sha(self) -> None:
        # Independent of the parser: a plain regex over the raw text.
        for m in re.finditer(r"^\s*(?:-\s+)?uses:\s*(\S+)", self.text, re.M):
            name, _, ref = m.group(1).partition("@")
            self.assertIn(name, ALLOWED_ACTIONS, m.group(0))
            self.assertEqual(ref, ALLOWED_ACTIONS[name], m.group(0))

    def test_the_pins_are_the_ones_ci_uses(self) -> None:
        # Parse `uses: <action>@<sha>` pairs; every use of an action in ci.yml must
        # carry the same SHA as here (a bare substring would accept a stray mention).
        ci = CI.read_text(encoding="utf-8")
        pairs: dict[str, set[str]] = {}
        for m in re.finditer(r"^\s*(?:-\s+)?uses:\s*([\w./-]+)@([0-9a-f]{40})\b", ci, re.M):
            pairs.setdefault(m.group(1), set()).add(m.group(2))
        for name, sha in ALLOWED_ACTIONS.items():
            self.assertEqual(pairs.get(name), {sha}, f"{name} is not pinned only at {sha} in ci.yml")

    def test_toolchain_is_the_exact_pin(self) -> None:
        # toolchain_pins.py compares this value with mise.toml; here it only has to
        # be present, exact, and installed wherever the probe is built or run.
        for job in ("build", "record-shard", "verify-shard", "verify-retry"):
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

    def test_a_second_cron_or_a_different_cron(self) -> None:
        self.assertFires(
            "rule_triggers",
            self.mutated('    - cron: "41 1 * * 0"\n', '    - cron: "41 1 * * 0"\n    - cron: "41 1 * * 1-6"\n'),
        )
        self.assertFires("rule_triggers", self.mutated('"41 1 * * 0"', '"0 0 * * *"'))

    def test_a_missing_schedule(self) -> None:
        text = self.mutated('  schedule:\n    - cron: "41 1 * * 0"\n', "")
        self.assertFires("rule_triggers", text)

    def test_a_mode_choice_that_is_not_record_and_verify(self) -> None:
        self.assertFires("rule_triggers", self.mutated("options: [record, verify]", "options: [record]"))
        self.assertFires("rule_triggers", self.mutated("options: [record, verify]", "options: [record, verify, nightly]"))

    def test_other_triggers_are_forbidden(self) -> None:
        for trig in ("pull_request_target:", "workflow_run:\n    workflows: [x]"):
            text = self.mutated("on:\n  workflow_dispatch:", f"on:\n  {trig}\n  workflow_dispatch:")
            self.assertFires("rule_triggers", text)
            self.assertFires("rule_no_unsafe_keys", text)

    def test_actions_read_anywhere(self) -> None:
        text = self.mutated("    permissions:\n      contents: read\n    outputs:\n      run:", "    permissions:\n      contents: read\n      actions: read\n    outputs:\n      run:")
        self.assertFires("rule_permissions", text)

    def test_a_write_permission_outside_report(self) -> None:
        text = self.mutated("    timeout-minutes: 20\n    permissions:\n      contents: read\n", "    timeout-minutes: 20\n    permissions:\n      contents: write\n")
        self.assertFires("rule_permissions", text)

    def test_a_gate_that_lets_the_schedule_record(self) -> None:
        self.assertFires("rule_gate", self.mutated("              mode=verify\n", "              mode=record\n"))
        self.assertFires("rule_gate", self.mutated('record | verify) mode="$INPUT_MODE" ;;', 'record | verify | other) mode="$INPUT_MODE" ;;'))

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
        end = self.text.index("\n  # ── verify-shard")
        text = self.text[:up] + self.text[val_comment:end] + "\n" + self.text[up:val_comment] + self.text[end:]
        self.assertNotEqual(text, self.text)
        self.assertFires("rule_join", text)

    def test_a_missing_concurrency_group(self) -> None:
        self.assertFires("rule_runner_and_concurrency", self.mutated("cancel-in-progress: false", "cancel-in-progress: true"))

    def test_a_loosened_record_if(self) -> None:
        text = self.mutated(" && startsWith(github.ref, 'refs/heads/')", "")
        self.assertFires("rule_record_shard", text)
        self.assertFires("rule_job_keys_and_conditions", text)

    def test_an_expression_in_a_comment_inside_run(self) -> None:
        # GitHub expands `${{ }}` in a `#` line of a run block too.
        text = self.mutated(
            "          set -euo pipefail\n          test -x /usr/bin/time\n",
            "          set -euo pipefail\n          # ${{ github.head_ref }}\n          test -x /usr/bin/time\n",
        )
        self.assertFires("rule_no_expressions_in_run", text)

    def test_a_reusable_workflow_job_with_inherited_secrets(self) -> None:
        text = self.mutated(
            "    timeout-minutes: 45\n", "    timeout-minutes: 45\n    uses: ./x.yml\n    secrets: inherit\n"
        )
        self.assertFires("rule_job_keys_and_conditions", text)

    def test_a_container_job(self) -> None:
        text = self.mutated("    timeout-minutes: 45\n", "    timeout-minutes: 45\n    container: ubuntu\n")
        self.assertFires("rule_job_keys_and_conditions", text)

    def test_a_job_level_environment_or_services(self) -> None:
        for extra in ("    environment: prod\n", "    services:\n      x:\n        image: y\n"):
            text = self.mutated("    timeout-minutes: 45\n", "    timeout-minutes: 45\n" + extra)
            self.assertFires("rule_job_keys_and_conditions", text)

    def test_record_join_that_runs_always(self) -> None:
        text = self.mutated(
            "    if: needs.gate.outputs.mode == 'record'\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30",
            "    if: always() && needs.gate.outputs.mode == 'record'\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30",
        )
        self.assertFires("rule_job_keys_and_conditions", text)

    def test_a_changed_needs(self) -> None:
        text = self.mutated("    needs: [gate, build, record-shard]\n", "    needs: [gate, build]\n")
        self.assertFires("rule_job_keys_and_conditions", text)

    def test_a_gate_that_no_longer_refuses_a_tag(self) -> None:
        text = self.mutated('case "$GITHUB_REF" in', 'case "$GITHUB_REF_NAME" in')
        self.assertFires("rule_gate", text)

    def test_a_gate_that_no_longer_refuses_another_repository(self) -> None:
        text = self.mutated('"$GITHUB_REPOSITORY" != "adhithyan15/coding-adventures"', '"$GITHUB_REPOSITORY" != "x/y"')
        self.assertFires("rule_gate", text)

    def test_a_summary_without_the_commit_or_ref(self) -> None:
        self.assertFires("rule_join", self.mutated('            echo "commit: $GITHUB_SHA"\n', ""))
        self.assertFires("rule_join", self.mutated('            echo "ref: $GITHUB_REF"\n', ""))

    # ── the verify path ─────────────────────────────────────────────────────

    def in_job(self, job: str, old: str, new: str, count: int = 1) -> str:
        """Mutate `old` only inside one job (several jobs share the same lines)."""
        head, sep, body = self.text.partition("\nname: vos-capacity-full\n")
        self.assertTrue(sep)
        start = body.index(f"\n  {job}:\n")
        nxt = body.find("\n  # ── ", start + 1)
        end = len(body) if nxt < 0 else nxt
        section = body[start:end]
        self.assertGreaterEqual(section.count(old), 1, f"{job}: mutation anchor missing: {old!r}")
        return head + sep + body[:start] + section.replace(old, new, count) + body[end:]

    def test_a_verify_shard_without_at_checkout(self) -> None:
        text = self.in_job("verify-shard", "check --full --at-checkout", "check --full")
        self.assertFires("rule_verify_shard", text)

    def test_a_verify_shard_that_survives_exit_two_or_no_file(self) -> None:
        self.assertFires("rule_verify_shard", self.in_job("verify-shard", "            0 | 1) ;;\n", "            0 | 1 | 2) ;;\n"))
        self.assertFires(
            "rule_verify_shard",
            self.in_job("verify-shard", '          if [ ! -s "$out/rerun.json" ]; then\n', '          if false; then\n'),
        )

    def test_a_verify_shard_that_ends_on_exit_one(self) -> None:
        # `|| rc=$?` is what lets exit 1 with a file succeed.
        self.assertFires("rule_verify_shard", self.in_job("verify-shard", " || rc=$?\n", "\n"))

    def test_a_verify_shard_missing_a_flag(self) -> None:
        for flag in ("--rerun-out", "--run-id", "--run-attempt", "--exe", "--jobs 2"):
            self.assertFires("rule_verify_shard", self.in_job("verify-shard", flag, "--other", 99))

    def test_a_verify_shard_against_another_file(self) -> None:
        text = self.in_job("verify-shard", '"$GITHUB_WORKSPACE/code/specs/data/vos30-capacity-results.json"', '"$RUNNER_TEMP/results.json"')
        self.assertFires("rule_verify_shard", text)

    def test_a_retry_inside_the_shard(self) -> None:
        text = self.in_job("verify-shard", "          printf '%s\\n' \"$rc\" > \"$out/exit-code\"\n",
                           "          for again in 1 2; do \"$exe\" check --full \"$out\"; done\n")
        self.assertFires("rule_verify_shard", text)

    def test_a_retry_that_is_not_a_matrix_over_the_triage_list(self) -> None:
        text = self.in_job("verify-retry", "k: ${{ fromJSON(needs.verify-triage.outputs.retry) }}", "k: [0, 1]")
        self.assertFires("rule_verify_retry", text)

    def test_a_retry_artifact_without_the_suffix(self) -> None:
        text = self.in_job("verify-retry", "${{ github.run_attempt }}-retry", "${{ github.run_attempt }}")
        self.assertFires("rule_verify_retry", text)

    def test_a_triage_that_accepts_any_exit_code(self) -> None:
        self.assertFires("rule_verify_triage", self.in_job("verify-triage", "              *) echo", "              2) echo"))
        self.assertFires("rule_verify_triage", self.in_job("verify-triage", 'size="$(wc -c < "$shard/exit-code")"', 'size=1'))

    def test_a_triage_that_evaluates_artifact_content(self) -> None:
        text = self.in_job("verify-triage", 'v="$(tr -d \'\\n\' < "$shard/exit-code")"', 'v="$(tr -d \'\\n\' < "$shard/exit-code")"\n            eval "$v"')
        self.assertFires("rule_verify_triage", text)

    def test_a_triage_that_does_not_need_all_64(self) -> None:
        text = self.in_job("verify-triage", "          if ! diff ", "          if false && ! diff ")
        self.assertFires("rule_verify_triage", text)

    def test_a_triage_that_skips_when_a_shard_failed(self) -> None:
        text = self.in_job("verify-triage", "if: ${{ !cancelled() && ", "if: ${{ ")
        self.assertFires("rule_job_keys_and_conditions", text)

    def test_a_fan_in_missing_an_expect_flag(self) -> None:
        for flag in ("--expect-binary-sha256", "--expect-run-id", "--expect-run-attempt", "--expect-shards 64", "--expect-jobs 2"):
            self.assertFires("rule_verify_fan_in", self.in_job("verify-fan-in", flag, "--other"))

    def test_a_decision_diff_without_expect_shards(self) -> None:
        text = self.in_job("verify-fan-in", "            --rerun \"$RUNNER_TEMP/verify-joined.json\" \\\n            --expect-shards 64\n",
                           "            --rerun \"$RUNNER_TEMP/verify-joined.json\"\n")
        self.assertFires("rule_verify_fan_in", text)

    def test_a_decision_diff_recorded_under_the_download_directory(self) -> None:
        text = self.in_job("verify-fan-in", '--recorded "$GITHUB_WORKSPACE/code/specs/data/vos30-capacity-results.json"',
                           '--recorded "$RUNNER_TEMP/artifacts/results.json"')
        self.assertFires("rule_verify_fan_in", text)

    def test_a_fan_in_without_its_own_checkout(self) -> None:
        head, sep, body = self.text.partition("\nname: vos-capacity-full\n")
        start = body.index("\n  verify-fan-in:\n")
        a = body.index("      - name: Check out the run's commit\n", start)
        b = body.index("      # The committed file is the recording", a)
        self.assertFires("rule_verify_fan_in", head + sep + body[:a] + body[b:])

    def test_a_fan_in_that_accepts_a_stray_artifact(self) -> None:
        text = self.in_job("verify-fan-in", "          if ! diff ", "          if false && ! diff ")
        self.assertFires("rule_verify_fan_in", text)
        self.assertFires("rule_verify_fan_in", self.in_job("verify-fan-in", "-retry\"\n            done < ", "\"\n            done < "))

    def test_a_fan_in_where_the_retry_does_not_replace_the_shard(self) -> None:
        text = self.in_job("verify-fan-in", 'if grep -qx "$k" "$RUNNER_TEMP/retry.txt"; then', "if false; then")
        self.assertFires("rule_verify_fan_in", text)

    def test_the_joined_verify_file_must_be_kept_90_days_before_decision_diff(self) -> None:
        self.assertFires("rule_verify_fan_in", self.in_job("verify-fan-in", "retention-days: 90", "retention-days: 5"))
        head, sep, body = self.text.partition("\nname: vos-capacity-full\n")
        start = body.index("\n  verify-fan-in:\n")
        up = body.index("      # Before decision-diff on purpose", start)
        cmp_ = body.index("      # The verdict comes from DATA", up)
        end = body.index("\n  # ── verify-verdict", cmp_)
        swapped = body[:up] + body[cmp_:end] + "\n" + body[up:cmp_] + body[end:]
        self.assertNotEqual(swapped, body)
        self.assertFires("rule_verify_fan_in", head + sep + swapped)

    def test_a_fan_in_that_does_not_require_the_shards(self) -> None:
        text = self.in_job("verify-fan-in", " && needs.verify-shard.result == 'success'", "")
        self.assertFires("rule_verify_fan_in", text)
        self.assertFires("rule_job_keys_and_conditions", text)

    def test_a_checkout_without_the_head_assertion(self) -> None:
        for job in ("build", "record-shard", "verify-shard", "verify-retry", "verify-fan-in"):
            text = self.in_job(job, 'test "$(git rev-parse HEAD)" = "$GITHUB_SHA"', "true")
            self.assertFires("rule_checkout_is_the_run_commit", text)

    def test_a_verdict_that_does_not_fail(self) -> None:
        self.assertFires("rule_verify_verdict", self.mutated('"$FAN_IN_RESULT" != "success"', '"$FAN_IN_RESULT" = "nothing"'))
        self.assertFires("rule_job_keys_and_conditions", self.mutated("    if: always() && needs.gate.outputs.mode == 'verify'\n", "    if: needs.gate.outputs.mode == 'verify'\n"))

    def test_a_report_that_forwards_unchecked_indices(self) -> None:
        self.assertFires("rule_report", self.mutated("grep -Eq", "grep -Fq", 99))

    def test_a_verify_leg_without_a_timeout_or_the_right_one(self) -> None:
        self.assertFires("rule_timeouts", self.in_job("verify-shard", "    timeout-minutes: 330\n", "    timeout-minutes: 360\n"))
        self.assertFires("rule_timeouts", self.in_job("verify-retry", "        timeout-minutes: 320\n", ""))


if __name__ == "__main__":
    unittest.main()
