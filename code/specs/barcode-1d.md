# Barcode 1D

## Overview

This spec defines the shared abstraction for linear barcode formats in the
coding-adventures monorepo.

The goal is to make multiple 1D symbologies look different where they must be
different, but feel the same once they cross into a reusable geometry layer.

The central idea is simple:

- Code 39, Codabar, and ITF talk about narrow and wide elements
- UPC-A, EAN-13, and Code 128 talk about module widths and guard patterns
- all of them can still be represented as a left-to-right sequence of bar and
  space runs with numeric widths

That shared run stream is the seam between symbology logic and rendering.

## Goals

- provide one reusable linear-barcode geometry layer
- expose explainable intermediate data for visualizers
- avoid baking SVG concerns into barcode packages
- let future backends render the same scene to SVG, Canvas, PNG, or another
  output format

## Non-Goals

- decoding or scanner simulation
- automatic format selection from arbitrary input
- pixel-level rasterization

## Core Data Model

### Runs

Every 1D barcode should expand into a stream of alternating runs.

```typescript
type Barcode1DRunColor = "bar" | "space";

interface Barcode1DRun {
  color: Barcode1DRunColor;
  modules: number;
  sourceLabel: string;
  sourceIndex: number;
  role:
    | "data"
    | "start"
    | "stop"
    | "guard"
    | "check"
    | "inter-character-gap";
}
```

This structure deliberately stays barcode-aware. It still answers questions
such as:

- which digit or symbol produced this run?
- is this a guard pattern or data?
- how wide is it in barcode modules?

### Symbol Layout

Visualizers and human-readable overlays often need to know the span occupied by
each encoded symbol.

```typescript
interface Barcode1DSymbolLayout {
  label: string;
  startModule: number;
  endModule: number;
  sourceIndex: number;
  role: "data" | "start" | "stop" | "guard" | "check";
}
```

`startModule` is inclusive and `endModule` is exclusive.

### Scene Layout Metadata

The shared package should calculate the total geometry before renderers see it.

```typescript
interface Barcode1DLayout {
  leftQuietZoneModules: number;
  rightQuietZoneModules: number;
  contentModules: number;
  totalModules: number;
  symbolLayouts: Barcode1DSymbolLayout[];
}
```

## Render Configuration

The shared 1D package should define the geometry knobs used by all linear
symbologies:

```typescript
interface Barcode1DRenderConfig {
  moduleWidth: number;
  barHeight: number;
  quietZoneModules: number;
  includeHumanReadableText: boolean;
  textFontSize: number;
  textMargin: number;
  foreground: string;
  background: string;
}
```

These values affect drawing only. They must not change what data is encoded.

## Translation to Draw Instructions

The shared 1D package should provide a function similar to:

```typescript
function drawBarcode1D(
  runs: Barcode1DRun[],
  options?: {
    renderConfig?: Partial<Barcode1DRenderConfig>;
    humanReadableText?: string;
    metadata?: Record<string, string | number | boolean>;
  },
): DrawScene;
```

The default translation rules are:

1. Start at the left quiet zone.
2. Convert each run's module width into scene width using `moduleWidth`.
3. Emit rectangles only for `bar` runs.
4. Keep spaces implicit by advancing `x`.
5. Add optional text below the bars.
6. Return scene metadata that preserves the original layout information.

## Why Numeric Modules?

Numeric module widths are the right shared unit because:

- Code 39 can map narrow and wide to `1` and `3`
- Codabar can map narrow and wide to `1` and `2` or `1` and `3`, depending on
  the chosen renderer ratio
- UPC-A and EAN-13 already work in equal modules
- Code 128 characters are defined as six run widths whose total is 11 modules

This lets the shared package stay simple without flattening away useful
symbology detail.

## Public API

The shared TypeScript package should export:

- `Barcode1DRun`
- `Barcode1DSymbolLayout`
- `Barcode1DLayout`
- `Barcode1DRenderConfig`
- `DEFAULT_BARCODE_1D_RENDER_CONFIG`
- `computeBarcode1DLayout()`
- `drawBarcode1D()`

## Future Extensions

- optional per-symbol label rendering under exact spans
- guard-bar extensions for retail formats
- separate text-layout strategies for UPC/EAN
- overlays for scanner simulation
