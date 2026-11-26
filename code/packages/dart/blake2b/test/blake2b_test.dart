// Unit tests for the BLAKE2b lane: all 23 reference tests mirrored
// case-for-case (canonical hashlib vectors, block boundaries, digest
// sizes, keyed variants, salt/personal, streaming, validation, hex),
// plus lane extras (builder defaults, option immutability, error value
// semantics).

import 'dart:typed_data';

import 'package:coding_adventures_blake2b/blake2b.dart';
import 'package:test/test.dart';

Uint8List _range(int start, int end) =>
    Uint8List.fromList([for (var i = start; i < end; i++) i & 0xff]);

String _oneShotHex(List<int> data) => blake2bHex(data);

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  group('blake2b', () {
    // --- Canonical vectors (Python hashlib.blake2b oracle) ---

    test('empty default', () {
      expect(
        _oneShotHex([]),
        equals(
          '786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce',
        ),
      );
    });

    test('abc', () {
      expect(
        _oneShotHex('abc'.codeUnits),
        equals(
          'ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923',
        ),
      );
    });

    test('fox', () {
      expect(
        _oneShotHex('The quick brown fox jumps over the lazy dog'.codeUnits),
        equals(
          'a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918',
        ),
      );
    });

    test('truncated digest size 32', () {
      final got = blake2bHex([], const Blake2bOptions(digestSize: 32));
      expect(
        got,
        equals(
          '0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8',
        ),
      );
    });

    test('keyed long vector', () {
      final key = _range(1, 65);
      final data = _range(0, 256);
      final got = blake2bHex(data, Blake2bOptions(key: key));
      expect(
        got,
        equals(
          '402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3',
        ),
      );
    });

    // --- Block-boundary sizes ---

    test('block boundary sizes', () {
      const cases = {
        0: '786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce',
        1: '4fe4da61bcc756071b226843361d74944c72245d23e8245ea678c13fdcd7fe2ae529cf999ad99cc24f7a73416a18ba53e76c0afef83b16a568b12fbfc1a2674d',
        63: '70b2a0e6daecac22c7a2df82c06e3fc0b4c66bd5ef8098e4ed54e723b393d79ef3bceba079a01a14c6ef2ae2ed1171df1662cd14ef38e6f77b01c7f48144dd09',
        64: '3db7bb5c40745f0c975ac6bb8578f590e2cd2cc1fc6d13533ef725325c9fddff5cca24e7a591a0f6032a24fad0e09f6df873c4ff314628391f78df7f09cb7ed7',
        65: '149c114a3e8c6e06bafee27c9d0de0e39ef28294fa0d9f81876dcceb10bb41101e256593587e46b844819ed7ded90d56c0843df06c95d1695c3de635cd7a888e',
        127:
            '71546bbf9110ad184cc60f2eb120fcfd9b4dbbca7a7f1270045b8a23a6a4f4330f65c1f030dd2f5fabc6c57617242c37cf427bd90407fac5b9deffd3ae888c39',
        128:
            '2d9e329f42afa3601d646692b81c13e87fcaff5bf15972e9813d7373cb6d181f9599f4d513d4af4fd6ebd37497aceb29aba5ee23ed764d8510b552bd088814fb',
        129:
            '47889df9eb4d717afc5019df5c6a83df00a0b8677395e078cd5778ace0f338a618e68b7d9afb065d9e6a01ccd31d109447e7fae771c3ee3e105709194122ba2b',
        255:
            '1a5199ac66a00e8a87ad1c7fbad30b33137dd8312bf6d98602dacf8f40ea2cb623a7fbc63e5a6bfa434d337ae7da5ca1a52502a215a3fe0297a151be85d88789',
        256:
            '91019c558584980249ca43eceed27e19f1c3c24161b93eed1eee2a6a774f60bf8a81b43750870bee1698feac9c5336ae4d5c842e7ead159bf3916387e8ded9ae',
        257:
            '9f1975efca45e7b74b020975d4d2c22802906ed8bfefca51ac497bd23147fc8f303890d8e5471ab6caaa02362e831a9e8d3435279912ccd4842c7806b096c348',
        1024:
            'eddc3f3af9392eff065b359ce5f2b28f71e9f3a3a50e60ec27787b9fa623094d17b046c1dfce89bc5cdfc951b95a9a9c05fb8cc2361c905db01dd237fe56efb3',
        4096:
            '31404c9c7ed64c59112579f300f2afef181ee6283c3918bf026c4ed4bcde0697a7834f3a3410396622ef3d4f432602528a689498141c184cc2063554ba688dc7',
        9999:
            'b4a5808e65d7424b517bde11e04075a09b1343148e3ab2c8b13ff35c542e0a2beff6309ecc54b59ac046f6d65a9e3680c6372a033607709c95d5fd8070be6069',
      };
      for (final entry in cases.entries) {
        final data = Uint8List.fromList(
            [for (var i = 0; i < entry.key; i++) (i * 7 + 3) & 0xff]);
        expect(_oneShotHex(data), equals(entry.value),
            reason: 'size ${entry.key}');
      }
    });

    // --- Variable digest sizes ---

    test('variable digest sizes', () {
      final data = 'The quick brown fox jumps over the lazy dog'.codeUnits;
      const cases = {
        1: 'b5',
        16: '249df9a49f517ddcd37f5c897620ec73',
        20: '3c523ed102ab45a37d54f5610d5a983162fde84f',
        32: '01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9',
        48: 'b7c81b228b6bd912930e8f0b5387989691c1cee1e65aade4da3b86a3c9f678fc8018f6ed9e2906720c8d2a3aeda9c03d',
        64: 'a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918',
      };
      for (final entry in cases.entries) {
        final out = blake2b(data, Blake2bOptions(digestSize: entry.key));
        expect(out.length, equals(entry.key));
        expect(_hex(out), equals(entry.value),
            reason: 'digestSize ${entry.key}');
      }
    });

    // --- Keyed across multiple key lengths ---

    test('keyed variants', () {
      final data = 'secret message body'.codeUnits;
      const cases = {
        1: 'affd4e429aa2fb18da276f6ecff16f7d048769cacefe1a7ac75184448e082422',
        16: '5f8510d05dac42e8b6fc542af93f349d41ae4ebaf5cecae4af43fae54c7ca618',
        32: '88a78036d5890e91b5e3d70ba4738d2be302b76e0857d8ee029dc56dfa04fe67',
        64: 'df7eab2ec9135ab8c58f48c288cdc873bac245a7fa46ca9f047cab672bd1eabb',
      };
      for (final entry in cases.entries) {
        final key = _range(1, entry.key + 1);
        final got = blake2bHex(
          data,
          Blake2bOptions(key: key, digestSize: 32),
        );
        expect(got, equals(entry.value), reason: 'keyLen ${entry.key}');
      }
    });

    test('salt and personal', () {
      final salt = _range(0, 16);
      final personal = _range(16, 32);
      final got = blake2bHex(
        'parameterized hash'.codeUnits,
        Blake2bOptions(salt: salt, personal: personal),
      );
      expect(
        got,
        equals(
          'a2185d648fc63f3d363871a76360330c9b238af5466a20f94bb64d363289b95da0453438eea300cd6f31521274ec001011fa29e91a603fabf00f2b454e30bf3d',
        ),
      );
    });

    // --- Streaming ---

    test('streaming single chunk matches one-shot', () {
      final h = Blake2bHasher();
      h.update('hello world'.codeUnits);
      expect(_hex(h.digest()), equals(_oneShotHex('hello world'.codeUnits)));
    });

    test('streaming byte by byte matches one-shot', () {
      final data = _range(0, 200);
      final h = Blake2bHasher(const Blake2bOptions(digestSize: 32));
      for (final b in data) {
        h.update([b]);
      }
      final want = blake2b(data, const Blake2bOptions(digestSize: 32));
      expect(_hex(h.digest()), equals(_hex(want)));
    });

    test('streaming chunks across block boundary', () {
      final data = _range(0, 129);
      final h = Blake2bHasher();
      h.update(data.sublist(0, 127));
      h.update(data.sublist(127));
      expect(_hex(h.digest()), equals(_oneShotHex(data)));
    });

    test('streaming exact block then more (off-by-one)', () {
      // 128 bytes exact, then 4 more. The 128-byte block must NOT be
      // flagged final while more data is still coming.
      final data = Uint8List.fromList([for (var i = 0; i < 132; i++) i & 0xff]);
      final h = Blake2bHasher();
      h.update(data.sublist(0, 128));
      h.update(data.sublist(128));
      expect(_hex(h.digest()), equals(_oneShotHex(data)));
    });

    test('digest is idempotent', () {
      final h = Blake2bHasher();
      h.update('hello'.codeUnits);
      expect(h.hexDigest(), equals(h.hexDigest()));
    });

    test('update after digest continues the stream', () {
      final h = Blake2bHasher(const Blake2bOptions(digestSize: 32));
      h.update('hello '.codeUnits);
      h.digest();
      h.update('world'.codeUnits);
      final want = blake2bHex(
        'hello world'.codeUnits,
        const Blake2bOptions(digestSize: 32),
      );
      expect(h.hexDigest(), equals(want));
    });

    test('copy is independent', () {
      final h = Blake2bHasher();
      h.update('prefix '.codeUnits);
      final clone = h.copy();
      h.update('path A'.codeUnits);
      clone.update('path B'.codeUnits);
      expect(_hex(h.digest()), equals(_oneShotHex('prefix path A'.codeUnits)));
      expect(
          _hex(clone.digest()), equals(_oneShotHex('prefix path B'.codeUnits)));
    });

    // --- Validation ---

    test('rejects digest size zero', () {
      expect(
        () => Blake2bHasher(const Blake2bOptions(digestSize: 0)),
        throwsA(equals(const InvalidDigestSize(0))),
      );
    });

    test('rejects digest size 65', () {
      expect(
        () => Blake2bHasher(const Blake2bOptions(digestSize: 65)),
        throwsA(equals(const InvalidDigestSize(65))),
      );
    });

    test('rejects key too long', () {
      final key = Uint8List(65);
      expect(
        () => Blake2bHasher(Blake2bOptions(key: key)),
        throwsA(equals(const KeyTooLong(65))),
      );
    });

    test('rejects wrong salt length', () {
      final salt = Uint8List(8);
      expect(
        () => Blake2bHasher(Blake2bOptions(salt: salt)),
        throwsA(equals(const InvalidSaltLength(8))),
      );
    });

    test('rejects wrong personal length', () {
      final personal = Uint8List(20);
      expect(
        () => Blake2bHasher(Blake2bOptions(personal: personal)),
        throwsA(equals(const InvalidPersonalLength(20))),
      );
    });

    test('accepts max 64-byte key', () {
      final key = Uint8List.fromList(List<int>.filled(64, 0x41));
      expect(blake2b('x'.codeUnits, Blake2bOptions(key: key)), hasLength(64));
    });

    test('error display texts are non-empty and exact', () {
      const errors = [
        InvalidDigestSize(0),
        KeyTooLong(65),
        InvalidSaltLength(8),
        InvalidPersonalLength(20),
      ];
      for (final e in errors) {
        expect(e.toString(), isNotEmpty);
      }
      expect(
        const InvalidDigestSize(0).toString(),
        equals('digest_size must be in [1, 64], got 0'),
      );
      expect(
        const KeyTooLong(65).toString(),
        equals('key length must be in [0, 64], got 65'),
      );
    });

    test('hex digest matches hex of digest', () {
      final h = Blake2bHasher(const Blake2bOptions(digestSize: 32));
      h.update('hex check'.codeUnits);
      expect(h.hexDigest(), equals(_hex(h.digest())));
    });

    // --- Lane extras ---

    test('default options hash unkeyed full digests', () {
      final a = Blake2bHasher();
      final b = Blake2bHasher(const Blake2bOptions());
      a.update([1, 2, 3]);
      b.update([1, 2, 3]);
      expect(_hex(a.digest()), equals(_hex(b.digest())));
      expect(a.digest().length, equals(64));
    });

    test('update returns the hasher for chaining', () {
      final h = Blake2bHasher();
      final same = h.update([1]).update([2, 3]);
      expect(identical(same, h), isTrue);
      expect(_hex(h.digest()), equals(_oneShotHex([1, 2, 3])));
    });

    test('one-shot helpers accept omitted options', () {
      expect(blake2b([9]).length, equals(64));
      expect(blake2bHex([9]).length, equals(128));
    });

    test('error values compare by type and payload', () {
      expect(
        const InvalidDigestSize(0),
        isNot(equals(const KeyTooLong(0))),
      );
      expect(
        const InvalidDigestSize(0).hashCode,
        equals(const InvalidDigestSize(0).hashCode),
      );
      expect(
        const KeyTooLong(65).hashCode,
        equals(const KeyTooLong(65).hashCode),
      );
      expect(
        const InvalidSaltLength(8).hashCode,
        equals(const InvalidSaltLength(8).hashCode),
      );
      expect(
        const InvalidPersonalLength(20).hashCode,
        equals(const InvalidPersonalLength(20).hashCode),
      );
    });
  });
}
