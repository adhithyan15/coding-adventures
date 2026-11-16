/// Closed enumeration of error codes for SHA-512 operations (Standing Rule #20).
enum Sha512ErrorCode {
  invalidByte,
  invalidOffsetOrLength,
  messageTooLong,
  corpusMalformed,
  corpusOversized,
  unknown,
}

/// Centralized error and diagnostic messages for SHA-512 (Standing Rule #20).
abstract final class Sha512Messages {
  static const String errInvalidByte = 'sha512: byte value out of range 0..255';
  static const String errInvalidOffsetOrLength =
      'sha512: offset or length out of bounds';
  static const String errMessageTooLong =
      'sha512: message length must be less than 2^128 bits';
  static const String errCorpusMalformed =
      'sha512: malformed fixture corpus row';
  static const String errCorpusOversized =
      'sha512: fixture corpus file exceeds 4096 bytes';
  static const String errUnknown = 'sha512: unknown error';

  static String messageFor(Sha512ErrorCode? code) {
    if (code == null) return errUnknown;
    switch (code) {
      case Sha512ErrorCode.invalidByte:
        return errInvalidByte;
      case Sha512ErrorCode.invalidOffsetOrLength:
        return errInvalidOffsetOrLength;
      case Sha512ErrorCode.messageTooLong:
        return errMessageTooLong;
      case Sha512ErrorCode.corpusMalformed:
        return errCorpusMalformed;
      case Sha512ErrorCode.corpusOversized:
        return errCorpusOversized;
      case Sha512ErrorCode.unknown:
        return errUnknown;
    }
  }
}
