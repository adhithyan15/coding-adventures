"""Python wrapper for the native Metal Paint VM bridge."""

from __future__ import annotations

import platform
from typing import Any

from pixel_container import PixelContainer

try:
    from paint_vm_metal_native.paint_vm_metal_native import (  # type: ignore[import]
        render_rect_scene_native as _render_rect_scene_native,
    )
except ImportError:  # pragma: no cover - exercised on unsupported hosts
    _render_rect_scene_native = None

__all__ = ["PaintVmMetalNativeError", "available", "render", "supported_runtime"]


class PaintVmMetalNativeError(RuntimeError):
    """Raised when the native Metal Paint VM cannot execute."""


def supported_runtime() -> bool:
    return platform.system() == "Darwin" and platform.machine().lower() in {
        "arm64",
        "aarch64",
    }


def available() -> bool:
    return supported_runtime() and _render_rect_scene_native is not None


def _lookup_value(obj: Any, key: str, default: Any = ...):
    if isinstance(obj, dict):
        if key in obj:
            return obj[key]
        if default is ...:
            raise PaintVmMetalNativeError(f"scene is missing {key!r}")
        return default

    if hasattr(obj, key):
        return getattr(obj, key)

    if default is ...:
        raise PaintVmMetalNativeError(f"scene is missing {key!r}")
    return default


def _encode_instruction(instruction: Any) -> tuple[float, float, float, float, str]:
    kind = _lookup_value(instruction, "kind")
    if kind != "rect":
        raise PaintVmMetalNativeError("only rect paint instructions are supported right now")

    return (
        float(_lookup_value(instruction, "x")),
        float(_lookup_value(instruction, "y")),
        float(_lookup_value(instruction, "width")),
        float(_lookup_value(instruction, "height")),
        str(_lookup_value(instruction, "fill", "#000000")),
    )


def render(scene: Any) -> PixelContainer:
    if not supported_runtime():
        raise PaintVmMetalNativeError("Metal is only available on macOS arm64")
    if _render_rect_scene_native is None:
        raise PaintVmMetalNativeError("paint_vm_metal_native extension is not available")

    instructions = _lookup_value(scene, "instructions")
    rects = [_encode_instruction(instruction) for instruction in instructions]

    width, height, data = _render_rect_scene_native(
        float(_lookup_value(scene, "width")),
        float(_lookup_value(scene, "height")),
        str(_lookup_value(scene, "background", "#ffffff")),
        rects,
    )
    return PixelContainer(width=width, height=height, data=bytearray(data))
