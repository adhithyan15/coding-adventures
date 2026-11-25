/// Standard constants for EAN-13 encoding and layout.
library;

/// Package version, mirroring the reference.
const String ean13Version = '0.1.0';

/// Side guard bars (`101`), opening and closing the symbol.
const String ean13SideGuard = '101';

/// Center guard bars (`01010`) between the left and right halves.
const String ean13CenterGuard = '01010';

/// L-code (odd-parity) patterns for digits 0-9, each 7 modules.
const List<String> ean13LeftPatterns = [
  '0001101', // 0
  '0011001', // 1
  '0010011', // 2
  '0111101', // 3
  '0100011', // 4
  '0110001', // 5
  '0101111', // 6
  '0111011', // 7
  '0110111', // 8
  '0001011', // 9
];

/// G-code (even-parity, mirrored) patterns for digits 0-9, each 7 modules.
const List<String> ean13GPatterns = [
  '0100111', // 0
  '0110011', // 1
  '0011011', // 2
  '0100001', // 3
  '0011101', // 4
  '0111001', // 5
  '0000101', // 6
  '0010001', // 7
  '0001001', // 8
  '0010111', // 9
];

/// R-code patterns for digits 0-9, each 7 modules.
const List<String> ean13RightPatterns = [
  '1110010', // 0
  '1100110', // 1
  '1101100', // 2
  '1000010', // 3
  '1011100', // 4
  '1001110', // 5
  '1010000', // 6
  '1000100', // 7
  '1001000', // 8
  '1110100', // 9
];

/// Left-half L/G parity selections indexed by the leading digit. The
/// leading digit itself is not bar-encoded; its value hides in the
/// parity mix of the next six digits.
const List<String> ean13LeftParityPatterns = [
  'LLLLLL', // 0
  'LLGLGG', // 1
  'LLGGLG', // 2
  'LLGGGL', // 3
  'LGLLGG', // 4
  'LGGLLG', // 5
  'LGGGLL', // 6
  'LGLGLG', // 7
  'LGLGGL', // 8
  'LGGLGL', // 9
];

/// Maximum accepted input scalars (spec: `input-too-long` above this).
const int ean13MaxScalarLimit = 4096;

/// Accepted payload lengths: 12 digits (check digit computed) or 13
/// digits (check digit verified).
const List<int> ean13AcceptedLengths = [12, 13];

/// Digits per encoded half of the symbol (excluding the leading digit).
const int ean13HalfLength = 6;

/// Modules per encoded digit.
const int ean13DigitModules = 7;

/// Total modules of an EAN-13 symbol: 3 + 6*7 + 5 + 6*7 + 3.
const int ean13TotalModules = 95;

/// Default quiet zone, mirroring the ITF/UPC-A lanes and the reference.
const int ean13QuietZoneModules = 10;

/// UTF-16 surrogate bounds for host-string validation.
const int ean13HighSurrogateMin = 0xd800;
const int ean13HighSurrogateMax = 0xdbff;
const int ean13LowSurrogateMin = 0xdc00;
const int ean13LowSurrogateMax = 0xdfff;

/// ASCII digit bounds (`0`-`9` only; no Unicode digits, no folding).
const int ean13AsciiZero = 0x30;
const int ean13AsciiNine = 0x39;
