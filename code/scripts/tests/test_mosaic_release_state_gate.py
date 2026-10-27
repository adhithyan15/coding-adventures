"""Tests for the shared Mosaic release upgrade and corrupt-recovery gate.

Run with::

    python -m unittest discover -s code/scripts/tests -p 'test_mosaic_release_state_gate.py'

(Discovery, not a dotted module path: the top-level ``code`` directory shares
its name with a standard-library module.)

The tests build small fixtures in a temporary directory rather than reusing a
committed one, so each failure mode can be produced by changing exactly one
thing. The committed Engram fixture gets its own end-to-end class at the bottom.
"""

from __future__ import annotations

import copy
import io
import json
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from typing import Any, ClassVar
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import mosaic_release_state_gate as gate

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
ENGRAM_FIXTURE = (
    REPOSITORY_ROOT
    / "code"
    / "programs"
    / "mosaic"
    / "engram-app"
    / "fixtures"
    / "release-upgrade-v0.4.0.json"
)


def fixture_document(**overrides: Any) -> dict[str, Any]:
    """A minimal valid generic fixture. v1 keeps data at the top; v3 nests it.

    That mirrors the real Engram move from v1 to v3, so the version-scoped
    expectations here exercise the same shape change the gate exists for.
    """

    document: dict[str, Any] = {
        "fixtureSchema": gate.FIXTURE_SCHEMA,
        "fixtureVersion": 1,
        "sourceApp": "demo-app",
        "sourceVersion": "1.2.3",
        "provenance": "built by the test suite",
        "snapshotSchema": "demo-mosaic-app",
        "snapshotVersion": 1,
        "acceptedLiveVersions": [1, 3],
        "expectations": [
            {"liveVersions": [1], "pointer": "/decks/0/name", "equals": "Tamil"},
            {
                "liveVersions": [3],
                "pointer": "/session/state/decks/0/name",
                "equals": "Tamil",
            },
        ],
        "state": {"decks": [{"name": "Tamil"}]},
    }
    document.update(overrides)
    return document


def envelope(schema: str, version: int, payload: Any) -> bytes:
    """A live state file as a host would write it."""

    raw = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    return gate.encode_envelope(schema, version, raw)


def symlinks_supported(directory: Path) -> bool:
    """Windows without developer mode cannot create symlinks; skip there."""

    probe = directory / "symlink-probe"
    try:
        probe.symlink_to(directory / "absent")
    except (OSError, NotImplementedError):
        return False
    probe.unlink()
    return True


class GateTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()
        self.root = Path(self._temporary.name)
        self.state = self.root / "demo-app" / "mosaic-state.v1.json"

    def tearDown(self) -> None:
        self._temporary.cleanup()

    def write_fixture(self, document: dict[str, Any]) -> Path:
        path = self.root / "fixture.json"
        path.write_text(json.dumps(document), encoding="utf-8")
        return path

    def fixture(self, **overrides: Any) -> gate.GateFixture:
        return gate.parse_fixture(fixture_document(**overrides))


class MaterializeAndVerifyTests(GateTestCase):
    def test_materialize_then_verify_is_the_happy_path(self) -> None:
        fixture = self.fixture()
        gate.materialize_fixture(fixture, self.state)

        snapshot = json.loads(self.state.read_bytes())
        self.assertEqual(snapshot["schema"], "demo-mosaic-app")
        self.assertEqual(snapshot["version"], 1)
        self.assertEqual(json.loads(bytes(snapshot["bytes"])), fixture.state)
        payload = gate.verify_state(fixture, self.state)
        self.assertEqual(payload, fixture.state)

    def test_materialized_file_is_compact_sorted_and_ends_in_one_lf(self) -> None:
        gate.materialize_fixture(self.fixture(), self.state)
        data = self.state.read_bytes()
        self.assertTrue(data.endswith(b"}\n"))
        self.assertNotIn(b"\r", data)
        self.assertNotIn(b" ", data)
        self.assertTrue(data.startswith(b'{"bytes":['))

    def test_materialize_keeps_non_ascii_as_utf8_like_serde(self) -> None:
        fixture = self.fixture(
            state={"decks": [{"name": "Tamil"}], "word": "அம்மா"},
        )
        gate.materialize_fixture(fixture, self.state)
        payload = bytes(json.loads(self.state.read_bytes())["bytes"])
        self.assertIn("அம்மா".encode(), payload)

    def test_a_newer_accepted_live_version_passes(self) -> None:
        # The new host re-persisted: v3 bytes, the deck nested under session.
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(
            envelope(
                "demo-mosaic-app",
                3,
                {"session": {"state": {"decks": [{"name": "Tamil"}]}}},
            )
        )
        gate.verify_state(self.fixture(), self.state)

    def test_the_version_selects_which_pointers_apply(self) -> None:
        # The v3 shape labelled v1: the v1 pointer applies and finds nothing,
        # even though the v3 pointer would have matched. (The test above is
        # the converse: v3 bytes pass although "/decks/0/name" is absent.)
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(
            envelope(
                "demo-mosaic-app",
                1,
                {"session": {"state": {"decks": [{"name": "Tamil"}]}}},
            )
        )
        with self.assertRaisesRegex(
            gate.GateError, "v1 payload has nothing at '/decks/0/name'"
        ):
            gate.verify_state(self.fixture(), self.state)

    def test_an_unaccepted_version_is_rejected(self) -> None:
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(
            envelope("demo-mosaic-app", 2, {"decks": [{"name": "Tamil"}]})
        )
        with self.assertRaisesRegex(
            gate.GateError, "changed the snapshot version to 2"
        ):
            gate.verify_state(self.fixture(), self.state)

    def test_a_boolean_version_is_not_version_one(self) -> None:
        self.state.parent.mkdir(parents=True)
        self.state.write_text(
            json.dumps({"schema": "demo-mosaic-app", "version": True, "bytes": []}),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(gate.GateError, "snapshot version"):
            gate.verify_state(self.fixture(), self.state)

    def test_a_schema_change_is_rejected(self) -> None:
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(envelope("other-app", 1, {"decks": [{"name": "Tamil"}]}))
        with self.assertRaisesRegex(gate.GateError, "changed the snapshot schema"):
            gate.verify_state(self.fixture(), self.state)

    def test_a_corrupt_sibling_fails_even_when_the_state_looks_fine(self) -> None:
        fixture = self.fixture()
        gate.materialize_fixture(fixture, self.state)
        gate.corrupt_sibling(self.state).write_text("damaged", encoding="utf-8")
        with self.assertRaisesRegex(gate.GateError, "quarantined a compatible"):
            gate.verify_state(fixture, self.state)

    def test_a_corrupt_sibling_is_reported_before_a_missing_state(self) -> None:
        self.state.parent.mkdir(parents=True)
        gate.corrupt_sibling(self.state).write_text("damaged", encoding="utf-8")
        with self.assertRaisesRegex(gate.GateError, "quarantined a compatible"):
            gate.verify_state(self.fixture(), self.state)

    def test_a_missing_state_is_rejected(self) -> None:
        with self.assertRaisesRegex(gate.GateError, "does not exist"):
            gate.verify_state(self.fixture(), self.state)

    def test_a_directory_at_the_state_path_is_rejected(self) -> None:
        self.state.mkdir(parents=True)
        with self.assertRaisesRegex(gate.GateError, "not a regular file"):
            gate.verify_state(self.fixture(), self.state)

    def test_bad_bytes_are_rejected(self) -> None:
        cases = {
            "out of range": [256],
            "negative": [-1],
            "boolean": [True],
            "float": [1.0],
            "string": ["{"],
            "not a list": "eyJ9",
        }
        self.state.parent.mkdir(parents=True)
        for name, encoded in cases.items():
            with self.subTest(name):
                self.state.write_text(
                    json.dumps(
                        {"schema": "demo-mosaic-app", "version": 1, "bytes": encoded}
                    ),
                    encoding="utf-8",
                )
                with self.assertRaisesRegex(gate.GateError, "invalid snapshot bytes"):
                    gate.verify_state(self.fixture(), self.state)

    def test_bytes_that_are_not_a_json_object_are_rejected(self) -> None:
        self.state.parent.mkdir(parents=True)
        for name, payload, message in (
            ("array", b"[]", "decode to a JSON object"),
            ("not json", b"{", "not JSON"),
            ("not utf-8", b"\xff", "not UTF-8"),
        ):
            with self.subTest(name):
                self.state.write_bytes(
                    gate.encode_envelope("demo-mosaic-app", 1, payload)
                )
                with self.assertRaisesRegex(gate.GateError, message):
                    gate.verify_state(self.fixture(), self.state)

    def test_an_envelope_that_is_not_an_object_is_rejected(self) -> None:
        self.state.parent.mkdir(parents=True)
        self.state.write_text("[]\n", encoding="utf-8")
        with self.assertRaisesRegex(gate.GateError, "must be a JSON object"):
            gate.verify_state(self.fixture(), self.state)

    def test_a_pointer_mismatch_is_rejected(self) -> None:
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(
            envelope("demo-mosaic-app", 1, {"decks": [{"name": "Hindi"}]})
        )
        with self.assertRaisesRegex(gate.GateError, "is 'Hindi', expected 'Tamil'"):
            gate.verify_state(self.fixture(), self.state)

    def test_a_missing_pointer_path_is_rejected(self) -> None:
        self.state.parent.mkdir(parents=True)
        for name, payload in (
            ("empty collection", {"decks": []}),
            ("no key", {"cards": []}),
            ("scalar in the way", {"decks": "none"}),
        ):
            with self.subTest(name):
                self.state.write_bytes(envelope("demo-mosaic-app", 1, payload))
                with self.assertRaisesRegex(
                    gate.GateError, "has nothing at '/decks/0/name'"
                ):
                    gate.verify_state(self.fixture(), self.state)

    def test_an_oversized_state_is_refused_before_parsing(self) -> None:
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(b" " * (gate.MAX_STATE_BYTES + 1))
        with self.assertRaisesRegex(gate.GateError, "exceeds the"):
            gate.verify_state(self.fixture(), self.state)

    def test_a_symlinked_state_is_refused(self) -> None:
        if not symlinks_supported(self.root):
            self.skipTest("symlinks are unavailable on this platform")
        fixture = self.fixture()
        real = self.root / "elsewhere.json"
        gate.materialize_fixture(fixture, real)
        self.state.parent.mkdir(parents=True)
        self.state.symlink_to(real)
        with self.assertRaisesRegex(gate.GateError, "symlink"):
            gate.verify_state(fixture, self.state)

    def test_a_dangling_symlink_counts_as_a_corrupt_sibling(self) -> None:
        if not symlinks_supported(self.root):
            self.skipTest("symlinks are unavailable on this platform")
        fixture = self.fixture()
        gate.materialize_fixture(fixture, self.state)
        gate.corrupt_sibling(self.state).symlink_to(self.root / "absent")
        with self.assertRaisesRegex(gate.GateError, "quarantined a compatible"):
            gate.verify_state(fixture, self.state)

    def test_materialize_refuses_to_write_through_a_symlink(self) -> None:
        if not symlinks_supported(self.root):
            self.skipTest("symlinks are unavailable on this platform")
        target = self.root / "victim.txt"
        target.write_text("keep me", encoding="utf-8")
        self.state.parent.mkdir(parents=True)
        self.state.symlink_to(target)
        with self.assertRaisesRegex(gate.GateError, "non-regular path"):
            gate.materialize_fixture(self.fixture(), self.state)
        self.assertEqual(target.read_text(encoding="utf-8"), "keep me")

    # The two tests below simulate the race the ``lstat`` cannot close: a FIFO
    # put in place after the ``lstat`` said "regular file". ``lstat`` is
    # patched to say so, and the gate must still refuse the FIFO, through the
    # ``fstat`` on the opened descriptor, without hanging on the open.
    def _fifo_that_lstat_calls_regular(self) -> Any:
        if not hasattr(os, "mkfifo"):
            self.skipTest("FIFOs are unavailable on this platform")
        self.state.parent.mkdir(parents=True)
        os.mkfifo(self.state)
        regular = os.stat(self.root / "fixture-for-lstat", follow_symlinks=False)
        return mock.patch.object(gate.os, "lstat", return_value=regular)

    def test_a_fifo_swapped_in_after_lstat_is_refused_on_read(self) -> None:
        (self.root / "fixture-for-lstat").write_text("x", encoding="utf-8")
        with (
            self._fifo_that_lstat_calls_regular(),
            self.assertRaisesRegex(gate.GateError, "not a regular file"),
        ):
            gate.read_regular_file(self.state, 1024, "state")

    def test_a_fifo_swapped_in_after_lstat_is_refused_on_write(self) -> None:
        (self.root / "fixture-for-lstat").write_text("x", encoding="utf-8")
        with (
            self._fifo_that_lstat_calls_regular(),
            self.assertRaisesRegex(gate.GateError, "non-regular path"),
        ):
            gate.materialize_fixture(self.fixture(), self.state)

    def test_materialize_overwrites_an_existing_regular_file(self) -> None:
        self.state.parent.mkdir(parents=True)
        self.state.write_text(
            "older contents that are much longer than needed", encoding="utf-8"
        )
        gate.materialize_fixture(self.fixture(), self.state)
        gate.verify_state(self.fixture(), self.state)


class QuarantineTests(GateTestCase):
    def setUp(self) -> None:
        super().setUp()
        self.probe = self.root / "probe.json"
        self.state.parent.mkdir(parents=True)
        self.probe.write_bytes(b"invalid demo state\n")

    def test_a_matching_corrupt_sibling_passes(self) -> None:
        gate.corrupt_sibling(self.state).write_bytes(b"invalid demo state\n")
        # The host started fresh and persisted an empty state: still correct.
        self.state.write_bytes(envelope("demo-mosaic-app", 3, {}))
        gate.verify_quarantine(self.state, self.probe, "demo-app")

    def test_a_matching_corrupt_sibling_with_no_live_state_passes(self) -> None:
        gate.corrupt_sibling(self.state).write_bytes(b"invalid demo state\n")
        gate.verify_quarantine(self.state, self.probe)

    def test_a_differing_corrupt_sibling_fails(self) -> None:
        gate.corrupt_sibling(self.state).write_bytes(b"invalid demo state")
        with self.assertRaisesRegex(gate.GateError, "differs from the probe"):
            gate.verify_quarantine(self.state, self.probe, "demo-app")

    def test_a_missing_corrupt_sibling_fails(self) -> None:
        with self.assertRaisesRegex(gate.GateError, "quarantined state does not exist"):
            gate.verify_quarantine(self.state, self.probe)

    def test_a_copy_left_at_the_live_path_fails(self) -> None:
        gate.corrupt_sibling(self.state).write_bytes(b"invalid demo state\n")
        self.state.write_bytes(b"invalid demo state\n")
        with self.assertRaisesRegex(gate.GateError, "copied, not moved"):
            gate.verify_quarantine(self.state, self.probe)

    def test_a_symlinked_corrupt_sibling_is_refused(self) -> None:
        if not symlinks_supported(self.root):
            self.skipTest("symlinks are unavailable on this platform")
        gate.corrupt_sibling(self.state).symlink_to(self.probe)
        with self.assertRaisesRegex(gate.GateError, "symlink"):
            gate.verify_quarantine(self.state, self.probe)


class FixtureValidationTests(GateTestCase):
    def assert_rejected(self, document: Any, message: str) -> None:
        with self.assertRaisesRegex(gate.GateError, message):
            gate.parse_fixture(document)

    def test_the_minimal_fixture_is_valid(self) -> None:
        fixture = self.fixture()
        self.assertEqual(fixture.source_app, "demo-app")
        self.assertEqual(fixture.accepted_live_versions, (1, 3))
        self.assertEqual(fixture.expectations[0].live_versions, (1,))

    def test_a_legacy_or_foreign_schema_is_rejected_by_name(self) -> None:
        self.assert_rejected(
            fixture_document(fixtureSchema="task-app/release-upgrade-fixture"),
            "schema must be",
        )
        self.assert_rejected([], "must be a JSON object")

    def test_unknown_and_missing_keys_are_rejected(self) -> None:
        self.assert_rejected(fixture_document(expectation=[]), "unknown key")
        document = fixture_document()
        del document["acceptedLiveVersions"]
        self.assert_rejected(document, "missing acceptedLiveVersions")
        document = fixture_document()
        document["expectations"][0]["liveVersion"] = [1]
        self.assert_rejected(document, r"expectations\[0\] has unknown key")

    def test_field_types_are_enforced(self) -> None:
        cases = {
            "fixtureVersion": (2, "fixtureVersion"),
            "sourceApp": ("Demo App", "sourceApp"),
            "sourceVersion": ("v1.2.3", "strict SemVer"),
            "provenance": (7, "provenance"),
            "snapshotSchema": ("", "snapshotSchema"),
            "snapshotVersion": (True, "snapshotVersion"),
            "state": ([], "state must be a JSON object"),
            "expectations": ([], "non-empty list"),
        }
        for field, (value, message) in cases.items():
            with self.subTest(field):
                self.assert_rejected(fixture_document(**{field: value}), message)

    def test_accepted_versions_must_be_a_clean_list(self) -> None:
        for value, message in (
            ([], "non-empty list"),
            ([1, 1], "must not repeat"),
            ([1, -3], "non-negative"),
            ([1, False], "non-negative integers"),
        ):
            with self.subTest(value):
                self.assert_rejected(
                    fixture_document(acceptedLiveVersions=value), message
                )

    def test_expectation_versions_must_be_accepted(self) -> None:
        document = fixture_document()
        document["expectations"][1]["liveVersions"] = [2, 3]
        self.assert_rejected(document, r"version\(s\) \[2\]")

    def test_every_accepted_version_needs_an_expectation(self) -> None:
        document = fixture_document()
        document["expectations"] = document["expectations"][:1]
        self.assert_rejected(document, r"version\(s\) \[3\] have no applicable")

    def test_an_unscoped_expectation_covers_every_version(self) -> None:
        fixture = self.fixture(
            expectations=[{"pointer": "", "equals": {"decks": [{"name": "Tamil"}]}}],
            acceptedLiveVersions=[1],
        )
        gate.materialize_fixture(fixture, self.state)
        gate.verify_state(fixture, self.state)

    def test_malformed_expectations_are_rejected(self) -> None:
        for raw, message in (
            ("not an object", "must be an object"),
            ({"pointer": 3, "equals": 1}, "pointer must be a string"),
            ({"pointer": "decks", "equals": 1}, "start with '/'"),
            ({"pointer": "/a~2", "equals": 1}, "invalid '~' escape"),
        ):
            with self.subTest(raw):
                self.assert_rejected(
                    fixture_document(acceptedLiveVersions=[1], expectations=[raw]),
                    message,
                )

    def test_load_fixture_reads_a_file_and_refuses_bad_json(self) -> None:
        path = self.write_fixture(fixture_document())
        self.assertEqual(gate.load_fixture(path).source_version, "1.2.3")
        path.write_text("{", encoding="utf-8")
        with self.assertRaisesRegex(gate.GateError, "not JSON"):
            gate.load_fixture(path)


class PointerTests(unittest.TestCase):
    DOCUMENT: ClassVar[dict[str, Any]] = {
        "a/b": 1,
        "m~n": 2,
        "~1": 3,
        "": 4,
        "list": [10, 20, {"x": None}],
        "flag": False,
    }

    def test_escapes_unescape_in_the_rfc_order(self) -> None:
        self.assertEqual(gate.parse_pointer("/a~1b"), ["a/b"])
        self.assertEqual(gate.parse_pointer("/m~0n"), ["m~n"])
        # ~01 is the literal key "~1", not "/": ~1 is replaced first.
        self.assertEqual(gate.parse_pointer("/~01"), ["~1"])
        self.assertEqual(gate.parse_pointer(""), [])
        self.assertEqual(gate.parse_pointer("/"), [""])

    def test_resolution_handles_keys_indexes_and_null(self) -> None:
        resolve = gate.resolve_pointer
        self.assertEqual(resolve(self.DOCUMENT, "/a~1b"), 1)
        self.assertEqual(resolve(self.DOCUMENT, "/m~0n"), 2)
        self.assertEqual(resolve(self.DOCUMENT, "/~01"), 3)
        self.assertEqual(resolve(self.DOCUMENT, "/"), 4)
        self.assertEqual(resolve(self.DOCUMENT, "/list/1"), 20)
        self.assertIsNone(resolve(self.DOCUMENT, "/list/2/x"))
        self.assertIs(resolve(self.DOCUMENT, ""), self.DOCUMENT)

    def test_unresolvable_pointers_are_missing_not_errors(self) -> None:
        for pointer in (
            "/list/3",
            "/list/-",
            "/list/01",
            "/list/x",
            "/flag/0",
            "/absent",
        ):
            with self.subTest(pointer):
                self.assertIs(
                    gate.resolve_pointer(self.DOCUMENT, pointer), gate._MISSING
                )

    def test_pointer_text_is_never_evaluated(self) -> None:
        hostile = "/__import__('os').system('exit 1')"
        self.assertIs(gate.resolve_pointer(self.DOCUMENT, hostile), gate._MISSING)


class JsonEqualityTests(unittest.TestCase):
    def test_equality_is_json_strict(self) -> None:
        self.assertFalse(gate.json_equal(True, 1))
        self.assertFalse(gate.json_equal(0, False))
        self.assertFalse(gate.json_equal(1, "1"))
        self.assertFalse(gate.json_equal(None, False))
        self.assertFalse(gate.json_equal(float("nan"), float("nan")))
        self.assertFalse(gate.json_equal([1], [1, 2]))
        self.assertFalse(gate.json_equal({"a": 1}, {"b": 1}))
        self.assertFalse(gate.json_equal({"a": True}, {"a": 1}))
        self.assertTrue(gate.json_equal(1, 1.0))
        self.assertTrue(gate.json_equal(False, False))
        self.assertTrue(gate.json_equal(None, None))
        self.assertTrue(
            gate.json_equal({"a": [1, {"b": None}]}, {"a": [1, {"b": None}]})
        )


class CommandLineTests(GateTestCase):
    def run_cli(self, *argv: str) -> tuple[int, str]:
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            status = gate.main(list(argv))
        return status, stderr.getvalue()

    def test_materialize_verify_and_quarantine_through_the_cli(self) -> None:
        fixture = str(self.write_fixture(fixture_document()))
        self.assertEqual(
            self.run_cli(
                "materialize", "--fixture", fixture, "--output", str(self.state)
            ),
            (0, ""),
        )
        self.assertEqual(
            self.run_cli("verify", "--fixture", fixture, "--state", str(self.state)),
            (0, ""),
        )

        probe = self.root / "probe.json"
        probe.write_bytes(b"garbage\n")
        os.replace(self.state, gate.corrupt_sibling(self.state))
        gate.corrupt_sibling(self.state).write_bytes(b"garbage\n")
        status, _ = self.run_cli(
            "verify-quarantine",
            "--state",
            str(self.state),
            "--probe",
            str(probe),
            "--app",
            "demo-app",
        )
        self.assertEqual(status, 0)

    def test_failures_exit_one_with_a_single_error_line(self) -> None:
        fixture = str(self.write_fixture(fixture_document()))
        status, message = self.run_cli(
            "verify", "--fixture", fixture, "--state", str(self.state)
        )
        self.assertEqual(status, 1)
        self.assertTrue(
            message.startswith("error: demo-app: upgrade state does not exist")
        )
        self.assertEqual(message.count("\n"), 1)


class EngramFixtureTests(GateTestCase):
    """The committed Engram v0.4.0 fixture, end to end.

    The Rust test ``the_v0_4_0_release_fixture_survives_the_upgrade`` in
    ``engram-mosaic-app`` is what proves these pointers describe the *real*
    current v3 bytes. Here the v3 file is a simulation built from the fixture's
    own state, which is enough to show the Python half accepts both outcomes a
    host can leave on disk: untouched v1, and re-persisted v3.
    """

    def setUp(self) -> None:
        super().setUp()
        self.fixture = gate.load_fixture(ENGRAM_FIXTURE)

    def test_fixture_identity(self) -> None:
        self.assertEqual(self.fixture.source_app, "engram")
        self.assertEqual(self.fixture.source_version, "0.4.0")
        self.assertEqual(self.fixture.snapshot_schema, "engram-mosaic-app")
        self.assertEqual(self.fixture.snapshot_version, 1)
        self.assertEqual(self.fixture.accepted_live_versions, (1, 3))

    def test_an_untouched_v1_file_verifies(self) -> None:
        gate.materialize_fixture(self.fixture, self.state)
        gate.verify_state(self.fixture, self.state)

    def test_a_re_persisted_v3_file_verifies(self) -> None:
        # The v3 shape: the facade's document beside the adapter's own field.
        v3 = {
            "session": {"state": copy.deepcopy(self.fixture.state), "cursor": {}},
            "adapterSelectedDeckId": "",
        }
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(envelope("engram-mosaic-app", 3, v3))
        gate.verify_state(self.fixture, self.state)

    def test_a_v3_file_that_lost_the_card_fails(self) -> None:
        state = copy.deepcopy(self.fixture.state)
        state["cards"] = []
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(
            envelope("engram-mosaic-app", 3, {"session": {"state": state}})
        )
        with self.assertRaisesRegex(
            gate.GateError, "nothing at '/session/state/cards/0"
        ):
            gate.verify_state(self.fixture, self.state)

    def test_an_unexplained_v2_file_fails(self) -> None:
        self.state.parent.mkdir(parents=True)
        self.state.write_bytes(envelope("engram-mosaic-app", 2, self.fixture.state))
        with self.assertRaisesRegex(gate.GateError, "snapshot version to 2"):
            gate.verify_state(self.fixture, self.state)


if __name__ == "__main__":
    unittest.main()
