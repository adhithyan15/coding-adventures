"""Exception hierarchy for code-packager.

All exceptions carry context attributes so callers can distinguish failures
programmatically without parsing the error message string.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from code_packager.target import Target


class PackagerError(Exception):
    """Base class for all code-packager failures."""


class UnsupportedTargetError(PackagerError):
    """No registered packager supports the requested target.

    Attributes
    ----------
    target:
        The :class:`~code_packager.target.Target` that was not found.
    """

    def __init__(self, target: "Target") -> None:
        self.target = target
        super().__init__(
            f"No packager found for target {target} "
            f"(arch={target.arch!r}, os={target.os!r}, "
            f"binary_format={target.binary_format!r})"
        )


class ArtifactTooLargeError(PackagerError):
    """The native-code blob exceeds the binary format's size limit.

    Attributes
    ----------
    artifact_size:
        Size in bytes of the offending artifact.
    limit:
        Maximum size the format supports.
    """

    def __init__(self, artifact_size: int, limit: int) -> None:
        self.artifact_size = artifact_size
        self.limit = limit
        super().__init__(
            f"Artifact is {artifact_size} bytes; format limit is {limit} bytes"
        )


class MissingMetadataError(PackagerError):
    """A required metadata key is absent from the artifact.

    Attributes
    ----------
    key:
        The metadata key that was expected but not found.
    """

    def __init__(self, key: str) -> None:
        self.key = key
        super().__init__(f"Required metadata key {key!r} is missing from artifact")
