/// Pure Dart Interleaved 2 of 5 (ITF) barcode encoder and layout.
library;

export 'src/constants.dart' show itfVersion;
export 'src/encoded_pair.dart' show EncodedPair;
export 'src/errors.dart' show InvalidItfInputError, ItfErrorCode, ItfError;
export 'src/itf_encoder.dart'
    show normalizeItf, encodeItf, expandItfRuns, layoutItf, drawItf;
