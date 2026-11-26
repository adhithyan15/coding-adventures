/// Standard constants for UPC-A encoding and layout.
library;

/// Package version, mirroring the reference.
const String upcAVersion = '0.1.0';

/// Side guard bars (`101`), opening and closing the symbol.
const String upcASideGuard = '101';

/// Center guard bars (`01010`) between the left and right halves.
const String upcACenterGuard = '01010';

/// L-code (odd-parity) patterns for digits 0-9, each 7 modules.
const List<String> upcALeftPatterns = [
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

/// R-code (even-parity) patterns for digits 0-9, each 7 modules.
const List<String> upcARightPatterns = [
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

/// Maximum accepted input scalars (spec: `input-too-long` above this).
const int upcAMaxScalarLimit = 4096;

/// Accepted payload lengths: 11 digits (check digit computed) or 12
/// digits (check digit verified).
const List<int> upcAAcceptedLengths = [11, 12];

/// Digits per half of the symbol.
const int upcAHalfLength = 6;

/// Modules per encoded digit.
const int upcADigitModules = 7;

/// Total modules of a UPC-A symbol: 3 + 6*7 + 5 + 6*7 + 3.
const int upcATotalModules = 95;

/// Default quiet zone, mirroring the ITF lane and the reference default.
const int upcAQuietZoneModules = 10;

/// UTF-16 surrogate bounds for host-string validation.
const int upcAHighSurrogateMin = 0xd800;
const int upcAHighSurrogateMax = 0xdbff;
const int upcALowSurrogateMin = 0xdc00;
const int upcALowSurrogateMax = 0xdfff;

/// ASCII digit bounds (`0`-`9` only; no Unicode digits, no folding).
const int upcAAsciiZero = 0x30;
const int upcAAsciiNine = 0x39;
