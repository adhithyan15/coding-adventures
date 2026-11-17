/// Immutable 64-bit word represented as high and low 32-bit unsigned integers.
/// Guarantees bit-identical execution across 64-bit VM and 53-bit JavaScript web runtimes.
class Word64 {
  final int hi;
  final int lo;

  const Word64(this.hi, this.lo);

  static const Word64 zero = Word64(0, 0);

  Word64 operator +(Word64 other) {
    final sumLo = lo + other.lo;
    final carry = sumLo > 0xFFFFFFFF ? 1 : 0;
    final newLo = sumLo & 0xFFFFFFFF;
    final newHi = (hi + other.hi + carry) & 0xFFFFFFFF;
    return Word64(newHi, newLo);
  }

  Word64 operator ^(Word64 other) {
    return Word64(hi ^ other.hi, lo ^ other.lo);
  }

  Word64 operator &(Word64 other) {
    return Word64(hi & other.hi, lo & other.lo);
  }

  Word64 operator |(Word64 other) {
    return Word64(hi | other.hi, lo | other.lo);
  }

  Word64 operator ~() {
    return Word64((~hi) & 0xFFFFFFFF, (~lo) & 0xFFFFFFFF);
  }

  Word64 rotr(int n) {
    if (n == 0) return this;
    if (n < 32) {
      final newHi = ((hi >>> n) | (lo << (32 - n))) & 0xFFFFFFFF;
      final newLo = ((lo >>> n) | (hi << (32 - n))) & 0xFFFFFFFF;
      return Word64(newHi, newLo);
    } else if (n == 32) {
      return Word64(lo, hi);
    } else {
      final shift = n - 32;
      final newHi = ((lo >>> shift) | (hi << (32 - shift))) & 0xFFFFFFFF;
      final newLo = ((hi >>> shift) | (lo << (32 - shift))) & 0xFFFFFFFF;
      return Word64(newHi, newLo);
    }
  }

  Word64 shr(int n) {
    if (n == 0) return this;
    if (n < 32) {
      final newHi = hi >>> n;
      final newLo = ((lo >>> n) | (hi << (32 - n))) & 0xFFFFFFFF;
      return Word64(newHi, newLo);
    } else if (n == 32) {
      return Word64(0, hi);
    } else {
      final newLo = hi >>> (n - 32);
      return Word64(0, newLo);
    }
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is Word64 && other.hi == hi && other.lo == lo);

  @override
  int get hashCode => Object.hash(hi, lo);
}

/// Centralized constants for SHA-512 (FIPS 180-4).
abstract final class Sha512Constants {
  static const int blockSize = 128;
  static const int digestLength = 64;
  static const int hexDigestLength = 128;
  static const int rounds = 80;
  static const int lengthFieldBytes = 16;
  static const int paddingOffset = 112;
  static const int paddingMarker = 0x80;
  static const int mask32 = 0xFFFFFFFF;

  static const int stateWords = 8;
  static const int scheduleInitWords = 16;
  static const int wordBytes = 8;
  static const int wordBits = 64;
  static const int halfWordBits = 32;
  static const int bitsPerByteShift = 3;
  static const int byteToBitCarryShift = 61;
  static const int nibbleBits = 4;
  static const int nibbleMask = 0x0F;
  static const int byteMask = 0xFF;

  static const int bs0R1 = 28;
  static const int bs0R2 = 34;
  static const int bs0R3 = 39;

  static const int bs1R1 = 14;
  static const int bs1R2 = 18;
  static const int bs1R3 = 41;

  static const int ss0R1 = 1;
  static const int ss0R2 = 8;
  static const int ss0S = 7;

  static const int ss1R1 = 19;
  static const int ss1R2 = 61;
  static const int ss1S = 6;

  static final BigInt overflowLimitHigh = BigInt.one << 61;
  static final BigInt mask64 = (BigInt.one << 64) - BigInt.one;

  static const List<Word64> initialState = [
    Word64(0x6A09E667, 0xF3BCC908),
    Word64(0xBB67AE85, 0x84CAA73B),
    Word64(0x3C6EF372, 0xFE94F82B),
    Word64(0xA54FF53A, 0x5F1D36F1),
    Word64(0x510E527F, 0xADE682D1),
    Word64(0x9B05688C, 0x2B3E6C1F),
    Word64(0x1F83D9AB, 0xFB41BD6B),
    Word64(0x5BE0CD19, 0x137E2179),
  ];

  static const List<Word64> k = [
    Word64(0x428A2F98, 0xD728AE22),
    Word64(0x71374491, 0x23EF65CD),
    Word64(0xB5C0FBCF, 0xEC4D3B2F),
    Word64(0xE9B5DBA5, 0x8189DBBC),
    Word64(0x3956C25B, 0xF348B538),
    Word64(0x59F111F1, 0xB605D019),
    Word64(0x923F82A4, 0xAF194F9B),
    Word64(0xAB1C5ED5, 0xDA6D8118),
    Word64(0xD807AA98, 0xA3030242),
    Word64(0x12835B01, 0x45706FBE),
    Word64(0x243185BE, 0x4EE4B28C),
    Word64(0x550C7DC3, 0xD5FFB4E2),
    Word64(0x72BE5D74, 0xF27B896F),
    Word64(0x80DEB1FE, 0x3B1696B1),
    Word64(0x9BDC06A7, 0x25C71235),
    Word64(0xC19BF174, 0xCF692694),
    Word64(0xE49B69C1, 0x9EF14AD2),
    Word64(0xEFBE4786, 0x384F25E3),
    Word64(0x0FC19DC6, 0x8B8CD5B5),
    Word64(0x240CA1CC, 0x77AC9C65),
    Word64(0x2DE92C6F, 0x592B0275),
    Word64(0x4A7484AA, 0x6EA6E483),
    Word64(0x5CB0A9DC, 0xBD41FBD4),
    Word64(0x76F988DA, 0x831153B5),
    Word64(0x983E5152, 0xEE66DFAB),
    Word64(0xA831C66D, 0x2DB43210),
    Word64(0xB00327C8, 0x98FB213F),
    Word64(0xBF597FC7, 0xBEEF0EE4),
    Word64(0xC6E00BF3, 0x3DA88FC2),
    Word64(0xD5A79147, 0x930AA725),
    Word64(0x06CA6351, 0xE003826F),
    Word64(0x14292967, 0x0A0E6E70),
    Word64(0x27B70A85, 0x46D22FFC),
    Word64(0x2E1B2138, 0x5C26C926),
    Word64(0x4D2C6DFC, 0x5AC42AED),
    Word64(0x53380D13, 0x9D95B3DF),
    Word64(0x650A7354, 0x8BAF63DE),
    Word64(0x766A0ABB, 0x3C77B2A8),
    Word64(0x81C2C92E, 0x47EDAEE6),
    Word64(0x92722C85, 0x1482353B),
    Word64(0xA2BFE8A1, 0x4CF10364),
    Word64(0xA81A664B, 0xBC423001),
    Word64(0xC24B8B70, 0xD0F89791),
    Word64(0xC76C51A3, 0x0654BE30),
    Word64(0xD192E819, 0xD6EF5218),
    Word64(0xD6990624, 0x5565A910),
    Word64(0xF40E3585, 0x5771202A),
    Word64(0x106AA070, 0x32BBD1B8),
    Word64(0x19A4C116, 0xB8D2D0C8),
    Word64(0x1E376C08, 0x5141AB53),
    Word64(0x2748774C, 0xDF8EEB99),
    Word64(0x34B0BCB5, 0xE19B48A8),
    Word64(0x391C0CB3, 0xC5C95A63),
    Word64(0x4ED8AA4A, 0xE3418ACB),
    Word64(0x5B9CCA4F, 0x7763E373),
    Word64(0x682E6FF3, 0xD6B2B8A3),
    Word64(0x748F82EE, 0x5DEFB2FC),
    Word64(0x78A5636F, 0x43172F60),
    Word64(0x84C87814, 0xA1F0AB72),
    Word64(0x8CC70208, 0x1A6439EC),
    Word64(0x90BEFFFA, 0x23631E28),
    Word64(0xA4506CEB, 0xDE82BDE9),
    Word64(0xBEF9A3F7, 0xB2C67915),
    Word64(0xC67178F2, 0xE372532B),
    Word64(0xCA273ECE, 0xEA26619C),
    Word64(0xD186B8C7, 0x21C0C207),
    Word64(0xEADA7DD6, 0xCDE0EB1E),
    Word64(0xF57D4F7F, 0xEE6ED178),
    Word64(0x06F067AA, 0x72176FBA),
    Word64(0x0A637DC5, 0xA2C898A6),
    Word64(0x113F9804, 0xBEF90DAE),
    Word64(0x1B710B35, 0x131C471B),
    Word64(0x28DB77F5, 0x23047D84),
    Word64(0x32CAAB7B, 0x40C72493),
    Word64(0x3C9EBE0A, 0x15C9BEBC),
    Word64(0x431D67C4, 0x9C100D4C),
    Word64(0x4CC5D4BE, 0xCB3E42B6),
    Word64(0x597F299C, 0xFC657E2A),
    Word64(0x5FCB6FAB, 0x3AD6FAEC),
    Word64(0x6C44198C, 0x4A475817),
  ];
}
