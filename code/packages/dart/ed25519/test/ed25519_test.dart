// Unit tests for the Ed25519 lane: every reference test mirrored
// case-for-case (field arithmetic, point operations, encoding, the
// four RFC 8032 vectors, verification edges, key generation), plus
// lane extras (length validation, value semantics, determinism).

import 'dart:typed_data';

import 'package:coding_adventures_ed25519/ed25519.dart';
import 'package:coding_adventures_ed25519/src/constants.dart';
import 'package:coding_adventures_ed25519/src/curve.dart';
import 'package:test/test.dart';

Uint8List _unhex(String s) => Uint8List.fromList([
      for (var i = 0; i < s.length; i += 2)
        int.parse(s.substring(i, i + 2), radix: 16),
    ]);

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  group('field', () {
    test('inverse of three', () {
      final three = Fe(BigInt.from(3));
      final product = three * three.invert();
      expect(product, equals(Fe.one));
    });

    test('inverse of one', () {
      expect(Fe.one.invert(), equals(Fe.one));
      expect(Fe.one.hashCode, equals(Fe(BigInt.one).hashCode));
    });

    test('sqrt(-1) squared is -1', () {
      const sm1 = [
        0xb0,
        0xa0,
        0x0e,
        0x4a,
        0x27,
        0x1b,
        0xee,
        0xc4,
        0x78,
        0xe4,
        0x2f,
        0xad,
        0x06,
        0x18,
        0x43,
        0x2f,
        0xa7,
        0xd7,
        0xfb,
        0x3d,
        0x99,
        0x00,
        0x4d,
        0x2b,
        0x0b,
        0xdf,
        0xc1,
        0x4f,
        0x80,
        0x24,
        0x83,
        0x2b,
      ];
      final root = Fe.fromBytes(sm1);
      expect(root.square(), equals(-Fe.one));
    });

    test('sqrt of a perfect square round-trips', () {
      final val = Fe(BigInt.from(42));
      final sq = val.square();
      final root = sq.sqrt();
      expect(root, isNotNull);
      expect(root!.square(), equals(sq));
    });

    test('two has no square root', () {
      expect(Fe(BigInt.two).sqrt(), isNull);
    });
  });

  group('points', () {
    test('identity addition', () {
      final bp = ExtPoint.base();
      expect(
        _hex(ExtPoint.identity().add(bp).encode().toList()),
        equals(_hex(bp.encode().toList())),
      );
    });

    test('double equals add', () {
      final bp = ExtPoint.base();
      expect(
        _hex(bp.double().encode().toList()),
        equals(_hex(bp.add(bp).encode().toList())),
      );
    });

    test('scalar mult by zero is identity', () {
      final bp = ExtPoint.base();
      final result = bp.scalarMul(List<int>.filled(32, 0));
      expect(
        _hex(result.encode().toList()),
        equals(_hex(ExtPoint.identity().encode().toList())),
      );
    });

    test('scalar mult by one is the point', () {
      final bp = ExtPoint.base();
      final one = List<int>.filled(32, 0)..[0] = 1;
      expect(
        _hex(bp.scalarMul(one).encode().toList()),
        equals(_hex(bp.encode().toList())),
      );
    });

    test('scalar mult by two is doubling', () {
      final bp = ExtPoint.base();
      final two = List<int>.filled(32, 0)..[0] = 2;
      expect(
        _hex(bp.scalarMul(two).encode().toList()),
        equals(_hex(bp.add(bp).encode().toList())),
      );
    });

    test('scalar mult by group order is identity', () {
      final bp = ExtPoint.base();
      final result = bp.scalarMul(ed25519LBytes);
      expect(
        _hex(result.encode().toList()),
        equals(_hex(ExtPoint.identity().encode().toList())),
      );
    });

    test('base point lies on the curve', () {
      final bx = Fe.fromBytes(ed25519BxBytes);
      final by = Fe.fromBytes(ed25519ByBytes);
      final dFe = Fe.fromBytes(ed25519DBytes);
      final xSq = bx.square();
      final ySq = by.square();
      expect(ySq - xSq, equals(Fe.one + dFe * xSq * ySq));
    });

    test('encode/decode round-trips base, identity, double', () {
      final bp = ExtPoint.base();
      for (final point in [
        bp,
        ExtPoint.identity(),
        bp.double(),
      ]) {
        final encoded = point.encode().toList();
        final decoded = ExtPoint.decode(encoded)!;
        expect(_hex(decoded.encode().toList()), equals(_hex(encoded)));
      }
    });
  });

  group('rfc8032', () {
    test('vector 1: empty message', () {
      final seed = _unhex(
        '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60',
      );
      final (pub, sec) = generateKeypair(seed);
      expect(
        _hex(pub.bytes),
        equals(
          'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
        ),
      );
      final sig = sign([], sec);
      expect(
        _hex(sig.bytes),
        equals(
          'e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155'
          '5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b',
        ),
      );
      expect(verify([], sig, pub), isTrue);
    });

    test('vector 2: one byte', () {
      final seed = _unhex(
        '4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb',
      );
      final (pub, sec) = generateKeypair(seed);
      expect(
        _hex(pub.bytes),
        equals(
          '3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c',
        ),
      );
      final sig = sign(_unhex('72'), sec);
      expect(
        _hex(sig.bytes),
        equals(
          '92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da'
          '085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00',
        ),
      );
      expect(verify(_unhex('72'), sig, pub), isTrue);
    });

    test('vector 3: two bytes', () {
      final seed = _unhex(
        'c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7',
      );
      final (pub, sec) = generateKeypair(seed);
      expect(
        _hex(pub.bytes),
        equals(
          'fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025',
        ),
      );
      final sig = sign(_unhex('af82'), sec);
      expect(
        _hex(sig.bytes),
        equals(
          '6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3a'
          'c18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a',
        ),
      );
      expect(verify(_unhex('af82'), sig, pub), isTrue);
    });

    test('vector 4: 1023 bytes', () {
      final seed = _unhex(
        'f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5',
      );
      final message = _unhex(
        '08b8b2b733424243760fe426a4b54908632110a66c2f6591eabd3345e3e4eb98'
        'fa6e264bf09efe12ee50f8f54e9f77b1e355f6c50544e23fb1433ddf73be84d8'
        '79de7c0046dc4996d9e773f4bc9efe5738829adb26c81b37c93a1b270b20329d'
        '658675fc6ea534e0810a4432826bf58c941efb65d57a338bbd2e26640f89ffbc'
        '1a858efcb8550ee3a5e1998bd177e93a7363c344fe6b199ee5d02e82d522c4fe'
        'ba15452f80288a821a579116ec6dad2b3b310da903401aa62100ab5d1a36553e'
        '06203b33890cc9b832f79ef80560ccb9a39ce767967ed628c6ad573cb116dbef'
        'fefd75499da96bd68a8a97b928a8bbc103b6621fcde2beca1231d206be6cd9ec'
        '7aff6f6c94fcd7204ed3455c68c83f4a41da4af2b74ef5c53f1d8ac70bdcb7ed'
        '185ce81bd84359d44254d95629e9855a94a7c1958d1f8ada5d0532ed8a5aa3fb'
        '2d17ba70eb6248e594e1a2297acbbb39d502f1a8c6eb6f1ce22b3de1a1f40cc2'
        '4554119a831a9aad6079cad88425de6bde1a9187ebb6092cf67bf2b13fd65f27'
        '088d78b7e883c8759d2c4f5c65adb7553878ad575f9fad878e80a0c9ba63bcbc'
        'c2732e69485bbc9c90bfbd62481d9089beccf80cfe2df16a2cf65bd92dd597b0'
        '7e0917af48bbb75fed413d238f5555a7a569d80c3414a8d0859dc65a46128bab'
        '27af87a71314f318c782b23ebfe808b82b0ce26401d2e22f04d83d1255dc51ad'
        'dd3b75a2b1ae0784504df543af8969be3ea7082ff7fc9888c144da2af58429ec'
        '96031dbcad3dad9af0dcbaaaf268cb8fcffead94f3c7ca495e056a9b47acdb75'
        '1fb73e666c6c655ade8297297d07ad1ba5e43f1bca32301651339e22904cc8c4'
        '2f58c30c04aafdb038dda0847dd988dcda6f3bfd15c4b4c4525004aa06eeff8c'
        'a61783aacec57fb3d1f92b0fe2fd1a85f6724517b65e614ad6808d6f6ee34dff'
        '7310fdc82aebfd904b01e1dc54b2927094b2db68d6f903b68401adebf5a7e08d'
        '78ff4ef5d63653a65040cf9bfd4aca7984a74d37145986780fc0b16ac451649d'
        'e6188a7dbdf191f64b5fc5e2ab47b57f7f7276cd419c17a3ca8e1b939ae49e48'
        '8acba6b965610b5480109c8b17b80e1b7b750dfc7598d5d5011fd2dcc5600a32'
        'ef5b52a1ecc820e308aa342721aac0943bf6686b64b2579376504ccc493d97e6'
        'aed3fb0f9cd71a43dd497f01f17c0e2cb3797aa2a2f256656168e6c496afc5fb'
        '93246f6b1116398a346f1a641f3b041e989f7914f90cc2c7fff357876e506b50'
        'd334ba77c225bc307ba537152f3f1610e4eafe595f6d9d90d11faa933a15ef13'
        '69546868a7f3a45a96768d40fd9d03412c091c6315cf4fde7cb68606937380db'
        '2eaaa707b4c4185c32eddcdd306705e4dc1ffc872eeee475a64dfac86aba41c0'
        '618983f8741c5ef68d3a101e8a3b8cac60c905c15fc910840b94c00a0b9d00',
      );
      expect(message.length, equals(1023));
      final (pub, sec) = generateKeypair(seed);
      expect(
        _hex(pub.bytes),
        equals(
          '278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e',
        ),
      );
      final sig = sign(message, sec);
      expect(
        _hex(sig.bytes),
        equals(
          'd686294b743c6760c6a78a2c4c2fc76115c2600b8f083acde59e7cee32578c0f'
          '59ea4219ab9b5896795e4e2b87a30270aa0e3099eee944e9e67a1b22df41ff07',
        ),
      );
      expect(verify(message, sig, pub), isTrue);
    });
  });

  group('verification edges', () {
    final seed = _unhex(
      '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60',
    );

    test('wrong message fails', () {
      final (pub, sec) = generateKeypair(seed);
      final sig = sign('hello'.codeUnits, sec);
      expect(verify('world'.codeUnits, sig, pub), isFalse);
    });

    test('wrong public key fails', () {
      final seed2 = _unhex(
        '4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb',
      );
      final (_, sec1) = generateKeypair(seed);
      final (pub2, _) = generateKeypair(seed2);
      final sig = sign('hello'.codeUnits, sec1);
      expect(verify('hello'.codeUnits, sig, pub2), isFalse);
    });

    test('tampered R fails', () {
      final (pub, sec) = generateKeypair(seed);
      final bytes = Uint8List.fromList(sign('hello'.codeUnits, sec).bytes);
      bytes[0] ^= 1;
      expect(
        verify('hello'.codeUnits, Ed25519Signature(bytes), pub),
        isFalse,
      );
    });

    test('tampered S fails', () {
      final (pub, sec) = generateKeypair(seed);
      final bytes = Uint8List.fromList(sign('hello'.codeUnits, sec).bytes);
      bytes[32] ^= 1;
      expect(
        verify('hello'.codeUnits, Ed25519Signature(bytes), pub),
        isFalse,
      );
    });

    test('S out of range fails', () {
      final (pub, sec) = generateKeypair(seed);
      final bytes = Uint8List.fromList(sign('hello'.codeUnits, sec).bytes);
      bytes.setRange(32, 64, ed25519LBytes);
      expect(
        verify('hello'.codeUnits, Ed25519Signature(bytes), pub),
        isFalse,
      );
    });

    test('identity key cannot verify an identity signature', () {
      final identity = ExtPoint.identity().encode().toList();
      final forged = Uint8List(64)..setRange(0, 32, identity);
      expect(isValidPublicKey(Uint8List.fromList(identity)), isFalse);
      expect(
        verify(
          'arbitrary package digest'.codeUnits,
          Ed25519Signature(forged),
          Ed25519PublicKey(Uint8List.fromList(identity)),
        ),
        isFalse,
      );
    });

    test('torsion and mixed-order keys are rejected', () {
      final encoded = List<int>.filled(32, 0xff);
      encoded[0] = 0xec;
      encoded[31] = 0x7f;
      final orderTwo = ExtPoint.decode(encoded)!;
      expect(
        _hex(orderTwo.double().encode().toList()),
        equals(_hex(ExtPoint.identity().encode().toList())),
      );
      expect(
        _hex(orderTwo.encode().toList()),
        isNot(equals(_hex(ExtPoint.identity().encode().toList()))),
      );

      final (valid, _) =
          generateKeypair(Uint8List.fromList(List<int>.filled(32, 42)));
      final validPoint = ExtPoint.decode(valid.bytes)!;
      final mixedOrder = validPoint.add(orderTwo).encode().toList();

      expect(isValidPublicKey(valid.bytes), isTrue);
      expect(
        isValidPublicKey(
          Uint8List.fromList(orderTwo.encode().toList()),
        ),
        isFalse,
      );
      expect(
        isValidPublicKey(Uint8List.fromList(mixedOrder)),
        isFalse,
      );
    });
  });

  group('keys', () {
    test('generation is deterministic', () {
      final seed = Uint8List.fromList([for (var i = 0; i < 32; i++) i]);
      final (pub1, sec1) = generateKeypair(seed);
      final (pub2, sec2) = generateKeypair(seed);
      expect(pub1, equals(pub2));
      expect(sec1, equals(sec2));
    });

    test('signing is deterministic', () {
      final seed = Uint8List.fromList([for (var i = 0; i < 32; i++) i]);
      final (_, sec) = generateKeypair(seed);
      expect(
        sign('hello'.codeUnits, sec),
        equals(sign('hello'.codeUnits, sec)),
      );
    });

    test('secret key format is seed then public key', () {
      final seed = Uint8List.fromList([for (var i = 0; i < 32; i++) i]);
      final (pub, sec) = generateKeypair(seed);
      expect(sec.bytes.sublist(0, 32), equals(seed));
      expect(sec.bytes.sublist(32, 64), equals(pub.bytes));
    });

    // --- Lane extras ---

    test('length validation rejects misshapen inputs', () {
      expect(() => generateKeypair(Uint8List(31)), throwsArgumentError);
      expect(() => generateKeypair(Uint8List(33)), throwsArgumentError);
      expect(() => Ed25519PublicKey(Uint8List(31)), throwsArgumentError);
      expect(() => Ed25519SecretKey(Uint8List(63)), throwsArgumentError);
      expect(() => Ed25519Signature(Uint8List(65)), throwsArgumentError);
      expect(isValidPublicKey(Uint8List(31)), isFalse);
    });

    test('value semantics hold without leaking bytes', () {
      final seed = Uint8List.fromList(List<int>.filled(32, 7));
      final (pub, sec) = generateKeypair(seed);
      final sig = sign([1, 2, 3], sec);
      expect(pub, equals(Ed25519PublicKey(Uint8List.fromList(pub.bytes))));
      expect(pub.hashCode, equals(Ed25519PublicKey(pub.bytes).hashCode));
      expect(sig, equals(Ed25519Signature(Uint8List.fromList(sig.bytes))));
      expect(sig.hashCode, equals(Ed25519Signature(sig.bytes).hashCode));
      expect(sec, equals(Ed25519SecretKey(Uint8List.fromList(sec.bytes))));
      expect(sec.hashCode, equals(Ed25519SecretKey(sec.bytes).hashCode));
      expect(pub.toString(), equals('Ed25519PublicKey(32 bytes)'));
      expect(pub.toString(), isNot(contains(_hex(pub.bytes))));
    });
  });
}
