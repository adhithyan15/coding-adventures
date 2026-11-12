"""Tests for `python_pins.py`, the generator and checker for Python pins.

Two things must hold for the pins to mean anything, and both are pinned here
with a fake uv so no network or uv binary is involved:

  * requirement EXTRACTION must exclude first-party names (otherwise PyPI would
    be consulted for packages that only exist in this repo) and must keep
    extras, markers and build-system.requires in the right bucket;

  * the CHECK must treat a name missing from the constraints file as a failure.
    uv lets an unlisted name float without complaint, so completeness is the
    one property uv itself will never enforce.
"""

from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import python_pins


def manifest(
    name,
    dependencies=(),
    optional=None,
    groups=None,
    build=(),
    sources=None,
):
    """Build the parsed form of a pyproject.toml."""

    document = {
        "project": {
            "name": name,
            "dependencies": list(dependencies),
            "optional-dependencies": optional or {},
        },
        "dependency-groups": groups or {},
    }
    if build:
        document["build-system"] = {"requires": list(build)}
    if sources:
        document["tool"] = {"uv": {"sources": sources}}
    return document


class Repo:
    """A throwaway directory holding fake tracked pyproject.toml files."""

    def __init__(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()

    def add(self, relative):
        path = self.root / relative / "pyproject.toml"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("", encoding="utf-8")
        return path

    def close(self):
        self._tmp.cleanup()


class DeriveRequirementsTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repo()
        self.addCleanup(self.repo.close)

    def derive(self, documents, tools=None):
        resolved = {self.repo.add(rel): doc for rel, doc in documents.items()}
        return python_pins.derive_requirements(self.repo.root, resolved, tools or {})

    def test_in_repo_projects_are_excluded(self):
        runtime, _ = self.derive(
            {
                "a": manifest("alpha", ["beta>=1", "requests>=2"]),
                "b": manifest("beta"),
            }
        )
        self.assertEqual(runtime.names(), {"requests"})

    def test_names_are_normalised_per_pep_503(self):
        runtime, _ = self.derive(
            {
                "a": manifest("alpha", ["Typing_Extensions>=4"]),
                "b": manifest("b", ["typing.extensions"]),
            }
        )
        self.assertEqual(runtime.names(), {"typing-extensions"})

    def test_in_repo_name_matching_is_normalised_too(self):
        runtime, _ = self.derive(
            {"a": manifest("Alpha_Pkg"), "b": manifest("b", ["alpha-pkg"])}
        )
        self.assertEqual(runtime.names(), set())

    def test_coding_adventures_namespace_is_always_first_party(self):
        # coding-adventures-pipeline has no project anywhere; it must not be
        # sent to PyPI.
        runtime, _ = self.derive(
            {"a": manifest("a", ["coding-adventures-pipeline", "pytest"])}
        )
        self.assertEqual(runtime.names(), {"pytest"})

    def test_uv_sources_path_redirects_make_a_name_first_party(self):
        # `matrix` is not a project name (the project is coding-adventures-
        # matrix) but [tool.uv.sources] points it at a tracked directory.
        self.repo.add("lib")
        runtime, _ = self.derive(
            {
                "lib": manifest("coding-adventures-matrix"),
                "app": manifest(
                    "app",
                    ["matrix", "numpy"],
                    sources={"matrix": {"path": "../lib", "editable": True}},
                ),
            }
        )
        self.assertEqual(runtime.names(), {"numpy"})

    def test_extras_and_markers_are_kept_verbatim(self):
        runtime, _ = self.derive(
            {
                "a": manifest(
                    "a",
                    ["requests[socks]>=2.0 ; python_version >= '3.9'"],
                )
            }
        )
        self.assertEqual(runtime.names(), {"requests"})
        self.assertEqual(
            runtime.lines(), ["requests[socks]>=2.0 ; python_version >= '3.9'"]
        )

    def test_optional_dependencies_and_groups_are_runtime(self):
        runtime, _ = self.derive(
            {
                "a": manifest(
                    "a",
                    optional={"dev": ["pytest>=8"]},
                    groups={"lint": ["ruff", {"include-group": "dev"}]},
                )
            }
        )
        self.assertEqual(runtime.names(), {"pytest", "ruff"})

    def test_build_system_requires_goes_to_the_build_set(self):
        runtime, build = self.derive(
            {"a": manifest("a", ["pytest"], build=["hatchling>=1.0"])}
        )
        self.assertEqual(runtime.names(), {"pytest"})
        self.assertEqual(build.names(), {"hatchling"})

    def test_tool_installs_join_the_runtime_set(self):
        runtime, _ = self.derive(
            {"a": manifest("a")},
            tools={".github/workflows/x.yml": {"Jsonschema", "coding-adventures-x"}},
        )
        self.assertEqual(runtime.names(), {"jsonschema"})
        self.assertEqual(
            runtime.sources["jsonschema"], {".github/workflows/x.yml"}
        )

    def test_sources_record_every_manifest(self):
        runtime, _ = self.derive(
            {"a": manifest("a", ["pytest"]), "b": manifest("b", ["pytest"])}
        )
        self.assertEqual(
            runtime.sources["pytest"], {"a/pyproject.toml", "b/pyproject.toml"}
        )


class ParsePinsTests(unittest.TestCase):
    def test_reads_names_versions_markers_and_ignores_hashes(self):
        text = (
            "# comment\n"
            "Foo_Bar==1.0 \\\n"
            "    --hash=sha256:aa \\\n"
            "    --hash=sha256:bb\n"
            "colorama==0.4.6 ; sys_platform == 'win32'\n"
        )
        self.assertEqual(
            python_pins.parse_pins(text),
            {("foo-bar", "1.0"), ("colorama", "0.4.6")},
        )

    def test_same_name_at_two_versions_is_kept(self):
        text = "x==1 ; python_version < '3.10'\nx==2 ; python_version >= '3.10'\n"
        self.assertEqual(python_pins.parse_pins(text), {("x", "1"), ("x", "2")})


class FakeUv:
    """Stands in for `uv pip compile`: returns a canned resolution."""

    def __init__(self, output):
        self.output = output
        self.calls = []

    def __call__(self, arguments):
        self.calls.append(arguments)
        return self.output


def requirements_of(*names):
    requirements = python_pins.Requirements()
    for name in names:
        requirements.add(name, name, f"{name}/pyproject.toml")
    return requirements


class CheckOneTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.path = Path(self._tmp.name) / "constraints.txt"

    def check(self, file_text, names, resolution):
        self.path.write_text(file_text, encoding="utf-8")
        return python_pins.check_one(
            requirements_of(*names),
            self.path,
            Path("constraints.txt"),
            FakeUv(resolution),
        )

    def test_up_to_date_file_passes_and_hash_lists_are_ignored(self):
        problems = self.check(
            "pytest==9.0 \\\n    --hash=sha256:aa\n",
            ["pytest"],
            "pytest==9.0\n",
        )
        self.assertEqual(problems, [])

    def test_missing_name_is_reported_with_its_source(self):
        # The dangerous case: uv itself would not complain.
        problems = self.check(
            "pytest==9.0\n", ["pytest", "requests"], "pytest==9.0\nrequests==2.0\n"
        )
        self.assertTrue(
            any("no pin for 'requests'" in p and "requests/pyproject.toml" in p
                for p in problems),
            problems,
        )

    def test_missing_name_is_reported_even_if_resolution_matches(self):
        # Even when the fake resolution equals the file, a required name the
        # file lacks must fail on its own.
        problems = self.check("pytest==9.0\n", ["pytest", "ruff"], "pytest==9.0\n")
        self.assertTrue(any("no pin for 'ruff'" in p for p in problems), problems)

    def test_newly_resolved_version_is_reported(self):
        problems = self.check("pytest==9.0\n", ["pytest"], "pytest==9.1\n")
        self.assertTrue(any("would add pytest==9.1" in p for p in problems))
        self.assertTrue(any("pytest==9.0 is no longer needed" in p for p in problems))

    def test_extra_pin_no_longer_required_is_reported(self):
        problems = self.check(
            "pytest==9.0\nold==1\n", ["pytest"], "pytest==9.0\n"
        )
        self.assertEqual(problems, ["constraints.txt: old==1 is no longer needed."])

    def test_missing_file_is_reported(self):
        problems = python_pins.check_one(
            requirements_of("pytest"),
            self.path.parent / "absent.txt",
            Path("absent.txt"),
            FakeUv(""),
        )
        self.assertEqual(problems, ["absent.txt does not exist."])

    def test_recompile_uses_the_file_as_constraint_without_hashes(self):
        self.path.write_text("pytest==9.0\n", encoding="utf-8")
        uv = FakeUv("pytest==9.0\n")
        python_pins.check_one(
            requirements_of("pytest"), self.path, Path("c.txt"), uv
        )
        (arguments,) = uv.calls
        self.assertIn("--constraint", arguments)
        self.assertEqual(
            arguments[arguments.index("--constraint") + 1], str(self.path)
        )
        self.assertNotIn("--generate-hashes", arguments)
        self.assertIn("--universal", arguments)


class CompileTests(unittest.TestCase):
    def test_generate_flag_requests_hashes(self):
        uv = FakeUv("")
        python_pins.compile_requirements(requirements_of("x"), hashes=True, runner=uv)
        self.assertIn("--generate-hashes", uv.calls[0])
        self.assertIn(python_pins.PYTHON_VERSION, uv.calls[0])

    def test_render_file_has_header_with_regenerate_command(self):
        text = python_pins.render_file("x==1", "What.", "UV_CONSTRAINT")
        self.assertTrue(text.startswith("# GENERATED FILE"))
        self.assertIn(python_pins.REGENERATE_COMMAND, text)
        self.assertTrue(text.rstrip().endswith("x==1"))


class MainTests(unittest.TestCase):
    def test_missing_uv_is_a_clear_error(self):
        original = python_pins.shutil.which
        python_pins.shutil.which = lambda _name: None
        self.addCleanup(setattr, python_pins.shutil, "which", original)
        with self.assertRaises(python_pins.PinsError) as caught:
            python_pins.find_uv()
        self.assertIn("uv was not found on PATH", str(caught.exception))

    def test_check_failure_prints_the_fix_command_and_exits_1(self):
        original = python_pins.check
        python_pins.check = lambda _root: ["python-constraints.txt: no pin for 'x'."]
        self.addCleanup(setattr, python_pins, "check", original)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = python_pins.main(["check"])
        self.assertEqual(code, 1)
        self.assertIn(python_pins.REGENERATE_COMMAND, out.getvalue())


if __name__ == "__main__":
    unittest.main()
