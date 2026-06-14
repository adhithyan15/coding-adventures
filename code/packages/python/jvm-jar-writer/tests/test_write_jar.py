"""Tests for ``write_jar`` — the actual JAR-byte producer.

Round-trip via Python's stdlib ``zipfile`` is the primary
correctness check (it's the same parser ``java -jar`` ultimately
uses).  Plus determinism / validation tests.
"""

from __future__ import annotations

import io
import zipfile

import pytest

from jvm_jar_writer import JarManifest, JarWriterError, write_jar


# A trivial 8-byte payload that stands in for ``.class`` bytes.
# Real class bytes start with the JVM magic ``0xCAFEBABE``; the
# JAR writer doesn't care.
_FAKE_CLASS = b"\xca\xfe\xba\xbe\x00\x00\x00\x34"


class TestRoundTrip:
    def test_zipfile_can_open_emitted_jar(self) -> None:
        jar = write_jar(
            classes=(("Foo.class", _FAKE_CLASS),),
            manifest=JarManifest(main_class="Foo"),
        )
        with zipfile.ZipFile(io.BytesIO(jar)) as zf:
            names = zf.namelist()
            assert names[0] == "META-INF/MANIFEST.MF"
            assert "Foo.class" in names

    def test_class_bytes_round_trip_unchanged(self) -> None:
        jar = write_jar(
            classes=(("a/b/C.class", _FAKE_CLASS),),
            manifest=JarManifest(),
        )
        with zipfile.ZipFile(io.BytesIO(jar)) as zf:
            assert zf.read("a/b/C.class") == _FAKE_CLASS

    def test_manifest_can_be_read_back(self) -> None:
        jar = write_jar(
            classes=(),
            manifest=JarManifest(
                main_class="com.example.Main",
                extra_attributes={"Built-By": "test"},
            ),
        )
        with zipfile.ZipFile(io.BytesIO(jar)) as zf:
            mf = zf.read("META-INF/MANIFEST.MF").decode("utf-8")
        assert "Manifest-Version: 1.0" in mf
        assert "Main-Class: com.example.Main" in mf
        assert "Built-By: test" in mf

    def test_multiple_classes_all_present(self) -> None:
        jar = write_jar(
            classes=(
                ("a/A.class", _FAKE_CLASS),
                ("b/B.class", _FAKE_CLASS),
                ("c/C.class", _FAKE_CLASS),
            ),
            manifest=JarManifest(),
        )
        with zipfile.ZipFile(io.BytesIO(jar)) as zf:
            names = set(zf.namelist())
        assert {"META-INF/MANIFEST.MF", "a/A.class", "b/B.class", "c/C.class"} <= names


class TestDeterminism:
    def test_byte_identical_outputs_for_identical_inputs(self) -> None:
        """The whole point of the deterministic ZIP-timestamp
        choice — feed the same inputs twice, get the same bytes."""
        args = {
            "classes": (("Foo.class", _FAKE_CLASS),),
            "manifest": JarManifest(main_class="Foo"),
        }
        first = write_jar(**args)
        second = write_jar(**args)
        assert first == second


class TestRejectedInputs:
    def test_meta_inf_path_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="reserved prefix"):
            write_jar(
                classes=(("META-INF/services/foo", b""),),
                manifest=JarManifest(),
            )

    def test_empty_path_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="must not be empty"):
            write_jar(classes=(("", _FAKE_CLASS),), manifest=JarManifest())

    def test_absolute_path_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="must not start with"):
            write_jar(
                classes=(("/Foo.class", _FAKE_CLASS),),
                manifest=JarManifest(),
            )

    def test_backslash_path_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="forward slashes"):
            write_jar(
                classes=((r"a\b\C.class", _FAKE_CLASS),),
                manifest=JarManifest(),
            )

    def test_duplicate_path_rejected(self) -> None:
        with pytest.raises(JarWriterError, match="duplicate"):
            write_jar(
                classes=(
                    ("Foo.class", _FAKE_CLASS),
                    ("Foo.class", _FAKE_CLASS),
                ),
                manifest=JarManifest(),
            )
