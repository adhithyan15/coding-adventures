/// ML02 gradient-descent optimizers for Dart.
library;

export 'src/constants.dart' show gradientDescentVersion, maxVectorLength;
export 'src/errors.dart'
    show
        GradientDescentErrorCode,
        GradientDescentError,
        InvalidGradientDescentInputError;
export 'src/sgd.dart' show sgd;
