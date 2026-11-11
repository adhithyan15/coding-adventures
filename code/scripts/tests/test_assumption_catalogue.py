"""Tests for `assumption_catalogue.py`, the integrity checker of the VER13 catalogue.

The catalogue is data that reports will pin by SHA-256, so the checker is the
only thing standing between a hand edit and a silently different set of
assumptions.  Every rule therefore has a *negative* test: a deliberately broken
copy of the real catalogue (or the real spec) that must be rejected with the
rule's id.  A rule with only a positive test would pass if the rule were deleted.

The final tests run the checker over the real repository files, which is what
makes `AC-14` (spec section 3 equals the JSON) a standing CI gate.
"""

from __future__ import annotations

import contextlib
import copy
import io
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import assumption_catalogue as ac
import generate_assumption_catalogue as gen

SPEC_TEXT = ac.SPEC_PATH.read_text(encoding="utf-8")
BACKLOG_TEXT = ac.BACKLOG_PATH.read_text(encoding="utf-8")
RAW = ac.CATALOGUE_PATH.read_bytes()
REAL = json.loads(RAW)


def fresh() -> dict:
    """A private deep copy of the real catalogue to break."""
    return copy.deepcopy(REAL)


def entry(data: dict, entry_id: str) -> dict:
    return next(e for e in data["assumptions"] if e["id"] == entry_id)


def run(data: dict, spec: str | None = SPEC_TEXT, backlog: str | None = BACKLOG_TEXT):
    return ac.check_catalogue(json.dumps(data).encode(), spec, backlog)[0]


def rules(findings) -> set[str]:
    return {f.rule for f in findings}


def messages(findings) -> str:
    return "\n".join(str(f) for f in findings)


class RealRepositoryTests(unittest.TestCase):
    """The committed files must satisfy every catalogue-only rule."""

    def test_real_catalogue_has_no_findings(self):
        findings, digest = ac.check_catalogue(RAW, SPEC_TEXT, BACKLOG_TEXT)
        self.assertEqual([], findings, messages(findings))
        self.assertRegex(digest, r"^[0-9a-f]{64}$")

    def test_has_the_84_entries_the_spec_promises(self):
        self.assertEqual(84, len(REAL["assumptions"]))

    def test_committed_json_is_what_the_generator_produces(self):
        # Fails when someone edits the JSON by hand or the generator overlay
        # without regenerating; the JSON must stay reproducible.
        expected = gen.render(gen.build_catalogue(SPEC_TEXT))
        self.assertEqual(expected, RAW.decode("utf-8"))

    def test_generator_check_mode(self):
        self.assertEqual(0, gen.main(["--check"]))
        with tempfile.TemporaryDirectory() as tmp:
            stale = Path(tmp) / "stale.json"
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                self.assertEqual(
                    1, gen.main(["--check", "--out", str(stale)])
                )  # missing
                stale.write_text("{}\n", encoding="utf-8")
                self.assertEqual(
                    1, gen.main(["--check", "--out", str(stale)])
                )  # different
            self.assertIn("stale", stderr.getvalue())

    def test_generator_writes_a_loadable_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "nested" / "catalogue.json"
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(0, gen.main(["--out", str(out)]))
            self.assertEqual(RAW, out.read_bytes())

    def test_legacy_ids_and_attestation_aliases(self):
        legacy = {e["id"] for e in REAL["assumptions"] if e["legacy"]}
        self.assertEqual(
            {
                "compiler.preserves_timing",
                "memory.unaliased",
                "wir.bound.heap-unchecked",
                "wir.bound.depth-unchecked",
            },
            legacy,
        )
        self.assertIn(
            "heap-bound:unchecked", entry(REAL, "wir.bound.heap-unchecked")["aliases"]
        )
        self.assertIn(
            "depth-bound:unchecked", entry(REAL, "wir.bound.depth-unchecked")["aliases"]
        )
        att = [e for e in REAL["assumptions"] if e["kind"] == "attestation"]
        self.assertEqual(9, len(att))
        for e in att:
            self.assertEqual("attested", e["status"])
            self.assertEqual(
                1, len([a for a in e["aliases"] if a.startswith("unverified::")])
            )

    def test_umbrella_refinements(self):
        umbrella = entry(REAL, "compiler.preserves_timing")
        self.assertEqual(7, len(umbrella["refined_by"]))
        for child in umbrella["refined_by"]:
            self.assertEqual("compiler.preserves_timing", entry(REAL, child)["refines"])

    def test_schema_file_vocabularies_match_the_checker(self):
        schema = json.loads(
            (
                REPO
                / "code"
                / "specs"
                / "schemas"
                / "ver13-assumption-catalogue-v1.schema.json"
            ).read_text()
        )
        item = schema["properties"]["assumptions"]["items"]
        props = item["properties"]
        self.assertEqual(set(ac.ENTRY_REQUIRED), set(item["required"]))
        self.assertEqual(set(ac.ENTRY_REQUIRED) | set(ac.ENTRY_OPTIONAL), set(props))
        self.assertEqual(list(ac.LAYERS), props["layer"]["enum"])
        self.assertEqual(list(ac.FAMILIES), props["family"]["enum"])
        self.assertEqual(list(ac.KINDS), props["kind"]["enum"])
        self.assertEqual(list(ac.STATUSES), props["status"]["enum"])
        self.assertEqual(list(ac.TIERS), props["tier"]["enum"])
        self.assertEqual(
            list(ac.PROOF_LABELS),
            props["applies_to"]["properties"]["labels"]["items"]["enum"],
        )
        self.assertEqual(
            list(ac.CLAIM_KINDS),
            props["applies_to"]["properties"]["claims"]["items"]["enum"],
        )
        discharge = props["discharge"]["oneOf"][1]
        self.assertEqual(list(ac.RUNGS), discharge["properties"]["rung"]["enum"])
        self.assertEqual(
            list(ac.DIFFICULTIES), discharge["properties"]["difficulty"]["enum"]
        )
        self.assertEqual(set(ac.DISCHARGE_REQUIRED), set(discharge["required"]))

    def test_catalogue_validates_against_the_json_schema(self):
        try:
            import jsonschema
        except ImportError:  # the checker itself needs no third-party package
            self.skipTest("jsonschema not installed")
        path = (
            REPO
            / "code"
            / "specs"
            / "schemas"
            / "ver13-assumption-catalogue-v1.schema.json"
        )
        jsonschema.validate(REAL, json.loads(path.read_text()))


class StrictLoadingTests(unittest.TestCase):
    def test_duplicate_json_key_is_rejected(self):
        text = '{"schema": "x", "schema": "y", "assumptions": []}'
        with self.assertRaisesRegex(ac.CatalogueError, "duplicate JSON key 'schema'"):
            ac.parse_catalogue_text(text)

    def test_duplicate_key_inside_an_entry_is_rejected(self):
        text = RAW.decode().replace(
            '"status": "assumed"', '"status": "assumed", "status": "floor"', 1
        )
        with self.assertRaisesRegex(ac.CatalogueError, "duplicate JSON key"):
            ac.check_catalogue(text.encode(), SPEC_TEXT, BACKLOG_TEXT)

    def test_invalid_json_nan_and_non_object_root(self):
        with self.assertRaisesRegex(ac.CatalogueError, "invalid JSON"):
            ac.parse_catalogue_text("{not json")
        with self.assertRaisesRegex(ac.CatalogueError, "non-JSON constant"):
            ac.parse_catalogue_text('{"schema": NaN}')
        with self.assertRaisesRegex(ac.CatalogueError, "root must be a JSON object"):
            ac.parse_catalogue_text("[]")

    def test_non_utf8_is_rejected(self):
        with self.assertRaisesRegex(ac.CatalogueError, "not UTF-8"):
            ac.check_catalogue(b"\xff\xfe", None, None)

    def test_digest_is_sha256_of_exact_bytes(self):
        import hashlib

        self.assertEqual(hashlib.sha256(RAW).hexdigest(), ac.catalogue_digest(RAW))
        # One extra byte of whitespace changes the pin: the digest is not canonicalised.
        self.assertNotEqual(ac.catalogue_digest(RAW), ac.catalogue_digest(RAW + b"\n"))


class StructureTests(unittest.TestCase):
    """Unknown fields, bad types and out-of-vocabulary values all fail closed."""

    def expect(self, data: dict, needle: str):
        findings = run(data)
        self.assertEqual({"schema"}, rules(findings), messages(findings))
        self.assertIn(needle, messages(findings))

    def test_root_problems(self):
        data = fresh()
        data["extra"] = 1
        self.expect(data, "unknown field 'extra'")
        data = fresh()
        data["schema"] = "other"
        self.expect(data, "schema must be")
        data = fresh()
        data["assumptions"] = {}
        self.expect(data, "assumptions must be a list")
        data = fresh()
        del data["schema"]
        self.expect(data, "missing field 'schema'")

    def test_entry_that_is_not_an_object(self):
        data = fresh()
        data["assumptions"].append("oops")
        self.expect(data, "entry must be an object")

    def test_unknown_and_missing_entry_fields(self):
        data = fresh()
        entry(data, "memory.unaliased")["surprise"] = True
        self.expect(data, "unknown field 'surprise'")
        data = fresh()
        del entry(data, "memory.unaliased")["status"]
        self.expect(data, "missing field 'status'")

    def test_entry_without_string_id(self):
        data = fresh()
        data["assumptions"][0]["id"] = 5
        self.expect(data, "id must match")

    def test_bad_id_characters(self):
        data = fresh()
        entry(data, "memory.unaliased")["id"] = "Bad Id"
        self.expect(data, "id must match")

    def test_scalar_type_errors(self):
        for field in ("title", "statement", "since", "used_by"):
            data = fresh()
            entry(data, "memory.unaliased")[field] = 3
            self.expect(data, f"{field} must be a string")
        data = fresh()
        entry(data, "memory.unaliased")["title"] = "  "
        self.expect(data, "title must not be empty")
        data = fresh()
        entry(data, "memory.unaliased")["legacy"] = "yes"
        self.expect(data, "legacy must be a boolean")
        data = fresh()
        entry(data, "memory.unaliased")["notes"] = 1
        self.expect(data, "notes must be a string")

    def test_vocabulary_errors(self):
        for field, bad in (
            ("layer", "kernel"),
            ("family", "zzz"),
            ("kind", "vibes"),
            ("status", "maybe"),
            ("tier", "O9"),
        ):
            data = fresh()
            entry(data, "memory.unaliased")[field] = bad
            self.expect(data, f"{field} {bad!r}")

    def test_reference_field_shapes(self):
        data = fresh()
        entry(data, "memory.unaliased")["aliases"] = "x"
        self.expect(data, "aliases must be a list")
        data = fresh()
        entry(data, "memory.unaliased")["aliases"] = ["Bad Alias"]
        self.expect(data, "alias 'Bad Alias'")
        data = fresh()
        entry(data, "memory.unaliased")["refines"] = 3
        self.expect(data, "refines must be an id string or null")
        data = fresh()
        entry(data, "memory.unaliased")["depends_on"] = "a"
        self.expect(data, "depends_on must be a list")
        data = fresh()
        entry(data, "memory.unaliased")["constrains_input_classes"] = ["vibes"]
        self.expect(data, "constrains_input_classes has unknown value")
        data = fresh()
        entry(data, "memory.unaliased")["constrains_input_classes"] = "hostile"
        self.expect(data, "constrains_input_classes must be a list")

    def test_applies_to_errors(self):
        data = fresh()
        entry(data, "memory.unaliased")["applies_to"] = []
        self.expect(data, "applies_to must be an object")
        data = fresh()
        entry(data, "memory.unaliased")["applies_to"]["extra"] = []
        self.expect(data, "applies_to: unknown field")
        data = fresh()
        entry(data, "memory.unaliased")["applies_to"]["labels"] = ["proved-ish"]
        self.expect(data, "applies_to.labels has unknown value")
        data = fresh()
        entry(data, "memory.unaliased")["applies_to"]["claims"] = "x"
        self.expect(data, "applies_to.claims must be a list")
        data = fresh()
        entry(data, "memory.unaliased")["applies_to"]["packages"] = ["Not A Package"]
        self.expect(data, "applies_to.packages bad entry")
        data = fresh()
        entry(data, "memory.unaliased")["applies_to"]["packages"] = "rust/x"
        self.expect(data, "applies_to.packages must be a list")

    def test_discharge_errors(self):
        data = fresh()
        entry(data, "memory.unaliased")["discharge"] = "later"
        self.expect(data, "discharge must be an object or null")
        data = fresh()
        entry(data, "memory.unaliased")["discharge"]["extra"] = 1
        self.expect(data, "discharge: unknown field")
        cases = (
            ("evidence_kinds", "x", "evidence_kinds must be a list"),
            ("rung", "R9", "discharge.rung"),
            ("tools", "x", "tools must be a list"),
            ("difficulty", "XXL", "discharge.difficulty"),
            ("backlog", "not a backlog id", "is not a backlog id"),
            ("backlog", 7, "is not a backlog id"),
            ("plan", "  ", "plan must be a non-empty string"),
        )
        for field, bad, needle in cases:
            data = fresh()
            entry(data, "memory.unaliased")["discharge"][field] = bad
            self.expect(data, needle)

    def test_discharged_by_errors(self):
        good = {
            "evidence": "e",
            "digest": "sha256:" + "0" * 64,
            "stack": "s",
            "pinned": {},
        }
        for mutate, needle in (
            (lambda g: "nope", "discharged_by must be a list"),
            (lambda g: ["nope"], "items must be objects"),
            (lambda g: [{**g, "extra": 1}], "unknown field"),
            (lambda g: [{**g, "evidence": ""}], "evidence must be a non-empty"),
            (lambda g: [{**g, "digest": "sha256:xyz"}], "digest must be sha256"),
            (lambda g: [{**g, "stack": ""}], "stack must be a non-empty"),
            (lambda g: [{**g, "pinned": []}], "pinned must be an object"),
        ):
            data = fresh()
            entry(data, "memory.unaliased")["discharged_by"] = mutate(good)
            findings = run(data)
            self.assertIn("schema", rules(findings))
            self.assertIn(needle, messages(findings))


class IdentityAndGraphTests(unittest.TestCase):
    """AC-14: ids unique, references resolve, relations acyclic."""

    def test_duplicate_id(self):
        data = fresh()
        data["assumptions"].append(copy.deepcopy(entry(data, "memory.unaliased")))
        findings = run(data, spec=None)
        self.assertIn("duplicate id", messages(findings))
        self.assertEqual({"AC-14"}, rules(findings))

    def test_alias_collides_with_an_id(self):
        data = fresh()
        entry(data, "wir.bound.heap-unchecked")["aliases"].append("memory.unaliased")
        self.assertIn("collides with an id", messages(run(data, spec=None)))

    def test_alias_owned_twice(self):
        data = fresh()
        entry(data, "wir.bound.depth-unchecked")["aliases"].append(
            "heap-bound:unchecked"
        )
        self.assertIn("also owned by", messages(run(data, spec=None)))

    def test_dangling_depends_on(self):
        data = fresh()
        entry(data, "memory.unaliased")["depends_on"] = ["wir.no-such-thing"]
        findings = run(data, spec=None)
        self.assertIn(
            "depends_on names unknown id 'wir.no-such-thing'", messages(findings)
        )

    def test_dangling_refines_and_refined_by(self):
        data = fresh()
        entry(data, "memory.unaliased")["refines"] = "ghost"
        entry(data, "wir.inline.sound")["refined_by"] = ["ghost2"]
        text = messages(run(data, spec=None))
        self.assertIn("refines names unknown id 'ghost'", text)
        self.assertIn("refined_by names unknown id 'ghost2'", text)

    def test_self_reference(self):
        data = fresh()
        entry(data, "memory.unaliased")["depends_on"] = ["memory.unaliased"]
        self.assertIn(
            "cannot depend on or refine itself", messages(run(data, spec=None))
        )

    def test_depends_on_cycle(self):
        data = fresh()
        entry(data, "wir.memory.provenance-model")["depends_on"] = ["memory.unaliased"]
        findings = run(data, spec=None)
        self.assertIn("depends_on cycle", messages(findings))
        self.assertIn("memory.unaliased", messages(findings))

    def test_refines_cycle(self):
        data = fresh()
        entry(data, "compiler.preserves_timing")["refines"] = "cc.version-pinned"
        entry(data, "cc.version-pinned")["refined_by"] = ["compiler.preserves_timing"]
        entry(data, "cc.version-pinned")["refines"] = "compiler.preserves_timing"
        entry(data, "compiler.preserves_timing")["refined_by"].append(
            "cc.version-pinned"
        )
        self.assertIn("refines cycle", messages(run(data, spec=None)))

    def test_union_cycle_through_both_relations(self):
        # A child that depends on a sibling which depends back on the umbrella's
        # child is fine; a child that *depends on its umbrella* closes a loop
        # only in the union graph (umbrella -> child via depends_on).
        data = fresh()
        entry(data, "compiler.preserves_timing")["depends_on"] = ["cc.version-pinned"]
        findings = run(data, spec=None)
        self.assertIn("depends_on+refines cycle", messages(findings))
        self.assertNotIn(
            "depends_on cycle:",
            messages(findings).replace("depends_on+refines cycle", ""),
        )

    def test_refines_and_refined_by_must_agree(self):
        data = fresh()
        entry(data, "compiler.preserves_timing")["refined_by"].remove(
            "cc.version-pinned"
        )
        self.assertIn("is not in its refined_by", messages(run(data, spec=None)))
        data = fresh()
        entry(data, "cc.caller-context")["refines"] = None
        self.assertIn("whose refines is not", messages(run(data, spec=None)))

    def test_find_cycle_helper(self):
        self.assertIsNone(ac.find_cycle({"a": ["b"], "b": [], "c": ["a", "missing"]}))
        self.assertEqual(["a", "b", "a"], ac.find_cycle({"a": ["b"], "b": ["a"]}))
        self.assertEqual(["c", "c"], ac.find_cycle({"a": [], "c": ["c"]}))
        # A deep chain must not hit Python's recursion limit.
        chain = {str(i): [str(i + 1)] for i in range(5000)}
        chain["5000"] = []
        self.assertIsNone(ac.find_cycle(chain))


class BacklogTests(unittest.TestCase):
    def test_unknown_backlog_id(self):
        data = fresh()
        entry(data, "memory.unaliased")["discharge"]["backlog"] = "ZZ-999"
        findings = run(data)
        self.assertIn("'ZZ-999' is not in VER-BACKLOG.md", messages(findings))
        self.assertEqual({"AC-14"}, rules(findings))

    def test_every_real_backlog_reference_exists(self):
        ids = ac.parse_backlog_ids(BACKLOG_TEXT)
        for e in REAL["assumptions"]:
            if e["discharge"]:
                self.assertIn(e["discharge"]["backlog"], ids)

    def test_parse_backlog_ids_shapes(self):
        text = (
            "| Priority / ID | Item |\n|---|---|\n"
            "| P1 / AS-001a | land it |\n| done-local / AS-000 | done |\n"
            "| AF-01 | no priority cell |\n| P2 / WD-1xx | wildcard row |\n"
            "not a table row AS-777\n| plain | row |"
        )
        self.assertEqual(
            {"AS-001a", "AS-000", "AF-01", "WD-1xx"}, ac.parse_backlog_ids(text)
        )


class ShapeRuleTests(unittest.TestCase):
    """Catalogue-side halves of AC-8, AC-3, AC-11 and AC-15."""

    def test_ac8_declared_must_be_floor(self):
        data = fresh()
        entry(data, "phys.side-channels.out-of-scope")["status"] = "assumed"
        self.assertIn("AC-8", rules(run(data, spec=None)))

    def test_ac8_floor_needs_null_discharge_and_vice_versa(self):
        data = fresh()
        e = entry(data, "phys.silicon.realization")
        e["discharge"] = copy.deepcopy(entry(data, "memory.unaliased")["discharge"])
        self.assertIn("exactly when status is floor", messages(run(data, spec=None)))
        data = fresh()
        entry(data, "memory.unaliased")["discharge"] = None
        self.assertIn("exactly when status is floor", messages(run(data, spec=None)))

    def test_ac8_physical_or_declared_never_discharged(self):
        data = fresh()
        e = entry(data, "phys.silicon.realization")
        e["status"] = "discharged"
        e["discharged_by"] = [
            {
                "evidence": "e",
                "digest": "sha256:" + "a" * 64,
                "stack": "s",
                "pinned": {},
            }
        ]
        self.assertIn("can never be discharged", messages(run(data, spec=None)))

    def test_ac3_discharged_requires_evidence(self):
        data = fresh()
        entry(data, "memory.unaliased")["status"] = "discharged"
        self.assertIn(
            "requires non-empty discharged_by", messages(run(data, spec=None))
        )

    def test_ac3_valid_discharge_shape_is_accepted_but_unevidenced_status_is_not(self):
        data = fresh()
        e = entry(data, "memory.unaliased")
        e["status"] = "discharged"
        e["discharged_by"] = [
            {
                "evidence": "warrant/x",
                "digest": "sha256:" + "b" * 64,
                "stack": "host",
                "pinned": {},
            }
        ]
        self.assertEqual([], run(data, spec=None))
        data = fresh()
        entry(data, "memory.unaliased")["discharged_by"] = [
            {
                "evidence": "warrant/x",
                "digest": "sha256:" + "b" * 64,
                "stack": "host",
                "pinned": {},
            }
        ]
        self.assertIn(
            "only allowed when status is discharged", messages(run(data, spec=None))
        )

    def test_ac11_attestation_alias_rules(self):
        data = fresh()
        e = entry(data, "att.ast-audit.no-deserialization")
        e["aliases"] = []
        self.assertIn("exactly one unverified:: alias", messages(run(data, spec=None)))
        data = fresh()
        entry(data, "memory.unaliased")["aliases"] = ["unverified::foo::bar"]
        self.assertIn("only attestations may own", messages(run(data, spec=None)))
        data = fresh()
        entry(data, "att.ast-audit.no-deserialization")["status"] = "evidenced"
        self.assertIn("go together", messages(run(data, spec=None)))

    def test_ac15_environment_may_not_constrain_hostile_input(self):
        data = fresh()
        entry(data, "uarch.cache.address-trace-suffices")[
            "constrains_input_classes"
        ] = ["hostile"]
        self.assertIn("AC-15", rules(run(data, spec=None)))
        data = fresh()
        entry(data, "spec.obs-model.complete")["constrains_input_classes"] = ["hostile"]
        self.assertIn("AC-15", rules(run(data, spec=None)))
        # A non-environment, non-spec entry may talk about hostile inputs.
        data = fresh()
        entry(data, "src.rust.int-semantics")["constrains_input_classes"] = ["hostile"]
        self.assertEqual([], run(data, spec=None))

    def test_family_must_match_id(self):
        data = fresh()
        entry(data, "memory.unaliased")["family"] = "src"
        self.assertIn("does not match id prefix", messages(run(data, spec=None)))
        data = fresh()
        entry(data, "att.ast-audit.no-deserialization")["family"] = "att"
        self.assertEqual([], run(data, spec=None))


class SpecSyncTests(unittest.TestCase):
    """AC-14 `CatalogueDrift`: the spec's section 3 and the JSON must agree."""

    def drift(self, data: dict, spec: str = SPEC_TEXT) -> str:
        findings = run(data, spec=spec)
        self.assertIn("AC-14", rules(findings), messages(findings))
        text = messages(findings)
        self.assertIn("CatalogueDrift", text)
        return text

    def test_json_entry_missing_from_spec_rows(self):
        data = fresh()
        data["assumptions"] = [
            e for e in data["assumptions"] if e["id"] != "gov.catalogue.integrity"
        ]
        # Removing it also unbalances every count.
        text = self.drift(data)
        self.assertIn(
            "gov.catalogue.integrity: CatalogueDrift: in spec section 3 but not in the JSON",
            text,
        )
        self.assertIn("spec says 84 entries, JSON has 83", text)
        self.assertIn("<family gov>", text)
        self.assertIn("<layer os/gov>", text)

    def test_json_entry_not_in_spec(self):
        data = fresh()
        extra = copy.deepcopy(entry(data, "gov.catalogue.integrity"))
        extra["id"] = "gov.invented"
        data["assumptions"].append(extra)
        self.assertIn(
            "gov.invented: CatalogueDrift: in the JSON but not in spec section 3",
            self.drift(data),
        )

    def test_spec_row_removed_changes_count_and_ids(self):
        spec = SPEC_TEXT.replace(
            "| `gov.catalogue.integrity` |", "| `gov.catalogue.removed` |", 1
        )
        text = self.drift(fresh(), spec)
        self.assertIn("gov.catalogue.removed", text)
        self.assertIn("gov.catalogue.integrity", text)

    def test_count_drift_in_prose(self):
        spec = SPEC_TEXT.replace("**84 entries**", "**85 entries**")
        text = self.drift(fresh(), spec)
        self.assertIn("spec says 85 entries, JSON has 84", text)
        self.assertIn("its tables have 84", text)

    def test_family_and_layer_count_drift(self):
        spec = SPEC_TEXT.replace("`wir` 12", "`wir` 13").replace("src 27", "src 28")
        text = self.drift(fresh(), spec)
        self.assertIn("<family wir>: CatalogueDrift: spec says 13, JSON has 12", text)
        self.assertIn("<layer src>: CatalogueDrift: spec says 28, JSON has 27", text)

    def test_legacy_and_attested_count_drift(self):
        spec = SPEC_TEXT.replace("**4 are legacy ids", "**5 are legacy ids").replace(
            "9 are\nthe existing `unverified::` ids",
            "8 are\nthe existing `unverified::` ids",
        )
        text = self.drift(fresh(), spec)
        self.assertIn("spec says 5 legacy ids, JSON has 4", text)
        self.assertIn("spec says 8 unverified ids, JSON has 9", text)

    def test_field_drift_per_entry(self):
        data = fresh()
        entry(data, "memory.unaliased")["statement"] = "rewritten"
        entry(data, "memory.unaliased")["tier"] = "O5"
        entry(data, "memory.unaliased")["discharge"]["difficulty"] = "XL"
        entry(data, "memory.unaliased")["discharge"]["tools"] = ["something-else"]
        entry(data, "memory.unaliased")["discharge"]["plan"] = "other plan"
        entry(data, "memory.unaliased")["used_by"] = "nobody"
        entry(data, "memory.unaliased")["layer"] = "mc"
        entry(data, "memory.unaliased")["legacy"] = False
        text = self.drift(data)
        for field in (
            "statement",
            "tier",
            "discharge.difficulty",
            "discharge.tools",
            "discharge.plan",
            "used_by",
            "layer",
            "legacy",
        ):
            self.assertIn(f"{field} is", text)

    def test_floor_entry_must_match_spec_difficulty(self):
        data = fresh()
        e = entry(data, "memory.unaliased")
        e["status"] = "floor"
        e["discharge"] = None
        self.assertIn("spec difficulty M but JSON has no discharge", self.drift(data))

    def test_missing_legacy_alias(self):
        data = fresh()
        entry(data, "wir.bound.heap-unchecked")["aliases"] = []
        self.assertIn(
            "alias 'heap-bound:unchecked' from the spec row is missing",
            self.drift(data),
        )

    def test_order_drift(self):
        data = fresh()
        data["assumptions"][0], data["assumptions"][1] = (
            data["assumptions"][1],
            data["assumptions"][0],
        )
        self.assertIn("<order>", self.drift(data))

    def test_duplicate_spec_row(self):
        row = next(
            ln
            for ln in SPEC_TEXT.splitlines()
            if ln.startswith("| `gov.catalogue.integrity`")
        )
        spec = SPEC_TEXT.replace(row, row + "\n" + row, 1)
        self.assertIn("appears twice in spec section 3", self.drift(fresh(), spec))

    def test_exemption_table_ids_must_exist(self):
        spec = SPEC_TEXT.replace(
            "`mc.isa.timing-data-independent`, `uarch.*`",
            "`mc.isa.bogus`, `nothing.*`",
            1,
        )
        text = self.drift(fresh(), spec)
        self.assertIn(
            "mc.isa.bogus: CatalogueDrift: exemption table names an id absent", text
        )
        self.assertIn(
            "nothing.*: CatalogueDrift: exemption wildcard matches no catalogue id",
            text,
        )

    def test_malformed_spec_inputs(self):
        with self.assertRaisesRegex(ac.CatalogueError, "no '## 3. Inventory'"):
            ac.parse_spec_inventory("# nothing here")
        bad_cells = SPEC_TEXT.replace(
            "| `gov.catalogue.integrity` |", "| `gov.catalogue.integrity` | extra |", 1
        )
        with self.assertRaisesRegex(ac.CatalogueError, "row has 8 cells"):
            ac.parse_spec_inventory(bad_cells)
        no_tick = "## 3. Inventory\n### 3.1 x\n| `a` | b | c | d | e | S | O1 |\n"
        self.assertEqual(1, len(ac.parse_spec_inventory(no_tick)))
        broken_id = "## 3. Inventory\n### 3.1 x\n| `` | b | c | d | e | S | O1 |\n"
        with self.assertRaisesRegex(ac.CatalogueError, "not backticked"):
            ac.parse_spec_inventory(broken_id)
        with self.assertRaisesRegex(ac.CatalogueError, "no section 3.9"):
            ac.parse_spec_counts("## 3. Inventory\n")
        with self.assertRaisesRegex(ac.CatalogueError, "size sentences not recognised"):
            ac.parse_spec_counts("### 3.9 Size\n\nnothing\n\n## 4. x\n")
        with self.assertRaisesRegex(ac.CatalogueError, "exemptions table not found"):
            ac.parse_spec_exemption_ids("nothing")

    def test_helper_normalisers(self):
        self.assertEqual("L", ac.normalise_difficulty("S (ct-compare) / L (general)"))
        self.assertEqual("N/A", ac.normalise_difficulty("N/A at WIR"))
        self.assertEqual("N/A", ac.normalise_difficulty(""))
        self.assertEqual("XL", ac.normalise_difficulty("XL"))
        self.assertEqual("x y", ac.normalise_statement("*(attested: `u::x`)* x y"))
        self.assertEqual("plain", ac.normalise_statement("  plain "))
        self.assertEqual((), ac.split_tools("n/a"))
        self.assertEqual((), ac.split_tools("same"))
        self.assertEqual(("a (b, c)", "d"), ac.split_tools("a (b, c); d"))
        self.assertEqual(("`x`"[1:-1], "y"), ac.split_tools("`x`, y"))
        self.assertEqual("cc", ac.family_of("compiler.preserves_timing"))
        self.assertEqual("wir", ac.family_of("memory.unaliased"))
        self.assertEqual("att", ac.family_of("att.x.y"))
        self.assertEqual("os", ac.layer_of("os.x", "8"))
        self.assertEqual("spec", ac.layer_of("spec.x", "1"))


class OrchestrationTests(unittest.TestCase):
    def test_structural_errors_short_circuit_later_rules(self):
        data = fresh()
        del entry(data, "memory.unaliased")["status"]
        findings = run(data)
        self.assertEqual({"schema"}, rules(findings))

    def test_skipping_spec_and_backlog(self):
        findings, _ = ac.check_catalogue(RAW, None, None)
        self.assertEqual([], findings)

    def test_finding_str(self):
        self.assertEqual("[AC-14] x: y", str(ac.Finding("AC-14", "x", "y")))

    def test_generator_overlay_covers_every_entry(self):
        # Every id in the spec must have evidence kinds unless it is a floor entry;
        # a new spec row without an overlay must fail loudly, not default silently.
        rows = ac.parse_spec_inventory(SPEC_TEXT)
        for row in rows:
            if row.id not in gen.FLOOR:
                self.assertIn(row.id, gen.EVIDENCE_KINDS, row.id)
        stale = set(gen.EVIDENCE_KINDS) - {r.id for r in rows}
        self.assertEqual(set(), stale)
        for table in (
            gen.KIND_BY_ID,
            gen.DEPENDS_ON,
            gen.BACKLOG_BY_ID,
            gen.CLAIM_OVERRIDES,
            gen.LABEL_OVERRIDES,
            gen.NOTES,
            gen.SINCE,
        ):
            self.assertEqual(
                set(),
                set(table) - {r.id for r in rows},
                "overlay names an id absent from the spec",
            )
        self.assertEqual(set(), gen.FLOOR - {r.id for r in rows})

    def test_generator_titles_are_short(self):
        for e in REAL["assumptions"]:
            self.assertLessEqual(len(e["title"]), gen.TITLE_MAX + 3)
        self.assertEqual("short", gen.make_title("short. Then more."))
        self.assertTrue(gen.make_title("word " * 60).endswith("..."))


KINDS_RAW = ac.EVIDENCE_KINDS_PATH.read_bytes()
NON_CLAIMS_RAW = ac.NON_CLAIMS_PATH.read_bytes()


def kinds() -> dict:
    return json.loads(KINDS_RAW)


def non_claims() -> dict:
    return json.loads(NON_CLAIMS_RAW)


def kind_row(data: dict, name: str) -> dict:
    return next(k for k in data["kinds"] if k["kind"] == name)


class CompanionFileTests(unittest.TestCase):
    """evidence-kinds.json, non-claims.json and the report pins (AS-001a part 2)."""

    def test_real_companion_files_and_pins_have_no_findings(self):
        findings = ac.check_companions(RAW, KINDS_RAW, NON_CLAIMS_RAW, ac.PACKAGES_ROOT)
        self.assertEqual([], findings, messages(findings))

    def test_every_catalogue_kind_is_defined_and_every_defined_kind_used(self):
        data = kinds()
        data["kinds"] = [k for k in data["kinds"] if k["kind"] != "miri-run"]
        found = ac.check_evidence_kinds(data, REAL["assumptions"])
        self.assertIn("evidence kind 'miri-run' is not defined", messages(found))
        data = kinds()
        extra = dict(kind_row(data, "miri-run"), kind="orphan", patterns=["orphan/*"])
        data["kinds"].append(extra)
        found = ac.check_evidence_kinds(data, REAL["assumptions"])
        self.assertIn(
            "orphan: evidence kind is named by no catalogue entry", messages(found)
        )

    def test_patterns_are_well_formed_namespaced_and_disjoint(self):
        for bad in ("*/x", "a//b", "a/b c", "/".join(["a"] * 17)):
            self.assertIsNone(ac.parse_pattern(bad), bad)
        self.assertEqual(["warrant", "*", "proof"], ac.parse_pattern("warrant/*/proof"))
        self.assertTrue(ac.patterns_overlap(["a", "*"], ["a", "b"]))
        self.assertFalse(ac.patterns_overlap(["a", "*"], ["a", "*", "c"]))
        data = kinds()
        kind_row(data, "miri-run")["patterns"] = ["warrant/*/proof/*/steps-bound"]
        text = messages(ac.check_evidence_kinds(data, REAL["assumptions"]))
        self.assertIn("outside the 'planned' namespace", text)
        self.assertIn("overlaps a pattern of", text)
        data = kinds()
        kind_row(data, "miri-run")["patterns"] = ["miri//x"]
        self.assertIn(
            "malformed pattern",
            messages(ac.check_evidence_kinds(data, REAL["assumptions"])),
        )

    def test_evidence_kind_field_errors(self):
        data = kinds()
        row = kind_row(data, "miri-run")
        row.update(manifest="ci", verdict="fail", per_run="no", description=" ")
        text = messages(ac.check_evidence_kinds(data, REAL["assumptions"]))
        for needle in ("manifest 'ci'", "verdict must be", "per_run", "description"):
            self.assertIn(needle, text)
        data = kinds()
        data["kinds"].append(dict(data["kinds"][0]))
        self.assertIn(
            "kind defined twice",
            messages(ac.check_evidence_kinds(data, REAL["assumptions"])),
        )
        self.assertTrue(ac.check_evidence_kinds([], REAL["assumptions"]))
        self.assertTrue(
            ac.check_evidence_kinds({"schema": "x", "kinds": []}, REAL["assumptions"])
        )

    def test_non_claim_errors(self):
        data = non_claims()
        data["non_claims"][0].update(
            package="*", cwe=True, claim="speed", source="../etc", statement=""
        )
        text = messages(ac.check_non_claims(data))
        for needle in ("package", "cwe", "claim 'speed'", "source", "statement"):
            self.assertIn(needle, text)
        data = non_claims()
        data["non_claims"][0]["source"] = "code/specs/NO-SUCH.md"
        self.assertIn("does not exist", messages(ac.check_non_claims(data)))
        data = non_claims()
        data["non_claims"].append(dict(data["non_claims"][0]))
        self.assertIn("duplicate non-claim", messages(ac.check_non_claims(data)))
        self.assertTrue(ac.check_non_claims({"schema": "ver13-non-claims-v1"}))

    def test_a_stale_report_pin_is_catalogue_drift(self):
        pin = ac.expected_pin(RAW, KINDS_RAW, NON_CLAIMS_RAW)
        self.assertEqual([], ac.check_report_pins(ac.PACKAGES_ROOT, pin))
        with tempfile.TemporaryDirectory() as tmp:
            pkg = Path(tmp) / "rust" / "demo"
            pkg.mkdir(parents=True)
            report = pkg / "assurance_report.json"
            report.write_text(json.dumps({"catalogue": pin}), encoding="utf-8")
            self.assertEqual([], ac.check_report_pins(Path(tmp), pin))
            stale = dict(pin, sha256="0" * 64)
            report.write_text(json.dumps({"catalogue": stale}), encoding="utf-8")
            found = ac.check_report_pins(Path(tmp), pin)
            self.assertEqual({"AC-14"}, rules(found))
            self.assertIn("CatalogueDrift", messages(found))
            report.write_text("{}", encoding="utf-8")
            self.assertIn(
                "CatalogueDrift", messages(ac.check_report_pins(Path(tmp), pin))
            )
            report.write_text('{"a": 1, "a": 2}', encoding="utf-8")
            self.assertIn("unreadable", messages(ac.check_report_pins(Path(tmp), pin)))

    def test_editing_any_companion_file_changes_the_pin(self):
        pin = ac.expected_pin(RAW, KINDS_RAW, NON_CLAIMS_RAW)
        self.assertNotEqual(
            pin, ac.expected_pin(RAW, KINDS_RAW + b"\n", NON_CLAIMS_RAW)
        )
        self.assertNotEqual(
            pin, ac.expected_pin(RAW, KINDS_RAW, NON_CLAIMS_RAW + b"\n")
        )

    def test_companion_schemas_match_the_checker_and_validate(self):
        schemas = REPO / "code" / "specs" / "schemas"
        ek = json.loads((schemas / "ver13-evidence-kinds-v1.schema.json").read_text())
        nc = json.loads((schemas / "ver13-non-claims-v1.schema.json").read_text())
        item = ek["properties"]["kinds"]["items"]
        self.assertEqual(set(ac.EVIDENCE_KIND_FIELDS), set(item["required"]))
        self.assertEqual(list(ac.MANIFESTS), item["properties"]["manifest"]["enum"])
        row = nc["properties"]["non_claims"]["items"]
        self.assertEqual(set(ac.NON_CLAIM_FIELDS), set(row["required"]))
        self.assertEqual(list(ac.CLAIM_KINDS), row["properties"]["claim"]["enum"])
        try:
            import jsonschema
        except ImportError:  # the checker itself needs no third-party package
            self.skipTest("jsonschema not installed")
        jsonschema.validate(kinds(), ek)
        jsonschema.validate(non_claims(), nc)


def run_cli(*args: str) -> tuple[int, str, str]:
    """Run ``main`` in-process and capture its exit code, stdout and stderr."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = ac.main(list(args))
    return code, out.getvalue(), err.getvalue()


class HardeningTests(unittest.TestCase):
    """Security review of AS-001a part 2: the Python validator agrees with the
    Rust loader on edge cases and never ends in a traceback."""

    def test_trailing_newline_is_rejected_like_rust(self):
        for regex, good in (
            (ac.KIND_RE, "miri-run"),
            (ac.SEGMENT_RE, "warrant"),
            (ac.ID_RE, "memory.unaliased"),
            (ac.ALIAS_RE, "heap-bound:unchecked"),
            (ac.PACKAGE_RE, "rust/gf256"),
            (ac.BACKLOG_ID_RE, "AS-002"),
            (ac.DIGEST_RE, "sha256:" + "0" * 64),
        ):
            self.assertIsNotNone(regex.fullmatch(good), regex.pattern)
            self.assertIsNone(regex.fullmatch(good + "\n"), regex.pattern)
        data = kinds()
        kind_row(data, "miri-run")["kind"] = "miri-run\n"
        text = messages(ac.check_evidence_kinds(data, REAL["assumptions"]))
        self.assertIn("kind must match", text)
        self.assertIsNone(ac.parse_pattern("miri/*/x\n"))
        cat = fresh()
        cat["assumptions"][0]["id"] = cat["assumptions"][0]["id"] + "\n"
        self.assertIn("id must match", messages(ac.validate_structure(cat)))
        nc = non_claims()
        nc["non_claims"][0]["package"] = "rust/gf256\n"
        self.assertIn("package", messages(ac.check_non_claims(nc)))

    def test_unhashable_values_are_findings_not_type_errors(self):
        data = kinds()
        kind_row(data, "miri-run")["manifest"] = ["proof"]
        self.assertIn(
            "manifest must be a string",
            messages(ac.check_evidence_kinds(data, REAL["assumptions"])),
        )
        nc = non_claims()
        nc["non_claims"][0].update(package=["rust/gf256"], cwe={"a": 1}, claim=[1])
        text = messages(ac.check_non_claims(nc))
        self.assertIn("package", text)
        self.assertIn("cwe", text)

    def test_deep_nesting_is_a_catalogue_error_not_a_recursion_error(self):
        deep = "[" * 200_000 + "]" * 200_000
        with self.assertRaises(ac.CatalogueError) as caught:
            ac.parse_catalogue_text(deep)
        self.assertIn("nested too deeply", str(caught.exception))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "kinds.json"
            path.write_text("[" * 100_000 + "]" * 100_000, encoding="utf-8")
            code, _, err = run_cli("--evidence-kinds", str(path))
        self.assertEqual(1, code)
        self.assertIn("ERROR", err)
        self.assertIn("nested too deeply", err)

    def test_depth_limit_is_exact_and_ignores_brackets_in_strings(self):
        ok = "[" * ac.MAX_JSON_DEPTH + "]" * ac.MAX_JSON_DEPTH
        ac.check_json_depth(ok)  # exactly at the limit: accepted
        deep = "[" * (ac.MAX_JSON_DEPTH + 1) + "]" * (ac.MAX_JSON_DEPTH + 1)
        with self.assertRaises(ac.CatalogueError) as caught:
            ac.check_json_depth(deep)
        self.assertIn("nested too deeply", str(caught.exception))
        # Brackets inside strings, including after an escaped quote, are content.
        ac.check_json_depth('{"a": "' + "[" * 500 + '"}')
        ac.check_json_depth('{"a": "x\\"' + "[" * 500 + '"}')
        # The limit applies before json.loads is ever called.
        with self.assertRaises(ac.CatalogueError) as caught:
            ac.parse_catalogue_text('{"a": ' + "[" * 65 + "]" * 65 + "}")
        self.assertIn("nested too deeply", str(caught.exception))

    def test_oversized_files_are_refused_before_parsing(self):
        with tempfile.TemporaryDirectory() as tmp:
            big = Path(tmp) / "non-claims.json"
            big.write_bytes(b" " * (ac.MAX_NON_CLAIMS_BYTES + 1))
            with self.assertRaises(ac.CatalogueError):
                ac.read_capped(big, ac.MAX_NON_CLAIMS_BYTES)
            code, _, err = run_cli("--non-claims", str(big))
            self.assertEqual(1, code)
            self.assertIn("limit", err)
            ok = Path(tmp) / "ok.json"
            ok.write_bytes(b"{}")
            self.assertEqual(b"{}", ac.read_capped(ok, 2))
            pkg = Path(tmp) / "rust" / "demo"
            pkg.mkdir(parents=True)
            report = pkg / "assurance_report.json"
            report.write_bytes(b" " * (ac.MAX_REPORT_BYTES + 1))
            pin = ac.expected_pin(RAW, KINDS_RAW, NON_CLAIMS_RAW)
            self.assertIn("limit", messages(ac.check_report_pins(Path(tmp), pin)))
        self.assertEqual(1024 * 1024, ac.MAX_CATALOGUE_BYTES)
        self.assertEqual(256 * 1024, ac.MAX_EVIDENCE_KINDS_BYTES)

    def test_count_caps(self):
        cat = fresh()
        cat["assumptions"] = cat["assumptions"] * 49  # > 4096 entries
        self.assertIn("more than 4096 entries", messages(ac.validate_structure(cat)))
        data = kinds()
        row = kind_row(data, "miri-run")
        row["patterns"] = [f"m{i}/x" for i in range(ac.MAX_LIST_ITEMS + 1)]
        self.assertIn(
            "more than 256 patterns",
            messages(ac.check_evidence_kinds(data, REAL["assumptions"])),
        )
        data = kinds()
        for n, row in enumerate(data["kinds"][:17]):
            row["patterns"] = [f"ns{n}/p{i}" for i in range(ac.MAX_LIST_ITEMS)]
        self.assertIn(
            "more than 4096 patterns",
            messages(ac.check_evidence_kinds(data, REAL["assumptions"])),
        )
        nc = non_claims()
        nc["non_claims"] = nc["non_claims"] * (ac.MAX_ENTRIES + 1)
        self.assertIn("more than 4096 rows", messages(ac.check_non_claims(nc)))


class CommandLineTests(unittest.TestCase):
    def cli(self, *args: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = ac.main(list(args))
        return code, out.getvalue(), err.getvalue()

    def test_default_run_passes_and_prints_pin(self):
        code, out, err = self.cli()
        self.assertEqual(0, code, err)
        self.assertIn("OK (84 entries)", out)
        self.assertIn(f"catalogue sha256: {ac.catalogue_digest(RAW)}", out)
        self.assertIn(f"evidence-kinds sha256: {ac.catalogue_digest(KINDS_RAW)}", out)

    def test_a_stale_report_pin_fails_the_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            pkg = Path(tmp) / "rust" / "demo"
            pkg.mkdir(parents=True)
            (pkg / "assurance_report.json").write_text("{}", encoding="utf-8")
            code, _, err = self.cli("--packages-root", tmp)
        self.assertEqual(1, code)
        self.assertIn("CatalogueDrift", err)

    def test_digest_only(self):
        code, out, _ = self.cli("--digest")
        self.assertEqual(0, code)
        self.assertEqual(ac.catalogue_digest(RAW) + "\n", out)

    def test_findings_exit_nonzero(self):
        data = fresh()
        entry(data, "memory.unaliased")["depends_on"] = ["ghost"]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "c.json"
            path.write_text(json.dumps(data), encoding="utf-8")
            code, _, err = self.cli("--catalogue", str(path))
        self.assertEqual(1, code)
        self.assertIn("FAIL [AC-14] memory.unaliased", err)
        self.assertIn("finding(s)", err)

    def test_unreadable_or_malformed_input_exits_nonzero(self):
        code, _, err = self.cli("--catalogue", "/nonexistent/catalogue.json")
        self.assertEqual(1, code)
        self.assertIn("ERROR", err)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "c.json"
            path.write_text('{"schema": "a", "schema": "b"}', encoding="utf-8")
            code, _, err = self.cli("--catalogue", str(path))
        self.assertEqual(1, code)
        self.assertIn("duplicate JSON key", err)

    def test_script_runs_as_a_program(self):
        import subprocess

        proc = subprocess.run(
            [
                sys.executable,
                str(REPO / "code" / "scripts" / "assumption_catalogue.py"),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(0, proc.returncode, proc.stderr)
        self.assertTrue(re.search(r"catalogue sha256: [0-9a-f]{64}", proc.stdout))


if __name__ == "__main__":
    unittest.main()
