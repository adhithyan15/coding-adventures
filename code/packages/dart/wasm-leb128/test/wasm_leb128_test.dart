import 'package:coding_adventures_wasm_leb128/wasm_leb128.dart';
import 'package:test/test.dart';

void main() {
  test('encodes and decodes unsigned values', () {
    final encoded = encodeUnsigned(624485);
    expect(encoded, [0xe5, 0x8e, 0x26]);

    final decoded = decodeUnsigned(encoded);
    expect(decoded.value, 624485);
    expect(decoded.bytesRead, 3);
  });

  test('encodes and decodes signed values', () {
    final encoded = encodeSigned(-2);
    expect(encoded, [0x7e]);

    final decoded = decodeSigned(encoded, bitWidth: 32);
    expect(decoded.value, -2);
    expect(decoded.bytesRead, 1);
  });

  test('throws on unterminated encoding', () {
    expect(
      () => decodeUnsigned([0x80, 0x80]),
      throwsA(isA<Leb128Exception>()),
    );
  });
}
