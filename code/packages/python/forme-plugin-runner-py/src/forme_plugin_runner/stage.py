"""Authored stage contract and local cancellation primitives."""

from __future__ import annotations

from collections.abc import Awaitable, Callable, Sequence
from dataclasses import dataclass, field
from typing import Any, TypeVar

RunFunction = Callable[[Any, Any, Any], Awaitable[Any]]
InitFunction = Callable[[Any, Any], Awaitable[None]]
DisposeFunction = Callable[[Any], Awaitable[None]]
T = TypeVar("T", bound=RunFunction)


class StageError(Exception):
    """A typed failure safe to expose through the plugin boundary."""

    def __init__(
        self,
        code: str,
        message: str,
        *,
        input_path: str | None = None,
        input_id: str | None = None,
        stage_name: str | None = None,
        recoverable: bool = False,
        fields: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.input_path = input_path
        self.input_id = input_id
        self.stage_name = stage_name
        self.recoverable = recoverable
        self.fields = fields or {}


class CancellationError(Exception):
    """Raised when the host cancels the active invocation."""


class CapabilityError(StageError):
    """Typed host capability denial."""

    def __init__(self, message: str, capability: str) -> None:
        super().__init__("CAPABILITY_DENIED", message, fields={"capability": capability})
        self.capability = capability


class CancellationToken:
    """Local synchronous cancellation observation for an async stage."""

    def __init__(self) -> None:
        self._cancelled = False
        self._reason = "operation cancelled"

    @property
    def is_cancelled(self) -> bool:
        return self._cancelled

    def cancel(self, reason: str | None = None) -> None:
        self._cancelled = True
        if reason:
            self._reason = reason

    def throw_if_cancelled(self) -> None:
        if self._cancelled:
            raise CancellationError(self._reason)


@dataclass(frozen=True)
class Stage:
    """Immutable authored contract consumed by :func:`run_plugin`."""

    name: str
    version: str
    api_version: int
    description: str
    consumes: str
    produces: str
    capabilities: tuple[str, ...]
    run: RunFunction = field(compare=False, repr=False)
    init: InitFunction | None = field(default=None, compare=False, repr=False)
    dispose: DisposeFunction | None = field(default=None, compare=False, repr=False)


def define_stage(
    *,
    name: str,
    version: str,
    api_version: int,
    consumes: str,
    produces: str,
    capabilities: Sequence[str],
    description: str = "Forme Python plugin stage",
    init: InitFunction | None = None,
    dispose: DisposeFunction | None = None,
) -> Callable[[T], Stage]:
    """Decorate an async function with its immutable FM01 stage metadata."""

    def decorate(function: T) -> Stage:
        return Stage(
            name=name,
            version=version,
            api_version=api_version,
            description=description,
            consumes=consumes,
            produces=produces,
            capabilities=tuple(capabilities),
            run=function,
            init=init,
            dispose=dispose,
        )

    return decorate
