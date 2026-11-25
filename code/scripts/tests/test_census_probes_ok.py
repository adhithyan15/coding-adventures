"""Tests for code/programs/go/kiln/ci/census_probes_ok.py (the in-sandbox toolchain probe gate)."""

import contextlib
import importlib.util
import io
import json
import pathlib
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parents[2] / "programs/go/kiln/ci/census_probes_ok.py"
spec = importlib.util.spec_from_file_location("census_probes_ok", SCRIPT)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


def probes(**over):
    status = {t: "ok" for t in mod.REQUIRED + mod.COMPANIONS}
    status.update(over)
    # Sorted by tool and without a detail, as recfmt's EncodeProbes writes it.
    return {"toolchains": [{"tool": t, "status": s} for t, s in sorted(status.items())]}


class ProbesOkTests(unittest.TestCase):
    def run_main(self, content):
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d, "probes.json")
            if content is not None:
                p.write_text(content if isinstance(content, str) else json.dumps(content))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                rc = mod.main(["x", str(p), "3"])
            return rc, out.getvalue()

    def test_all_ok_passes(self):
        rc, out = self.run_main(probes())
        self.assertEqual(rc, 0)
        self.assertIn("every installed toolchain probed OK", out)

    def test_each_required_tool_must_be_ok(self):
        for tool in mod.REQUIRED:
            for status in ("absent", "refused", "mismatch", "timeout"):
                rc, out = self.run_main(probes(**{tool: status}))
                self.assertEqual(rc, 1, (tool, status))
                self.assertIn(f"{tool}: {status}", out)
                self.assertIn("not the PR8 gate", out)

    def test_a_missing_required_tool_fails(self):
        doc = probes()
        doc["toolchains"] = [e for e in doc["toolchains"] if e["tool"] != "ghc"]
        rc, out = self.run_main(doc)
        self.assertEqual(rc, 1)
        self.assertIn("ghc: not probed", out)

    def test_a_companion_mismatch_is_reported_not_fatal(self):
        rc, out = self.run_main(probes(uv="mismatch"))
        self.assertEqual(rc, 0)
        self.assertIn("uv: mismatch", out)

    def test_a_missing_file_fails(self):
        rc, out = self.run_main(None)
        self.assertEqual(rc, 1)
        self.assertIn("unreadable (FileNotFoundError)", out)

    def test_strict_parsing_rejects_malformed_files(self):
        bad = [
            "{not json",
            "[]",
            "{}",
            '{"toolchains": {}}',
            '{"toolchains": [1]}',
            '{"toolchains": [{"tool": "go"}]}',
            '{"toolchains": [{"tool": "go", "status": "ok", "extra": 1}]}',
            '{"toolchains": [{"tool": "go", "status": "fine"}]}',
            '{"toolchains": [{"tool": "", "status": "ok"}]}',
            '{"toolchains": [{"tool": "go", "status": "ok"}, {"tool": "go", "status": "ok"}]}',
            "[" * 200000,
        ]
        for text in bad:
            rc, out = self.run_main(text)
            self.assertEqual(rc, 1, text[:40])
            self.assertIn("not OK", out)

    def test_a_mismatch_detail_is_accepted(self):
        doc = probes(go="mismatch")
        for e in doc["toolchains"]:
            if e["tool"] == "go":
                e["detail"] = "go version go1.25"
        rc, out = self.run_main(doc)
        self.assertEqual(rc, 1)
        self.assertIn("go: mismatch", out)


if __name__ == "__main__":
    unittest.main()
