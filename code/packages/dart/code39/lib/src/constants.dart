/// Standard constants for Code 39 encoding and layout.
library;

/// Package version, mirroring the reference.
const String code39Version = '0.1.0';

/// Element patterns per character: nine alternating bar/space widths.
/// Lowercase is narrow, uppercase is wide; position 0, 2, 4, 6, 8 are
/// bars and 1, 3, 5, 7 are spaces.
const Map<String, String> code39Patterns = {
  '0': 'bwbWBwBwb',
  '1': 'BwbWbwbwB',
  '2': 'bwBWbwbwB',
  '3': 'BwBWbwbwb',
  '4': 'bwbWBwbwB',
  '5': 'BwbWBwbwb',
  '6': 'bwBWBwbwb',
  '7': 'bwbWbwBwB',
  '8': 'BwbWbwBwb',
  '9': 'bwBWbwBwb',
  'A': 'BwbwbWbwB',
  'B': 'bwBwbWbwB',
  'C': 'BwBwbWbwb',
  'D': 'bwbwBWbwB',
  'E': 'BwbwBWbwb',
  'F': 'bwBwBWbwb',
  'G': 'bwbwbWBwB',
  'H': 'BwbwbWBwb',
  'I': 'bwBwbWBwb',
  'J': 'bwbwBWBwb',
  'K': 'BwbwbwbWB',
  'L': 'bwBwbwbWB',
  'M': 'BwBwbwbWb',
  'N': 'bwbwBwbWB',
  'O': 'BwbwBwbWb',
  'P': 'bwBwBwbWb',
  'Q': 'bwbwbwBWB',
  'R': 'BwbwbwBWb',
  'S': 'bwBwbwBWb',
  'T': 'bwbwBwBWb',
  'U': 'BWbwbwbwB',
  'V': 'bWBwbwbwB',
  'W': 'BWBwbwbwb',
  'X': 'bWbwBwbwB',
  'Y': 'BWbwBwbwb',
  'Z': 'bWBwBwbwb',
  '-': 'bWbwbwBwB',
  '.': 'BWbwbwBwb',
  ' ': 'bWBwbwBwb',
  r'$': 'bWbWbWbwb',
  '/': 'bWbWbwbWb',
  '+': 'bWbwbWbWb',
  '%': 'bwbWbWbWb',
  '*': 'bWbwBwBwb',
};

/// The start/stop delimiter, also the reserved character.
const String code39StartStop = '*';

/// Elements per encoded character (nine alternating widths).
const int code39ElementsPerChar = 9;

/// Modules for wide and narrow elements in the canonical projection.
const int code39WideModules = 3;
const int code39NarrowModules = 1;

/// Maximum accepted input scalars (spec: `input-too-long` above this).
const int code39MaxScalarLimit = 4096;

/// Default quiet zone, mirroring the sibling lanes.
const int code39QuietZoneModules = 10;

/// UTF-16 surrogate bounds for host-string validation.
const int code39HighSurrogateMin = 0xd800;
const int code39HighSurrogateMax = 0xdbff;
const int code39LowSurrogateMin = 0xdc00;
const int code39LowSurrogateMax = 0xdfff;

/// ASCII lowercase bounds (the only folded range, per the spec).
const int code39AsciiLowerA = 0x61;
const int code39AsciiLowerZ = 0x7a;

/// ASCII case-fold distance (`a` - `A`).
const int code39AsciiCaseDelta = 0x20;
