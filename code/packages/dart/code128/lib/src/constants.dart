/// Code 128 symbol patterns indexed by value (0-106).
///
/// Transcribed programmatically from the reference table; values
/// 0-103 are 11 modules, the stop symbol (106) is 13 modules.
/// Every pattern starts with a bar and (except stop) ends with a
/// space, so concatenated symbols alternate without gaps.
library;

/// Package version, mirroring the reference.
const String code128Version = '0.1.0';

/// Start Code B symbol value.
const int code128StartB = 104;

/// Stop symbol value.
const int code128Stop = 106;

/// Checksum modulus.
const int code128Modulus = 103;

/// Code Set B value base: value = code point - 32.
const int code128ValueBase = 32;

/// Modules per data/check symbol; the stop symbol uses 13.
const int code128SymbolModules = 11;
const int code128StopModules = 13;

/// Maximum accepted input scalars (spec: `input-too-long` above this).
const int code128MaxScalarLimit = 4096;

/// Default quiet zone, mirroring the sibling lanes.
const int code128QuietZoneModules = 10;

/// Printable-ASCII domain bounds (space through tilde).
const int code128AsciiMin = 0x20;
const int code128AsciiMax = 0x7e;

/// UTF-16 surrogate bounds for host-string validation.
const int code128HighSurrogateMin = 0xd800;
const int code128HighSurrogateMax = 0xdbff;
const int code128LowSurrogateMin = 0xdc00;
const int code128LowSurrogateMax = 0xdfff;

/// The 107 symbol patterns.
const List<String> code128Patterns = [
  '11011001100', // 0
  '11001101100', // 1
  '11001100110', // 2
  '10010011000', // 3
  '10010001100', // 4
  '10001001100', // 5
  '10011001000', // 6
  '10011000100', // 7
  '10001100100', // 8
  '11001001000', // 9
  '11001000100', // 10
  '11000100100', // 11
  '10110011100', // 12
  '10011011100', // 13
  '10011001110', // 14
  '10111001100', // 15
  '10011101100', // 16
  '10011100110', // 17
  '11001110010', // 18
  '11001011100', // 19
  '11001001110', // 20
  '11011100100', // 21
  '11001110100', // 22
  '11101101110', // 23
  '11101001100', // 24
  '11100101100', // 25
  '11100100110', // 26
  '11101100100', // 27
  '11100110100', // 28
  '11100110010', // 29
  '11011011000', // 30
  '11011000110', // 31
  '11000110110', // 32
  '10100011000', // 33
  '10001011000', // 34
  '10001000110', // 35
  '10110001000', // 36
  '10001101000', // 37
  '10001100010', // 38
  '11010001000', // 39
  '11000101000', // 40
  '11000100010', // 41
  '10110111000', // 42
  '10110001110', // 43
  '10001101110', // 44
  '10111011000', // 45
  '10111000110', // 46
  '10001110110', // 47
  '11101110110', // 48
  '11010001110', // 49
  '11000101110', // 50
  '11011101000', // 51
  '11011100010', // 52
  '11011101110', // 53
  '11101011000', // 54
  '11101000110', // 55
  '11100010110', // 56
  '11101101000', // 57
  '11101100010', // 58
  '11100011010', // 59
  '11101111010', // 60
  '11001000010', // 61
  '11110001010', // 62
  '10100110000', // 63
  '10100001100', // 64
  '10010110000', // 65
  '10010000110', // 66
  '10000101100', // 67
  '10000100110', // 68
  '10110010000', // 69
  '10110000100', // 70
  '10011010000', // 71
  '10011000010', // 72
  '10000110100', // 73
  '10000110010', // 74
  '11000010010', // 75
  '11001010000', // 76
  '11110111010', // 77
  '11000010100', // 78
  '10001111010', // 79
  '10100111100', // 80
  '10010111100', // 81
  '10010011110', // 82
  '10111100100', // 83
  '10011110100', // 84
  '10011110010', // 85
  '11110100100', // 86
  '11110010100', // 87
  '11110010010', // 88
  '11011011110', // 89
  '11011110110', // 90
  '11110110110', // 91
  '10101111000', // 92
  '10100011110', // 93
  '10001011110', // 94
  '10111101000', // 95
  '10111100010', // 96
  '11110101000', // 97
  '11110100010', // 98
  '10111011110', // 99
  '10111101110', // 100
  '11101011110', // 101
  '11110101110', // 102
  '11010000100', // 103
  '11010010000', // 104
  '11010011100', // 105
  '1100011101011', // 106
];
