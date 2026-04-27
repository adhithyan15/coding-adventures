from __future__ import annotations

import io
import zipfile

from jvm_jar_decoder import decode_jar


def test_decode_jar_splits_classes_resources_and_manifest() -> None:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        archive.writestr("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\n")
        archive.writestr("Example.class", b"\xCA\xFE\xBA\xBE")
        archive.writestr("config/app.properties", b"mode=test")

    jar = decode_jar(buffer.getvalue())

    assert jar.manifest == "Manifest-Version: 1.0\n"
    assert [entry.path for entry in jar.class_entries] == ["Example.class"]
    assert [entry.path for entry in jar.resource_entries] == ["config/app.properties"]
