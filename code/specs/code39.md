# Code 39

## Overview

This spec defines a dependency-free **Code 39** barcode package for the
coding-adventures monorepo.

The goal is to understand how a string becomes a machine-readable 1D barcode,
and to expose that structure clearly enough that a teaching visualizer can
build on top of it.

V1 deliberately focuses on:

- encoding a string into **Code 39**
- validating that the input only uses supported characters
- producing an explicit sequence of bars and spaces
- building backend-neutral draw instructions
- rendering those draw instructions as **SVG** via the first backend
- exposing intermediate structures for a future visualization app

This package has **no external dependencies**. If a target language/runtime
already provides native drawing primitives, file output, or string handling, we
may use them. Otherwise we build the required pieces ourselves.

## Why Code 39 First?

Code 39 is the best first 1D barcode for this repository because:

1. It is easy to explain.
2. It supports both digits and uppercase letters, so we can encode strings.
3. Each character is encoded independently, which makes it ideal for a
   visualizer.
4. It has explicit start/stop markers.
5. The checksum is optional, so the base version stays simple.

We are not starting with UPC-A or EAN-13 because those formats are more
constrained and add parity and guard-pattern rules that are useful later, but
not ideal for the first implementation.

## What the Package Does

Given an input string such as:

```text
HELLO123
```

the package:

1. normalizes and validates the input
2. wraps it with Code 39 start/stop characters (`*`)
3. maps each character to a Code 39 narrow/wide pattern
4. expands the pattern into an ordered run list of black bars and white spaces
5. builds backend-neutral draw instructions
6. renders the result through a backend such as SVG

The package should expose both:

- a **semantic representation** for learning and visualization
- a **renderable representation** for generating the final SVG

## Core Concepts

### Code 39 Alphabet

V1 supports the standard Code 39 character set:

```text
0-9
A-Z
- . space $ / + %
*   (reserved for start/stop only)
```

The application input must not contain `*` directly. The encoder inserts it
automatically as the start and stop delimiter.

### Bars and Spaces

Code 39 is a 1D symbology built from alternating:

- black bars
- white spaces

Each encoded character consists of **9 elements**:

- 5 bars
- 4 spaces

Of those 9 elements:

- 3 are **wide**
- 6 are **narrow**

Characters are separated by an additional **narrow inter-character gap**.

### Narrow and Wide

Code 39 is a width-based symbology. It does not care about absolute pixel size.
It cares about relative width classes:

- narrow
- wide

The renderer chooses concrete dimensions, for example:

```text
narrow = 4 px
wide   = 12 px
```

V1 should treat the ratio as a renderer configuration value, not as part of
the encoded data.

### Start and Stop

Every Code 39 barcode begins and ends with `*`. This is part of the symbology
itself, not part of the user's data.

For example:

```text
Input data: HELLO123
Encoded sequence: *HELLO123*
```

### Optional Checksum

Classic Code 39 allows an optional modulo-43 checksum. V1 does **not** require
it. The API should leave room for it to be added later without breaking the
core encoding model.

## Scope

### V1 In Scope

- Code 39 encoding only
- uppercase alphanumeric and Code 39 punctuation support
- start/stop insertion
- validation with clear error messages
- intermediate structured output for visualization
- translation to backend-neutral draw instructions
- deterministic output for tests

### V1 Out of Scope

- ASCII extension mode
- modulo-43 checksum
- image decoding / scanner implementation
- damaged-barcode recovery

## Data Model

The package should separate the barcode into progressively richer layers.

### Encoded Characters

```typescript
interface EncodedCharacter {
  char: string;                     // e.g. "H"
  isStartStop: boolean;             // true only for inserted '*'
  pattern: string;                  // 9-character narrow/wide pattern
}
```

`pattern` is a 9-element sequence describing alternating bar/space widths for
that character.

### Runs

```typescript
type RunColor = "bar" | "space";
type RunWidth = "narrow" | "wide";

interface BarcodeRun {
  color: RunColor;
  width: RunWidth;
  sourceChar: string;
  sourceIndex: number;
  isInterCharacterGap: boolean;
}
```

This structure is the most important output for the future visualizer.

### Draw Instructions

```typescript
interface RenderConfig {
  narrowUnit: number;
  wideUnit: number;
  barHeight: number;
  quietZoneUnits: number;
  includeHumanReadableText: boolean;
}

interface DrawScene { ... }
type DrawInstruction = DrawRect | DrawText | DrawGroup;
```

The draw-instructions package is the reusable seam:

- new 1D symbologies can reuse it
- SVG is only one renderer
- future frontends can target a canvas, terminal, PDF, or custom explainer view
  without changing the encoding logic

## Encoding Rules

### Normalization

V1 normalization rules:

- accept string input
- preserve spaces
- convert lowercase letters to uppercase before validation
- reject characters outside the Code 39 alphabet
- reject `*` in user input because it is reserved for start/stop

### Character Mapping

The package must include a complete Code 39 lookup table mapping each supported
character to its 9-element narrow/wide pattern.

This mapping table is part of the source code and should be documented clearly.

### Run Expansion

For each encoded character:

1. start with a bar
2. alternate bar, space, bar, space, ...
3. emit 9 runs using the character's pattern
4. if this is not the final character, append one narrow space as the
   inter-character gap

### Quiet Zone

The draw-instructions translation must include left and right quiet zones.

### Human-Readable Text

The draw scene may optionally include the original data text below the bars. In
V1, the text should show the user's input, not the inserted `*` markers.

## Public API

The public API is described in language-neutral pseudocode.

```python
class BarcodeError(Exception): ...
class InvalidCharacterError(BarcodeError): ...
class InvalidConfigurationError(BarcodeError): ...


class EncodedCharacter:
    char: str
    is_start_stop: bool
    pattern: str


class BarcodeRun:
    color: str
    width: str
    source_char: str
    source_index: int
    is_inter_character_gap: bool


class RenderConfig:
    narrow_unit: int
    wide_unit: int
    bar_height: int
    quiet_zone_units: int
    include_human_readable_text: bool

def normalize_code39(data: str) -> str: ...
def encode_code39_char(char: str) -> EncodedCharacter: ...
def encode_code39(data: str) -> list[EncodedCharacter]: ...
def expand_code39_runs(data: str) -> list[BarcodeRun]: ...
def draw_one_dimensional_barcode(
    runs: list[BarcodeRun],
    text: str | None,
    config: RenderConfig
) -> DrawScene: ...
def draw_code39(data: str, config: RenderConfig) -> DrawScene: ...
def render_code39(data: str, backend: Renderer[T], config: RenderConfig) -> T: ...
```

## SVG Rendering Contract

SVG is the preferred initial output format because:

- it is text-based
- it is easy to generate with no dependencies
- it is inspectable in tests
- it maps directly onto the conceptual model of vertical bars
- it works naturally in browser-based visualizers

The SVG renderer package should:

1. consume backend-neutral draw instructions
2. emit one `<rect>` per black bar
3. include overall `width`, `height`, and `viewBox`
4. optionally include a `<text>` label below the bars

## Error Handling

Errors must be explicit and educational.

Examples:

- `Invalid character: "@" is not supported by Code 39`
- `Input must not contain "*" because it is reserved for start/stop`
- `wide_unit must be greater than narrow_unit`
- `bar_height must be positive`

## Data Flow

```text
User input string
  -> normalize and validate
  -> insert start/stop markers
  -> map characters to Code 39 patterns
  -> expand to alternating runs
  -> convert symbolic widths to backend-neutral draw instructions
  -> render with SVG backend
```

## Testing Strategy

All implementations should include:

1. normalization tests
2. invalid-character tests
3. exact lookup-table tests for selected characters
4. full encoded-sequence tests including start/stop insertion
5. run-expansion tests verifying alternating bar/space structure
6. layout tests verifying x-positions and widths
7. SVG snapshot or structural tests

## Future Extensions

- modulo-43 checksum support
- ASCII extension mode
- scanner simulation over the run sequence
- a shared JSON export format for barcode explanations
