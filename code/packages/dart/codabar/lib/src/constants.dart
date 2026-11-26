/// Standard constants for Codabar encoding and layout.
library;

/// Package version, mirroring the reference.
const String codabarVersion = '0.1.0';

/// The four guard (start/stop) characters.
const List<String> codabarGuards = ['A', 'B', 'C', 'D'];

/// Default guard when the matching option is absent.
const String codabarDefaultGuard = 'A';

/// Binary module patterns per character. Body characters use 9 modules;
/// `:`, `/`, `.`, and `+` use 10.
const Map<String, String> codabarPatterns = {
  '0': '101010011',
  '1': '101011001',
  '2': '101001011',
  '3': '110010101',
  '4': '101101001',
  '5': '110101001',
  '6': '100101011',
  '7': '100101101',
  '8': '100110101',
  '9': '110100101',
  '-': '101001101',
  r'$': '101100101',
  ':': '1101011011',
  '/': '1101101011',
  '.': '1101101101',
  '+': '1011011011',
  'A': '1011001001',
  'B': '1001001011',
  'C': '1010010011',
  'D': '1010011001',
};

/// Maximum accepted input scalars (spec: `input-too-long` above this).
const int codabarMaxScalarLimit = 4096;

/// Default quiet zone, mirroring the sibling lanes.
const int codabarQuietZoneModules = 10;

/// UTF-16 surrogate bounds for host-string validation.
const int codabarHighSurrogateMin = 0xd800;
const int codabarHighSurrogateMax = 0xdbff;
const int codabarLowSurrogateMin = 0xdc00;
const int codabarLowSurrogateMax = 0xdfff;

/// ASCII lowercase bounds (the only folded range, per the spec).
const int codabarAsciiLowerA = 0x61;
const int codabarAsciiLowerZ = 0x7a;

/// ASCII case-fold distance (`a` - `A`).
const int codabarAsciiCaseDelta = 0x20;
