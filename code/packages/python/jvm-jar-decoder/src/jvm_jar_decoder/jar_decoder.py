"""JAR container decoding primitives for the JVM toolchain."""

from __future__ import annotations

import io
import zipfile
from dataclasses import dataclass


@dataclass(frozen=True)
class JarEntry:
    path: str
    data: bytes


@dataclass(frozen=True)
class JarArchive:
    manifest: str | None
    class_entries: tuple[JarEntry, ...]
    resource_entries: tuple[JarEntry, ...]


def decode_jar(data: bytes) -> JarArchive:
    class_entries: list[JarEntry] = []
    resource_entries: list[JarEntry] = []
    manifest: str | None = None

    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        for path in archive.namelist():
            payload = archive.read(path)
            if path == "META-INF/MANIFEST.MF":
                manifest = payload.decode("utf-8")
            elif path.endswith(".class"):
                class_entries.append(JarEntry(path=path, data=payload))
            else:
                resource_entries.append(JarEntry(path=path, data=payload))

    return JarArchive(
        manifest=manifest,
        class_entries=tuple(class_entries),
        resource_entries=tuple(resource_entries),
    )
