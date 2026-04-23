"""Tests for code_packager.errors exception hierarchy."""

from __future__ import annotations

import pytest

from code_packager import (
    ArtifactTooLargeError,
    MissingMetadataError,
    PackagerError,
    Target,
    UnsupportedTargetError,
)


class TestPackagerError:
    def test_is_exception(self):
        assert issubclass(PackagerError, Exception)

    def test_unsupported_is_packager_error(self):
        assert issubclass(UnsupportedTargetError, PackagerError)

    def test_too_large_is_packager_error(self):
        assert issubclass(ArtifactTooLargeError, PackagerError)

    def test_missing_metadata_is_packager_error(self):
        assert issubclass(MissingMetadataError, PackagerError)


class TestUnsupportedTargetError:
    def test_message(self):
        t = Target.linux_x64()
        err = UnsupportedTargetError(t)
        assert "x86_64" in str(err)
        assert "linux" in str(err)
        assert "elf64" in str(err)

    def test_target_attribute(self):
        t = Target.windows_x64()
        err = UnsupportedTargetError(t)
        assert err.target is t

    def test_raiseable(self):
        with pytest.raises(UnsupportedTargetError):
            raise UnsupportedTargetError(Target.wasm())


class TestArtifactTooLargeError:
    def test_message(self):
        err = ArtifactTooLargeError(artifact_size=5_000_000_000, limit=4_294_967_295)
        assert "5000000000" in str(err)
        assert "4294967295" in str(err)

    def test_attributes(self):
        err = ArtifactTooLargeError(artifact_size=100, limit=50)
        assert err.artifact_size == 100
        assert err.limit == 50


class TestMissingMetadataError:
    def test_message(self):
        err = MissingMetadataError("stack_size")
        assert "stack_size" in str(err)

    def test_key_attribute(self):
        err = MissingMetadataError("load_address")
        assert err.key == "load_address"
