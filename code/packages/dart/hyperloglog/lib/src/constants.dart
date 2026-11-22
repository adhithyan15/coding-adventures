/// Named constants for DT21 HyperLogLog cardinality sketch.

const int defaultPrecision = 14;
const int minimumPrecision = 4;
const int maximumPrecision = 16;
const int hashBitWidth = 64;
const int fmix64Shift = 33;
final BigInt fmix64Mul1 = BigInt.parse('ff51afd7ed558ccd', radix: 16);
final BigInt fmix64Mul2 = BigInt.parse('c4ceb9fe1a85ec53', radix: 16);
final BigInt mask64 = BigInt.parse('ffffffffffffffff', radix: 16);

const double alpha16 = 0.673;
const double alpha32 = 0.697;
const double alpha64 = 0.709;
const double alphaInfNum = 0.7213;
const double alphaInfDenom = 1.079;

const double smallRangeFactor = 2.5;
const double largeRangeDivisor = 30.0;
const double twoTo64 = 18446744073709551616.0;
const double errorRateNumerator = 1.04;
const int registerBitWidth = 6;
const int bitsPerByte = 8;
const int maxSafeInt = 0x7FFFFFFFFFFFFFFF;

const int highSurrogateMin = 0xD800;
const int highSurrogateMax = 0xDBFF;
const int lowSurrogateMin = 0xDC00;
const int lowSurrogateMax = 0xDFFF;

const String redactedPlaceholder = '<redacted>';
const String invalidPlaceholder = '<invalid>';
