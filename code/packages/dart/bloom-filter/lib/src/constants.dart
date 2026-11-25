/// Named constants for DT22 Bloom filter.

const int defaultExpectedItems = 1000;
const double defaultFalsePositiveRate = 0.01;
const int maximumBitCount = 134217728;
const int maximumHashCount = 64;

const int bitsPerByte = 8;
const int fmix32Shift1 = 16;
const int fmix32Shift2 = 13;
const int fmix32Mul1 = 0x85EBCA6B;
const int fmix32Mul2 = 0xC2B2AE35;
const int mask32 = 0xFFFFFFFF;
const int maxSafeInt = 0x7FFFFFFFFFFFFFFF;

const int highSurrogateMin = 0xD800;
const int highSurrogateMax = 0xDBFF;
const int lowSurrogateMin = 0xDC00;
const int lowSurrogateMax = 0xDFFF;

const String redactedPlaceholder = '<redacted>';
const String invalidPlaceholder = '<invalid>';
