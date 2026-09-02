"""The CI gate registry and ci.yml must agree.

A gate whose id does not match a real job silently never gates anything; a job
whose `if:` names a flag no gate produces evaluates to the empty string, is never
equal to 'true', and silently never runs. Both failures are invisible in a green
run — the job just reports "skipped" forever — so they are checked here rather
than left to review.

See code/specs/ci-gate-registry.md.
"""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
REGISTRY = REPO_ROOT / "code" / "specs" / "data" / "ci-gates.json"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"

# Job keys sit at exactly two spaces of indentation under `jobs:`.
JOB_KEY = re.compile(r"^  ([a-z0-9][a-z0-9-]*):$", re.MULTILINE)
# Every flag reference. Jobs downstream of detect read `needs.detect.outputs`;
# steps inside the detect job itself read `steps.detect.outputs`.
FLAG_REFERENCE = re.compile(r"(?:needs|steps)\.detect\.outputs\.(run_[a-z0-9_]+)")
# Every flag the detect job declares as an output.
FLAG_DECLARATION = re.compile(r"^      (run_[a-z0-9_]+):", re.MULTILINE)


def output_name(gate_id: str) -> str:
    """Mirror of cigates.OutputName in the Go build tool."""
    return "run_" + gate_id.replace("-", "_")


class CIGateRegistryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
        cls.gates = cls.registry["gates"]
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")
        # Scan for job keys only after `jobs:`; the `on:` block above it also
        # has two-space keys ("push:", "pull_request:") that are not jobs.
        cls.jobs_section = cls.workflow[cls.workflow.index("\njobs:\n") :]
        cls.job_keys = set(JOB_KEY.findall(cls.jobs_section))

    def test_derived_sets_are_not_empty(self) -> None:
        # Every assertion below is driven by a regex over ci.yml. If one of those
        # regexes stops matching — an indentation change, a formatting change —
        # the checks would pass by finding nothing. Fail loudly instead.
        self.assertGreater(len(self.job_keys), 5, "job-key scan found almost nothing")
        self.assertTrue(FLAG_DECLARATION.findall(self.workflow), "no run_* outputs found")
        self.assertTrue(FLAG_REFERENCE.findall(self.workflow), "no run_* references found")
        self.assertNotIn("push", self.job_keys, "job-key scan leaked into the `on:` block")

    # -- registry shape ----------------------------------------------------

    def test_schema_version_is_current(self) -> None:
        self.assertEqual(self.registry["schema_version"], 1)

    def test_every_gate_declares_a_description(self) -> None:
        for gate_id, gate in self.gates.items():
            with self.subTest(gate=gate_id):
                self.assertTrue(gate.get("description", "").strip())

    def test_every_gate_can_fire(self) -> None:
        # A gate with neither clause is dead code that reports "skipped" forever.
        for gate_id, gate in self.gates.items():
            with self.subTest(gate=gate_id):
                self.assertTrue(
                    gate.get("packages") or gate.get("paths"),
                    f"gate {gate_id} declares neither packages nor paths",
                )

    def test_every_gate_id_is_a_valid_actions_output_name(self) -> None:
        for gate_id in self.gates:
            with self.subTest(gate=gate_id):
                self.assertRegex(gate_id, r"^[a-z0-9][a-z0-9_-]*$")

    def test_gate_scopes_are_known(self) -> None:
        for gate_id, gate in self.gates.items():
            with self.subTest(gate=gate_id):
                self.assertIn(gate.get("scope", "job"), {"job", "step"})

    def test_declared_packages_exist_on_disk(self) -> None:
        # A typo in a package name makes that clause permanently dead.
        for gate_id, gate in self.gates.items():
            for name in gate.get("packages", []):
                with self.subTest(gate=gate_id, package=name):
                    if "/programs/" in name:
                        language, _, directory = name.partition("/programs/")
                        path = REPO_ROOT / "code" / "programs" / language / directory
                    else:
                        language, _, directory = name.partition("/")
                        path = REPO_ROOT / "code" / "packages" / language / directory
                    self.assertTrue(path.is_dir(), f"{name} does not resolve to {path}")

    def test_declared_paths_have_a_matching_prefix_on_disk(self) -> None:
        # Globs cannot be checked exactly, but the fixed prefix before the first
        # wildcard must exist, which catches renames and typos.
        for gate_id, gate in self.gates.items():
            for pattern in gate.get("paths", []):
                with self.subTest(gate=gate_id, path=pattern):
                    fixed = pattern.split("*")[0].rstrip("/")
                    if not fixed:
                        continue
                    candidate = REPO_ROOT / fixed
                    if candidate.exists():
                        continue
                    # A pattern like ".../test_build_tool_conformance_*.py"
                    # leaves a partial filename; check its directory instead.
                    self.assertTrue(
                        candidate.parent.is_dir(),
                        f"neither {candidate} nor its parent exists",
                    )

    # -- registry <-> workflow agreement -----------------------------------

    def test_every_job_scoped_gate_names_a_real_job(self) -> None:
        for gate_id, gate in self.gates.items():
            if gate.get("scope", "job") != "job":
                continue
            with self.subTest(gate=gate_id):
                self.assertIn(
                    gate_id,
                    self.job_keys,
                    f"gate {gate_id} does not match any job key in ci.yml",
                )

    def test_every_job_scoped_gate_is_wired_into_its_job(self) -> None:
        for gate_id, gate in self.gates.items():
            if gate.get("scope", "job") != "job":
                continue
            expected = f"if: needs.detect.outputs.{output_name(gate_id)} == 'true'"
            with self.subTest(gate=gate_id):
                self.assertIn(
                    expected,
                    self.workflow,
                    f"job {gate_id} does not gate itself on {output_name(gate_id)}",
                )

    def test_every_gate_is_declared_as_a_detect_output(self) -> None:
        declared = set(FLAG_DECLARATION.findall(self.workflow))
        for gate_id in self.gates:
            with self.subTest(gate=gate_id):
                self.assertIn(
                    output_name(gate_id),
                    declared,
                    f"detect does not expose {output_name(gate_id)} as an output",
                )

    def test_every_referenced_flag_comes_from_a_gate(self) -> None:
        # The reverse direction: an `if:` naming a flag no gate produces
        # evaluates to "" and the job never runs.
        known = {output_name(gate_id) for gate_id in self.gates}
        for flag in set(FLAG_REFERENCE.findall(self.workflow)):
            with self.subTest(flag=flag):
                self.assertIn(
                    flag,
                    known,
                    f"ci.yml references {flag}, which no registry gate produces",
                )

    def test_every_step_scoped_gate_is_used_somewhere(self) -> None:
        referenced = set(FLAG_REFERENCE.findall(self.workflow))
        for gate_id, gate in self.gates.items():
            if gate.get("scope", "job") != "step":
                continue
            with self.subTest(gate=gate_id):
                self.assertIn(
                    output_name(gate_id),
                    referenced,
                    f"step gate {gate_id} is declared but never used",
                )

    def test_gated_jobs_depend_on_detect(self) -> None:
        # `if: needs.detect.outputs.…` without `needs: detect` is a workflow
        # error, but an easy one to introduce when adding a job by copy-paste.
        for gate_id, gate in self.gates.items():
            if gate.get("scope", "job") != "job":
                continue
            body = self._job_body(gate_id)
            with self.subTest(gate=gate_id):
                self.assertIn("needs: detect", body, f"job {gate_id} lacks `needs: detect`")

    def test_ci_gate_requires_every_gated_job(self) -> None:
        # A gated job that is not in ci-gate's needs list can fail without
        # failing the one required status check.
        body = self._job_body("ci-gate")
        for gate_id, gate in self.gates.items():
            if gate.get("scope", "job") != "job":
                continue
            with self.subTest(gate=gate_id):
                self.assertIn(gate_id, body, f"ci-gate does not depend on {gate_id}")

    def _job_body(self, job_id: str) -> str:
        """Return the ci.yml text of one job, from its key to the next job key."""
        start = self.jobs_section.index(f"\n  {job_id}:\n")
        remainder = self.jobs_section[start + 1 :]
        following = JOB_KEY.search(remainder, pos=1)
        return remainder if following is None else remainder[: following.start()]


if __name__ == "__main__":
    unittest.main()
