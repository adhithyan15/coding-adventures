"""Minimal JAR decoder package."""

from __future__ import annotations

from jvm_jar_decoder.jar_decoder import JarArchive, JarEntry, decode_jar

__all__ = ["JarArchive", "JarEntry", "decode_jar"]
