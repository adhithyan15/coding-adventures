/// Package constants and version limits for Java lexical analysis.
const String javaLexerPackageVersion = '0.1.0';

const String defaultJavaVersion = '21';

const List<String> supportedJavaVersions = [
  '1.0',
  '1.1',
  '1.4',
  '5',
  '7',
  '8',
  '10',
  '14',
  '17',
  '21',
];

/// Canonical Java editions supported by grammar specifications.
const List<String> canonicalJavaVersions = supportedJavaVersions;

/// Complete lookup table mapping caller-provided version aliases to canonical version strings.
/// Eliminates string slicing and inline magic literals in operational logic.
const Map<String, String> javaVersionAliases = {
  '1.0': '1.0',
  'java1.0': '1.0',
  'java1_0': '1.0',
  '1_0': '1.0',
  '1.1': '1.1',
  'java1.1': '1.1',
  'java1_1': '1.1',
  '1_1': '1.1',
  '1.4': '1.4',
  'java1.4': '1.4',
  'java1_4': '1.4',
  '1_4': '1.4',
  '5': '5',
  'java5': '5',
  '1.5': '5',
  'java1.5': '5',
  'java1_5': '5',
  '1_5': '5',
  '7': '7',
  'java7': '7',
  '1.7': '7',
  'java1.7': '7',
  'java1_7': '7',
  '1_7': '7',
  '8': '8',
  'java8': '8',
  '1.8': '8',
  'java1.8': '8',
  'java1_8': '8',
  '1_8': '8',
  '10': '10',
  'java10': '10',
  '14': '14',
  'java14': '14',
  '17': '17',
  'java17': '17',
  '21': '21',
  'java21': '21',
};

/// Source ceiling: 1,048,576 UTF-16 code units (CWE-400 mitigation).
/// Dart `String.length` counts UTF-16 code units; the Swift lane counts
/// `source.utf16.count` so both lanes accept exactly the same inputs.
const int maxSourceLength = 1048576;
const int maxFixtureBytes = 2097152; // 2 MB fixture boundary (bytes)

const int highSurrogateMin = 0xd800;
const int highSurrogateMax = 0xdbff;
const int lowSurrogateMin = 0xdc00;
const int lowSurrogateMax = 0xdfff;
