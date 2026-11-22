/// Static diagnostic messages for BloomFilter under Standing Rule #20.

const String messageInvalidExpectedItems =
    'Expected items must be greater than zero.';
const String messageInvalidFalsePositiveRate =
    'False positive rate must be between 0.0 and 1.0 exclusive.';
const String messageInvalidBitCount = 'Bit count must be greater than zero.';
const String messageInvalidHashCount =
    'Hash count must be between 1 and 64 inclusive.';
const String messageInvalidMemoryBytes =
    'Memory bytes must be greater than zero and within allocation limits.';
const String messageAllocationLimitExceeded =
    'Requested bit count exceeds maximum allowed allocation.';
const String messageCapacityOverflow =
    'Calculated filter capacity exceeds integer limits.';
const String messageInvalidInput = 'Input must not be null.';
