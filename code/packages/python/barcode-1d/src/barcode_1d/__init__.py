"""High-level 1D barcode pipeline for Python."""

from __future__ import annotations

import importlib
import platform
from typing import Any

import codabar
import code128
import code39
import ean_13
import itf
from pixel_container import PixelContainer
import upc_a

__version__ = "0.1.0"

DEFAULT_LAYOUT_CONFIG = code39.DEFAULT_LAYOUT_CONFIG
DEFAULT_RENDER_CONFIG = DEFAULT_LAYOUT_CONFIG

__all__ = [
    "Barcode1DError",
    "BackendUnavailableError",
    "UnsupportedSymbologyError",
    "build_scene",
    "current_backend",
    "render_pixels",
    "render_png",
]


class Barcode1DError(RuntimeError):
    """Base error for the high-level 1D barcode pipeline."""


class UnsupportedSymbologyError(Barcode1DError):
    """Raised when the requested 1D barcode symbology is not available."""


class BackendUnavailableError(Barcode1DError):
    """Raised when the host OS cannot execute a native backend."""


def current_backend() -> str | None:
    if platform.system() == "Darwin" and platform.machine().lower() in {"arm64", "aarch64"}:
        return "metal"
    return None


def _normalize_symbology(symbology: str) -> str:
    normalized = symbology.replace("-", "").replace("_", "").lower()
    if normalized == "code39":
        return "code39"
    if normalized == "codabar":
        return "codabar"
    if normalized == "code128":
        return "code128"
    if normalized == "ean13":
        return "ean13"
    if normalized == "itf":
        return "itf"
    if normalized == "upca":
        return "upca"
    raise UnsupportedSymbologyError(f"unsupported symbology: {symbology}")


def build_scene(
    data: str,
    *,
    symbology: str = "code39",
    layout_config: Any = DEFAULT_LAYOUT_CONFIG,
):
    match _normalize_symbology(symbology):
        case "code39":
            return code39.layout_code39(data, layout_config)
        case "codabar":
            return codabar.layout_codabar(data, layout_config)
        case "code128":
            return code128.layout_code128(data, layout_config)
        case "ean13":
            return ean_13.layout_ean_13(data, layout_config)
        case "itf":
            return itf.layout_itf(data, layout_config)
        case "upca":
            return upc_a.layout_upc_a(data, layout_config)


def _load_module(module_name: str):
    try:
        return importlib.import_module(module_name)
    except ImportError as exc:  # pragma: no cover - exercised on unsupported hosts
        raise BackendUnavailableError(f"{module_name} is not installed") from exc


def render_pixels(
    data: str,
    *,
    symbology: str = "code39",
    layout_config: Any = DEFAULT_LAYOUT_CONFIG,
) -> PixelContainer:
    backend = current_backend()
    if backend != "metal":
        raise BackendUnavailableError("no native Paint VM is available for this host")

    scene = build_scene(data, symbology=symbology, layout_config=layout_config)
    return _load_module("paint_vm_metal_native").render(scene)


def render_png(
    data: str,
    *,
    symbology: str = "code39",
    layout_config: Any = DEFAULT_LAYOUT_CONFIG,
) -> bytes:
    pixels = render_pixels(data, symbology=symbology, layout_config=layout_config)
    return _load_module("paint_codec_png_native").encode(pixels)
