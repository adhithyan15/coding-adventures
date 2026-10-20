"""Regression tests for authenticated snapshot source extraction."""

import hashlib
import importlib.util
import io
import pathlib
import subprocess
import sys
import tarfile
import tempfile
import unittest

PACKAGE = pathlib.Path(__file__).resolve().parent.parent
GENERATOR_PATH = PACKAGE / "tools" / "generate_snapshot.py"
SPEC = importlib.util.spec_from_file_location("generate_snapshot", GENERATOR_PATH)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)

SOURCE = b"""\
pub static TLS_SERVER_ROOTS: &[TrustAnchor<'static>] = &[\n
    TrustAnchor {\n
        subject: Der::from_slice(b"\\x30"),\n
        subject_public_key_info: Der::from_slice(b"\\x31"),\n
        name_constraints: None\n
    }\n
];\n
"""


def write_archive(path: pathlib.Path, entries: list[tuple[str, bytes, str]]) -> None:
    with tarfile.open(path, mode="w:gz") as archive:
        for name, contents, kind in entries:
            member = tarfile.TarInfo(name)
            if kind == "file":
                member.size = len(contents)
                archive.addfile(member, io.BytesIO(contents))
            elif kind == "symlink":
                member.type = tarfile.SYMTYPE
                member.linkname = "elsewhere"
                archive.addfile(member)
            else:
                raise AssertionError(f"unsupported fixture kind {kind}")


class AuthenticatedSourceTests(unittest.TestCase):
    def test_reads_the_exact_regular_source_member(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive = pathlib.Path(directory) / "roots.crate"
            write_archive(archive, [(GENERATOR.SOURCE_MEMBER, SOURCE, "file")])
            self.assertEqual(GENERATOR.read_authenticated_source(archive), SOURCE.decode())

    def test_rejects_unexpected_path_link_and_duplicate(self) -> None:
        fixtures = (
            [("webpki-roots-1.0.9/src/other.rs", SOURCE, "file")],
            [(GENERATOR.SOURCE_MEMBER, b"", "symlink")],
            [
                (GENERATOR.SOURCE_MEMBER, SOURCE, "file"),
                (GENERATOR.SOURCE_MEMBER, SOURCE, "file"),
            ],
        )
        for entries in fixtures:
            with self.subTest(entries=entries), tempfile.TemporaryDirectory() as directory:
                archive = pathlib.Path(directory) / "roots.crate"
                write_archive(archive, entries)
                with self.assertRaises(ValueError):
                    GENERATOR.read_authenticated_source(archive)

    def test_rejects_separate_source_injection_and_mutated_archive(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            archive = root / "roots.crate"
            output = root / "roots.bin"
            source = root / "attacker.rs"
            write_archive(archive, [(GENERATOR.SOURCE_MEMBER, SOURCE, "file")])
            reviewed_digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            source.write_bytes(SOURCE.replace(b"\\x31", b"\\x32"))
            injected = subprocess.run(
                [
                    sys.executable,
                    str(GENERATOR_PATH),
                    "--archive",
                    str(archive),
                    "--archive-sha256",
                    reviewed_digest,
                    "--source",
                    str(source),
                    "--output",
                    str(output),
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(injected.returncode, 0)
            self.assertIn("unrecognized arguments: --source", injected.stderr)

            archive.write_bytes(archive.read_bytes() + b"mutated")
            mutated = subprocess.run(
                [
                    sys.executable,
                    str(GENERATOR_PATH),
                    "--archive",
                    str(archive),
                    "--archive-sha256",
                    reviewed_digest,
                    "--output",
                    str(output),
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(mutated.returncode, 0)
            self.assertIn("archive SHA-256 mismatch", mutated.stderr)


if __name__ == "__main__":
    unittest.main()
