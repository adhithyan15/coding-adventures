"""Tests for the relocatable-bundle gate."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import verify_relocatable_bundle as gate  # noqa: E402


def _bundle(root: Path, index_html: str, files: dict[str, bytes] | None = None) -> Path:
    dist = root / "dist"
    dist.mkdir(parents=True, exist_ok=True)
    (dist / "index.html").write_text(index_html, encoding="utf-8")
    for name, data in (files or {}).items():
        target = dist / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    return dist


RELATIVE = '<!doctype html><script type="module" src="./assets/app.js"></script>'
ABSOLUTE = '<!doctype html><script type="module" src="/assets/app.js"></script>'


class AssetReferenceTests(unittest.TestCase):
    def test_collects_src_and_href(self) -> None:
        html = '<link href="./a.css"><script src="./b.js"></script>'
        self.assertEqual(gate.asset_references(html), ["./a.css", "./b.js"])

    def test_ignores_offsite_and_inline_references(self) -> None:
        # These say nothing about whether the bundle relocates, and fetching
        # them would make the gate depend on the network.
        html = (
            '<link href="https://fonts.example/x.css">'
            '<link href="//cdn.example/y.css">'
            '<img src="data:image/png;base64,AAAA">'
            '<a href="#main">skip</a>'
            '<script src="./real.js"></script>'
        )
        self.assertEqual(gate.asset_references(html), ["./real.js"])

    def test_deduplicates_while_preserving_order(self) -> None:
        html = '<script src="./a.js"></script><script src="./a.js"></script><link href="./b.css">'
        self.assertEqual(gate.asset_references(html), ["./a.js", "./b.css"])

    def test_root_absolute_detection_excludes_protocol_relative(self) -> None:
        # `//host/x` has a leading slash but points off-site — a different
        # problem, and not one this gate is about.
        html = '<script src="/a.js"></script><link href="//cdn.example/b.css">'
        self.assertEqual(gate.root_absolute_references(html), ["/a.js"])


class VerifyTests(unittest.TestCase):
    def test_rejects_a_root_absolute_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(root, ABSOLUTE, {"assets/app.js": b"export {}\n"})
            problems = gate.verify(dist)
            self.assertTrue(problems)
            self.assertIn("/assets/app.js", problems[0])
            self.assertIn("root of a domain", problems[0])

    def test_accepts_a_relative_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(root, RELATIVE, {"assets/app.js": b"export {}\n"})
            self.assertEqual(gate.verify(dist), [])

    def test_rejects_a_relative_reference_whose_file_is_missing(self) -> None:
        # The load-bearing case for the *serving* half. The reference is
        # correctly relative, so the regex pre-check passes it — only actually
        # fetching it over HTTP reveals that nothing is there. Without this
        # test, the serve loop could be dead code and the suite would not say so.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(root, RELATIVE)  # no assets/app.js written
            problems = gate.verify(dist)
            self.assertEqual(len(problems), 1)
            self.assertIn("./assets/app.js", problems[0])
            self.assertIn("404", problems[0])

    def test_reports_the_nested_path_it_actually_requested(self) -> None:
        # The message must name the path that was fetched. An earlier draft
        # served the bundle's parent and *described* the result as nested, so
        # the reported path and the requested path disagreed — a report that
        # misstates what it verified.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(root, RELATIVE)
            problems = gate.verify(dist)
            self.assertIn(gate.MOUNT_PREFIX, problems[0])

    def test_checks_extra_paths_not_referenced_from_the_entry_point(self) -> None:
        # A wasm engine is fetched by script, so it never appears in
        # index.html. It is exactly the asset whose absence is invisible until
        # a user tries to import a deck.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(root, RELATIVE, {"assets/app.js": b"export {}\n"})
            problems = gate.verify(dist, extra_paths=("engine.wasm",))
            self.assertEqual(len(problems), 1)
            self.assertIn("engine.wasm", problems[0])

            (dist / "engine.wasm").write_bytes(b"\x00asm\x01\x00\x00\x00")
            self.assertEqual(gate.verify(dist, extra_paths=("engine.wasm",)), [])

    def test_reports_a_missing_entry_point(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            dist = Path(tmp) / "dist"
            dist.mkdir()
            problems = gate.verify(dist)
            self.assertEqual(len(problems), 1)
            self.assertIn("index.html", problems[0])

    def test_reports_every_broken_reference_at_once(self) -> None:
        # One run should surface the whole list rather than the first failure,
        # so a fix cycle is not one round trip per broken asset.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(
                root,
                '<!doctype html><link href="./a.css"><script src="./b.js"></script>',
            )
            problems = gate.verify(dist)
            self.assertEqual(len(problems), 2)


class RootAbsoluteAssetLiteralTests(unittest.TestCase):
    """The check for assets that only *script* fetches.

    This is the half that catches Engram's `WASM_URL = "/engram_engine.wasm"`.
    Neither the index.html scan nor serving the bundle sees it: the wasm never
    appears in the entry point, and the file *is* present at the bundle-relative
    path, so a fetch of it succeeds while the app asks for the wrong URL.
    """

    def test_flags_a_bundle_file_referenced_from_the_domain_root(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(
                root,
                RELATIVE,
                {
                    "assets/app.js": b'const W="/engine.wasm";export{W}\n',
                    "engine.wasm": b"\x00asm\x01\x00\x00\x00",
                },
            )
            self.assertEqual(gate.root_absolute_asset_literals(dist), ["engine.wasm"])

            problems = gate.verify(dist, extra_paths=("engine.wasm",))
            self.assertEqual(len(problems), 1)
            self.assertIn("/engine.wasm", problems[0])

    def test_accepts_the_relative_form(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(
                root,
                RELATIVE,
                {
                    "assets/app.js": b'const W="./engine.wasm";export{W}\n',
                    "engine.wasm": b"\x00asm\x01\x00\x00\x00",
                },
            )
            self.assertEqual(gate.root_absolute_asset_literals(dist), [])
            self.assertEqual(gate.verify(dist, extra_paths=("engine.wasm",)), [])

    def test_does_not_flag_leading_slashes_that_are_not_bundle_files(self) -> None:
        # Minified JS is full of leading slashes — regexes, dates, division.
        # Only a literal naming a file that actually sits in the bundle counts,
        # which is what keeps this precise instead of noisy.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = _bundle(
                root,
                RELATIVE,
                {
                    "assets/app.js": (
                        b'const re=/\\d+/g,d="/api/v1/users",r=a/b;export{re,d,r}\n'
                    ),
                    "engine.wasm": b"\x00asm\x01\x00\x00\x00",
                },
            )
            self.assertEqual(gate.root_absolute_asset_literals(dist), [])


class CommandLineTests(unittest.TestCase):
    def test_exit_code_reflects_the_verdict(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            good = _bundle(root, RELATIVE, {"assets/app.js": b"export {}\n"})
            self.assertEqual(gate.main([str(good)]), 0)

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bad = _bundle(root, ABSOLUTE, {"assets/app.js": b"export {}\n"})
            self.assertEqual(gate.main([str(bad)]), 1)


if __name__ == "__main__":
    unittest.main()
