"""Shared fixtures for code-packager tests."""

from __future__ import annotations

import pytest

from code_packager import CodeArtifact, Target


@pytest.fixture
def nop_x86() -> bytes:
    """One NOP byte: the smallest valid x86-64 instruction."""
    return b"\x90"


@pytest.fixture
def small_code() -> bytes:
    """xor rax,rax; ret — a minimal 4-byte x86-64 function returning 0."""
    return b"\x48\x31\xc0\xc3"


@pytest.fixture
def linux_artifact(small_code: bytes) -> CodeArtifact:
    return CodeArtifact(
        native_bytes=small_code,
        entry_point=0,
        target=Target.linux_x64(),
        symbol_table={"main": 0},
    )


@pytest.fixture
def windows_artifact(small_code: bytes) -> CodeArtifact:
    return CodeArtifact(
        native_bytes=small_code,
        entry_point=0,
        target=Target.windows_x64(),
    )


@pytest.fixture
def macos_arm64_artifact(small_code: bytes) -> CodeArtifact:
    return CodeArtifact(
        native_bytes=small_code,
        entry_point=0,
        target=Target.macos_arm64(),
    )


@pytest.fixture
def macos_x64_artifact(small_code: bytes) -> CodeArtifact:
    return CodeArtifact(
        native_bytes=small_code,
        entry_point=0,
        target=Target.macos_x64(),
    )


@pytest.fixture
def raw_artifact() -> CodeArtifact:
    return CodeArtifact(
        native_bytes=b"\xd7\x01\xc0",  # Some 8-bit ISA bytes
        entry_point=0,
        target=Target.raw(arch="i4004"),
    )


@pytest.fixture
def hex_artifact() -> CodeArtifact:
    return CodeArtifact(
        native_bytes=b"\xd7\x01\xc0",
        entry_point=0,
        target=Target.intel_4004(),
    )


@pytest.fixture
def wasm_artifact() -> CodeArtifact:
    # Minimal WASM function body: i32.const 42, end (0x41 0x2a 0x0b)
    return CodeArtifact(
        native_bytes=b"\x41\x2a\x0b",
        entry_point=0,
        target=Target.wasm(),
    )
