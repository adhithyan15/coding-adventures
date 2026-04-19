"""
window_native -- Native window bridge for Python.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from enum import Enum, IntEnum

from window_native.window_native import (  # type: ignore[import]
    WindowError,
    _close_window,
    _create_window,
    _window_id,
    _window_logical_size,
    _window_physical_size,
    _window_render_target_kind,
    _window_request_redraw,
    _window_scale_factor,
    _window_set_title,
    _window_set_visible,
)


class SurfacePreference(IntEnum):
    DEFAULT = 0
    METAL = 1
    DIRECT2D = 2
    CAIRO = 3
    CANVAS2D = 4


class RenderTargetKind(str, Enum):
    NONE = "none"
    APPKIT = "appkit"
    WIN32 = "win32"
    BROWSER_CANVAS = "browser-canvas"
    WAYLAND = "wayland"
    X11 = "x11"


@dataclass(frozen=True)
class LogicalSize:
    width: float
    height: float


@dataclass(frozen=True)
class PhysicalSize:
    width: int
    height: int


def _normalize_dimension(name: str, value: float) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError) as exc:
        raise TypeError(f"{name} must be numeric") from exc

    if not math.isfinite(number) or number < 0.0:
        raise WindowError(f"{name} must be a finite, non-negative number")

    return number


class Window:
    def __init__(self, handle: int) -> None:
        self._handle = handle

    def __enter__(self) -> "Window":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass

    def _require_open(self) -> int:
        if self._handle == 0:
            raise WindowError("window handle is closed")
        return self._handle

    def close(self) -> None:
        if self._handle != 0:
            _close_window(self._handle)
            self._handle = 0

    def id(self) -> int:
        return _window_id(self._require_open())

    def logical_size(self) -> LogicalSize:
        width, height = _window_logical_size(self._require_open())
        return LogicalSize(width=width, height=height)

    def physical_size(self) -> PhysicalSize:
        width, height = _window_physical_size(self._require_open())
        return PhysicalSize(width=width, height=height)

    def scale_factor(self) -> float:
        return _window_scale_factor(self._require_open())

    def request_redraw(self) -> None:
        _window_request_redraw(self._require_open())

    def set_title(self, title: str) -> None:
        _window_set_title(self._require_open(), title)

    def set_visible(self, visible: bool) -> None:
        _window_set_visible(self._require_open(), visible)

    def render_target_kind(self) -> RenderTargetKind:
        return RenderTargetKind(_window_render_target_kind(self._require_open()))


def create_window(
    *,
    title: str = "Coding Adventures Window",
    width: float = 800.0,
    height: float = 600.0,
    preferred_surface: SurfacePreference = SurfacePreference.DEFAULT,
    visible: bool = True,
    resizable: bool = True,
    decorations: bool = True,
    transparent: bool = False,
) -> Window:
    width = _normalize_dimension("width", width)
    height = _normalize_dimension("height", height)

    handle = _create_window(
        title,
        width,
        height,
        int(preferred_surface),
        bool(visible),
        bool(resizable),
        bool(decorations),
        bool(transparent),
    )
    return Window(handle)


__all__ = [
    "LogicalSize",
    "PhysicalSize",
    "RenderTargetKind",
    "SurfacePreference",
    "Window",
    "WindowError",
    "create_window",
]
