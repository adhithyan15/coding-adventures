/// Static diagnostic messages for HyperLogLog under Standing Rule #20.

const String messageInvalidPrecision =
    'Precision must be between 4 and 16 inclusive.';
const String messagePrecisionMismatch =
    'Cannot merge HyperLogLog sketches with different precision values.';
const String messageInvalidDesiredError =
    'Desired error must be a finite positive number.';
const String messageInvalidInput = 'Input must not be null.';
