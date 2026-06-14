"""Pure JAR (ZIP + ``META-INF/MANIFEST.MF``) writer.

A JAR is just a ZIP archive whose first entry is
``META-INF/MANIFEST.MF`` describing the contents.  We use Python's
stdlib ``zipfile`` to produce the ZIP container; the manifest
itself is plain text with strict 72-byte line wrapping per the
JAR specification (which traces back to RFC 822 mail headers).

Why we don't shell out to the ``jar`` tool
==========================================

Two reasons.  First, the educational repo is Python-only — every
package should run without external native toolchains.  Second,
``zipfile`` produces byte-deterministic output when fed a fixed
``date_time`` for every entry, which lets tests assert on JAR-byte
equality.  Real ``jar`` uses the current time and is not
deterministic.

Manifest format primer
======================

A manifest is a UTF-8 text file with sections.  Each section is a
list of ``Name: Value`` lines terminated by an empty line.  Lines
must be at most 72 bytes including the trailing CRLF; longer lines
fold by inserting ``CRLF<space>`` (a single SP after the line
break) and continuing.  The first section (the *main attributes*)
typically carries:

  Manifest-Version: 1.0
  Main-Class: com.example.Main

We always emit ``Manifest-Version: 1.0`` automatically; callers
add ``Main-Class`` (and any other attributes) via ``JarManifest``.

ECMA-style ZIP determinism
==========================

Every JAR entry uses ``date_time = (1980, 1, 1, 0, 0, 0)`` —
the lowest value the ZIP timestamp format can express.  Combined
with our consistent compression choice (``ZIP_DEFLATED`` for
``.class`` entries, ``ZIP_STORED`` for the manifest) this means
identical inputs produce byte-identical outputs.
"""

from __future__ import annotations

import io
import re
import zipfile
from dataclasses import dataclass, field
from typing import Final


class JarWriterError(ValueError):
    """Raised when a JAR cannot be written.

    Covers reserved-path entries, duplicate entries, malformed
    manifest attribute names, and similar caller mistakes.
    """


# Manifest attribute names follow the format from the JAR spec
# (which delegates to RFC 822): one or more ``A-Za-z0-9_-`` chars
# starting with a letter, max 70 bytes total (so the longest legal
# attribute name + ``": "`` + at least one value byte still fits in
# 72 bytes).  We use the strict subset that covers everything Twig
# / educational use cases ever need.
_VALID_ATTR_NAME: Final = re.compile(r"^[A-Za-z][A-Za-z0-9_-]{0,69}$")

# Reserved JAR-layout prefixes.  Anything starting with these is
# meta and not a user class.  We block writes there to avoid
# accidentally overwriting our own auto-generated manifest.
_RESERVED_PREFIXES: Final = ("META-INF/",)

# Deterministic ZIP timestamp.  1980-01-01 00:00:00 is the
# zero-point of the ZIP format's 16-bit MS-DOS time encoding.
_ZIP_EPOCH: Final = (1980, 1, 1, 0, 0, 0)

# Maximum line length per the JAR spec.  Includes the trailing
# CRLF, so payload per line is 70 bytes max for the first line
# and 71 bytes for continuation lines (which start with one SP).
_MAX_LINE_BYTES: Final = 72


@dataclass(frozen=True)
class JarManifest:
    """Manifest content for the JAR's ``META-INF/MANIFEST.MF``.

    ``main_class`` is the optional ``Main-Class`` attribute that
    makes the JAR executable via ``java -jar``.  Any further
    attributes go in ``extra_attributes`` as a name → value dict.

    The class auto-injects ``Manifest-Version: 1.0`` — callers
    don't need to (and shouldn't) include it.
    """

    main_class: str | None = None
    extra_attributes: dict[str, str] = field(default_factory=dict)


# ---------------------------------------------------------------------------
# Manifest encoding
# ---------------------------------------------------------------------------


def _encode_attribute(name: str, value: str) -> bytes:
    """Encode one ``Name: Value`` attribute, handling line folding.

    The JAR spec says lines are at most 72 bytes including CRLF.
    For the first line that's ``Name: `` (= len(name) + 2 bytes)
    plus value bytes plus CRLF, so the value's first chunk gets
    ``70 - len(name) - 2`` = ``68 - len(name)`` bytes.

    Continuation lines start with one SP, so each subsequent chunk
    gets ``70 - 1`` = ``69`` bytes (then CRLF closes to 72).
    """
    if not _VALID_ATTR_NAME.match(name):
        msg = (
            f"manifest attribute name {name!r} doesn't match "
            f"{_VALID_ATTR_NAME.pattern!r} — JAR / RFC 822 attribute "
            "names must start with a letter and contain only "
            "alphanumerics, dashes, and underscores"
        )
        raise JarWriterError(msg)
    if "\r" in value or "\n" in value:
        msg = (
            f"manifest attribute {name!r} value contains a line "
            "break; embed only single-line values"
        )
        raise JarWriterError(msg)

    encoded_value = value.encode("utf-8")
    out = bytearray()
    # First line: ``Name: <chunk>\r\n``.  Available bytes for the
    # first chunk = MAX_LINE_BYTES (72) - len("Name: ") - 2 (CRLF).
    header = f"{name}: ".encode("utf-8")
    first_room = _MAX_LINE_BYTES - len(header) - 2
    if first_room < 1:
        msg = (
            f"manifest attribute name {name!r} is too long — "
            "would leave no room for the value on the first line"
        )
        raise JarWriterError(msg)
    out.extend(header)
    chunk, encoded_value = encoded_value[:first_room], encoded_value[first_room:]
    out.extend(chunk)
    out.extend(b"\r\n")

    # Continuation lines: each starts with one SP, then ``MAX -
    # 1 - 2`` = 69 bytes of payload, then CRLF.
    cont_room = _MAX_LINE_BYTES - 1 - 2
    while encoded_value:
        chunk, encoded_value = (
            encoded_value[:cont_room],
            encoded_value[cont_room:],
        )
        out.extend(b" ")
        out.extend(chunk)
        out.extend(b"\r\n")
    return bytes(out)


def encode_manifest(manifest: JarManifest) -> bytes:
    """Encode a ``JarManifest`` as the bytes of
    ``META-INF/MANIFEST.MF``.

    Always starts with ``Manifest-Version: 1.0``.  ``Main-Class``
    follows when set.  Then any extra attributes in insertion
    order.  Closes with an empty CRLF (the section terminator).
    """
    out = bytearray()
    out.extend(_encode_attribute("Manifest-Version", "1.0"))
    if manifest.main_class is not None:
        out.extend(_encode_attribute("Main-Class", manifest.main_class))
    for key, value in manifest.extra_attributes.items():
        out.extend(_encode_attribute(key, value))
    # Sections terminate with an empty CRLF.
    out.extend(b"\r\n")
    return bytes(out)


# ---------------------------------------------------------------------------
# JAR assembly
# ---------------------------------------------------------------------------


def _validate_class_path(path: str) -> None:
    if not path:
        raise JarWriterError("class path must not be empty")
    if path.startswith("/"):
        raise JarWriterError(
            f"class path {path!r} must not start with '/' — JAR paths "
            "are relative to the archive root"
        )
    if "\\" in path:
        raise JarWriterError(
            f"class path {path!r} must use forward slashes (POSIX-style)"
        )
    for prefix in _RESERVED_PREFIXES:
        if path.startswith(prefix):
            raise JarWriterError(
                f"class path {path!r} starts with reserved prefix "
                f"{prefix!r} — JAR layout reserves that for the manifest "
                "and similar archive metadata"
            )


def write_jar(
    classes: tuple[tuple[str, bytes], ...],
    manifest: JarManifest,
) -> bytes:
    """Produce a JAR archive from a list of class entries + manifest.

    Parameters
    ----------
    classes
        Tuple of ``(path_within_jar, class_bytes)`` pairs.  Paths
        use forward slashes and are relative to the archive root
        (e.g. ``"com/example/Main.class"``).  No two paths may be
        equal.
    manifest
        Manifest content; ``Manifest-Version: 1.0`` is auto-added.

    Returns
    -------
    bytes
        The complete JAR archive bytes.  Suitable for writing to
        disk and invoking via ``java -jar <path>``.
    """
    # Validate class entries up-front so a bad input doesn't half-
    # write a JAR.
    seen_paths: set[str] = set()
    for path, _ in classes:
        _validate_class_path(path)
        if path in seen_paths:
            raise JarWriterError(f"duplicate class path: {path!r}")
        seen_paths.add(path)

    manifest_bytes = encode_manifest(manifest)

    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as zf:
        # Manifest first — JAR convention.  Stored (no compression)
        # because the manifest is small and easily inspected.
        info = zipfile.ZipInfo("META-INF/MANIFEST.MF", date_time=_ZIP_EPOCH)
        info.compress_type = zipfile.ZIP_STORED
        zf.writestr(info, manifest_bytes)

        # Class entries follow, deflated.
        for path, payload in classes:
            info = zipfile.ZipInfo(path, date_time=_ZIP_EPOCH)
            info.compress_type = zipfile.ZIP_DEFLATED
            zf.writestr(info, payload)

    return buffer.getvalue()
