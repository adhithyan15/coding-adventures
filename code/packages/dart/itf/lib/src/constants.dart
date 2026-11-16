/// Standard constants for Interleaved 2 of 5 (ITF) encoding and layout.
const String itfVersion = '0.1.0';

const String itfStartPattern = '1010';
const String itfStopPattern = '11101';

const List<String> itfDigitPatterns = [
  '00110', // 0: NNWWN
  '10001', // 1: WNNNW
  '01001', // 2: NWNNW
  '11000', // 3: WWNNN
  '00101', // 4: NNWNW
  '10100', // 5: WNWNN
  '01100', // 6: NWWNN
  '00011', // 7: NNNWW
  '10010', // 8: WNNWN
  '01010', // 9: NWNWN
];

const int itfMaxScalarLimit = 4096;
const int itfQuietZoneModules = 10;
const int itfDigitPairRunCount = 10;
const int itfDigitElementCount = 5;
const int itfPairLength = 2;
const int itfFramingSymbolCount = 2;

const int itfStartPatternModules = 4;
const int itfStopPatternModules = 5;

const int itfHighSurrogateMin = 0xd800;
const int itfHighSurrogateMax = 0xdbff;
const int itfLowSurrogateMin = 0xdc00;
const int itfLowSurrogateMax = 0xdfff;

const int itfAsciiZero = 0x30;
const int itfAsciiNine = 0x39;
const int itfWideMarkerCodeUnit = 0x31;

const String itfRoleStart = 'start';
const String itfRoleData = 'data';
const String itfRoleStop = 'stop';

const String itfLabelStart = 'start';
const String itfLabelStop = 'stop';

const int itfSourceIndexStart = -1;
const int itfSourceIndexStop = -2;

const String itfWideBar = '111';
const String itfNarrowBar = '1';
const String itfWideSpace = '000';
const String itfNarrowSpace = '0';

const String itfMetadataKeySymbology = 'symbology';
const String itfMetadataValueSymbology = 'itf';
const String itfMetadataKeyPairCount = 'pairCount';

const String itfDefaultSceneLabel = 'ITF barcode';
const String itfDefaultForeground = '#000000';
const String itfDefaultBackground = '#ffffff';
const int itfDefaultModuleWidth = 4;
const int itfDefaultBarHeight = 120;
