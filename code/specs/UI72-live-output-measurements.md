# UI72: Live output and measurement semantics

## Status

Implemented for Venture's shared browser model and available host boundaries.

## Purpose

`output`, `meter`, and `progress` carry live semantics that must not be
reimplemented by AppKit, WinUI, Qt, Flutter, Compose, or generated web shells.
This contract keeps dependencies, normalization, reset behavior, diagnostics,
and accessibility state in the shared Rust browser layer.

## Retained model

Every rendered live-value element receives a stable document-order `live:*`
key. `LiveValueState` exposes its semantic kind, accessible name and
description, normalized value and bounds, fractional position, human-readable
value text, and diagnostics. Host projections are bounded JSON assembled from
this model; hosts may choose a widget but may not reinterpret the value.

An `output` retains its initial text as its reset value and the ordered IDs
from its `for` attribute. Dependency queries resolve those IDs against current
control values each time, so script calculation never observes stale input.
The shared `recalculate_output` transaction accepts the script-owned result,
updates retained output text, and reflows every host from the same render tree.
Reset restores authored text for outputs associated with that form.

## Meter normalization

Meters default to `min=0` and `max=1`. Bounds are finite, `max` cannot fall
below `min`, `low` and `high` are clamped in order, and `optimum` defaults to
the midpoint. Values are clamped to the effective range. The model publishes
the fraction and one of `optimum`, `suboptimal`, or `even-less-good`.

## Progress normalization

Progress defaults to `max=1`; a missing value is indeterminate and has no
fraction. A present finite value is clamped to `0...max`. Invalid values are
reported without leaking parsing policy into a host. Non-positive or invalid
maximums fall back to one and remain diagnostic.

## Rendering and accessibility

Live updates synchronize into `BrowserRenderTree` before shared layout and
paint reflow. Value text, normalized bounds, fraction, indeterminate state,
meter region, names, descriptions, and diagnostics form the reusable
accessibility projection. Native ABIs expose the same JSON for macOS, Windows,
Qt, Flutter, and Compose.

## Acceptance

Parser tests retain output dependency tokens and measurement attributes.
Reducer tests cover dependency refresh, scripted output updates, reset,
meter-region classification, clamping, determinate/indeterminate transitions,
diagnostics, and host JSON. Venture core and Mosaic acceptance prove reflow and
the native ABI surface.
