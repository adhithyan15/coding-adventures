import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_der_asn1/der_asn1.dart';
import 'package:coding_adventures_der_tlv/der_tlv.dart';
import 'package:test/test.dart';

Map<String, dynamic> loadFixture(String name) {
  final candidates = [
    '../../../../specs/fixtures/$name/cases.json',
    '../../../specs/fixtures/$name/cases.json',
  ];
  for (final candidate in candidates) {
    final file = File(candidate);
    if (file.existsSync()) {
      return jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
    }
  }
  throw StateError('unable to locate $name fixture');
}

Uint8List materialize(Object? rawSegments) {
  final builder = BytesBuilder(copy: false);
  for (final raw in rawSegments! as List<dynamic>) {
    final segment = raw as Map<String, dynamic>;
    if (segment['hex'] case final String value) {
      builder.add([
        for (var index = 0; index < value.length; index += 2)
          int.parse(value.substring(index, index + 2), radix: 16),
      ]);
    } else {
      builder.add(
        Uint8List(segment['count'] as int)
          ..fillRange(
            0,
            segment['count'] as int,
            int.parse(segment['repeat_hex'] as String, radix: 16),
          ),
      );
    }
  }
  return builder.takeBytes();
}

Object? merged(
  Map<String, dynamic> defaults,
  Map<String, dynamic> overrides,
  String name,
) =>
    overrides.containsKey(name) ? overrides[name] : defaults[name];

DerLimits fixtureDerLimits(
  Map<String, dynamic> defaults,
  Map<String, dynamic> overrides,
) {
  final maxValue = merged(defaults, overrides, 'max_value_len');
  return DerLimits(
    maxInputLen: merged(defaults, overrides, 'max_input_len') as int,
    maxValueLen: maxValue == 'host-max'
        ? BigInt.parse('9223372036854775807')
        : BigInt.from(maxValue! as int),
    maxElements: merged(defaults, overrides, 'max_elements') as int,
    maxTagNumber: merged(defaults, overrides, 'max_tag_number') as int,
  );
}

Asn1Limits fixtureLimits(
  Map<String, dynamic> document,
  Map<String, dynamic> testCase,
) {
  final defaults = document['defaults'] as Map<String, dynamic>;
  final overrides =
      testCase['limits'] as Map<String, dynamic>? ?? <String, dynamic>{};
  final derOverrides =
      overrides['der'] as Map<String, dynamic>? ?? <String, dynamic>{};
  return Asn1Limits(
    der: fixtureDerLimits(
      defaults['der'] as Map<String, dynamic>,
      derOverrides,
    ),
    maxDepth: merged(defaults, overrides, 'max_depth') as int,
    maxTotalElements: merged(defaults, overrides, 'max_total_elements') as int,
    maxOidArcs: merged(defaults, overrides, 'max_oid_arcs') as int,
  );
}

String hex(Uint8List bytes) =>
    bytes.map((value) => value.toRadixString(16).padLeft(2, '0')).join();

Map<String, dynamic> tagProjection(Asn1Element element) => {
      'class': element.tag.tagClass.wireName,
      'constructed': element.tag.constructed,
      'number': element.tag.number,
    };

Map<String, dynamic> errorProjection(
  Asn1Exception error,
  String scope,
) {
  final result = <String, dynamic>{
    'outcome': 'error',
    'error_id': error.kind.id,
    'offset': error.offset,
    'offset_scope': scope,
  };
  if (error.framingKind case final DerErrorKind framingKind) {
    result['framing_error_id'] = framingKind.id;
  }
  return result;
}

Map<String, dynamic> verifyUpstream(
  Map<String, dynamic> upstream,
  Map<String, dynamic> testCase,
) {
  final id = testCase['der_tlv_case_id'] as String;
  final referenced = (upstream['cases'] as List<dynamic>)
      .cast<Map<String, dynamic>>()
      .singleWhere((candidate) => candidate['id'] == id);
  final overrides =
      referenced['limits'] as Map<String, dynamic>? ?? <String, dynamic>{};
  final limits = Asn1Limits(
    der: fixtureDerLimits(
      upstream['defaults'] as Map<String, dynamic>,
      overrides,
    ),
  );
  final decoder = Asn1Decoder(limits);
  final expected = referenced['expected'] as Map<String, dynamic>;
  try {
    final element = decoder.decodeExact(materialize(referenced['input']));
    expect(expected['outcome'], 'element', reason: id);
    expect(tagProjection(element), expected['tag'], reason: id);
    expect(element.header.length, expected['header_len'], reason: id);
    expect(element.encoded.length, expected['encoded_len'], reason: id);
  } on Asn1Exception catch (error) {
    final projected = errorProjection(error, 'operation-input');
    expect(expected['outcome'], 'error', reason: id);
    expect(projected['framing_error_id'], expected['error_id'], reason: id);
    expect(projected['offset'], expected['offset'], reason: id);
  }
  return {'outcome': 'upstream'};
}

Map<String, dynamic> primitiveResult(
  String operation,
  Asn1Element element,
  Asn1Limits limits,
  int tagNumber,
) {
  switch (operation) {
    case 'decode-boolean':
      return {
        'outcome': 'value',
        'boolean': decodeBoolean(element),
        'elements_read': 1,
      };
    case 'decode-integer':
    case 'integer-to-u64':
      final value = decodeInteger(element);
      final result = <String, dynamic>{
        'outcome': 'value',
        'signed_hex': hex(value.signedBytes),
        'negative': value.isNegative,
      };
      if (operation == 'integer-to-u64') {
        result['u64_decimal'] = value.toU64().toString();
      }
      return result;
    case 'decode-bit-string':
      final value = decodeBitString(element);
      return {
        'outcome': 'value',
        'bytes_hex': hex(value.bytes),
        'unused_bits': value.unusedBits,
        'bit_length': value.bitLength,
      };
    case 'decode-octet-string':
      return {
        'outcome': 'value',
        'bytes_hex': hex(decodeOctetString(element)),
      };
    case 'decode-implicit-octet-string':
      return {
        'outcome': 'value',
        'bytes_hex': hex(decodeImplicitOctetString(element, tagNumber)),
      };
    case 'decode-ia5-string':
      return {'outcome': 'value', 'text': decodeIA5String(element)};
    case 'decode-implicit-ia5-string':
      return {
        'outcome': 'value',
        'text': decodeImplicitIA5String(element, tagNumber),
      };
    case 'decode-null':
      decodeNull(element);
      return {'outcome': 'value'};
    case 'decode-object-identifier':
    case 'decode-implicit-object-identifier':
      final value = operation == 'decode-object-identifier'
          ? decodeObjectIdentifier(element, limits)
          : decodeImplicitObjectIdentifier(element, tagNumber, limits);
      return {
        'outcome': 'value',
        'bytes_hex': hex(value.encoded),
        'arcs_decimal': value.arcs.map((arc) => arc.toString()).toList(),
        'arc_count': value.arcCount,
      };
  }
  throw StateError('unsupported operation $operation');
}

Map<String, dynamic> cursorResult(
  Map<String, dynamic> testCase,
  Asn1Decoder decoder,
  Asn1Element root,
) {
  final cursor = decoder.sequence(root);
  final total = cursor.remaining.length;
  final events = <Map<String, dynamic>>[];
  for (final action in testCase['actions'] as List<dynamic>) {
    if (action == 'finish') {
      try {
        cursor.finish();
        events.add({'outcome': 'finished'});
      } on Asn1Exception catch (error) {
        events.add(errorProjection(error, 'container-value'));
      }
      continue;
    }
    if (action != 'read' && action != 'read-with-different-limits') {
      throw StateError('unsupported cursor action $action');
    }
    final active = action == 'read-with-different-limits'
        ? Asn1Decoder(
            decoder.limits.copyWith(
              maxTotalElements: decoder.limits.maxTotalElements + 1,
            ),
          )
        : decoder;
    try {
      final child = cursor.read(active);
      events.add(
        child == null
            ? {'outcome': 'end'}
            : {
                'outcome': 'value',
                'tag': tagProjection(child),
                'depth': child.depth,
              },
      );
    } on Asn1Exception catch (error) {
      events.add(errorProjection(error, 'container-value'));
    }
  }
  return {
    'outcome': 'value',
    'elements_read': decoder.elementsRead,
    'remaining_offset': total - cursor.remaining.length,
    'events': events,
  };
}

Map<String, dynamic> runCase(
  Map<String, dynamic> document,
  Map<String, dynamic> upstream,
  Map<String, dynamic> testCase,
) {
  if (testCase.containsKey('der_tlv_case_id')) {
    return verifyUpstream(upstream, testCase);
  }
  final limits = fixtureLimits(document, testCase);
  final decoder = Asn1Decoder(limits);
  final operation = testCase['operation'] as String;
  try {
    final root = decoder.decodeExact(materialize(testCase['input']));
    switch (operation) {
      case 'decode-exact':
        return {
          'outcome': 'value',
          'tag': tagProjection(root),
          'header_hex': hex(root.header),
          'value_hex': hex(root.value),
          'encoded_hex': hex(root.encoded),
          'depth': root.depth,
          'elements_read': decoder.elementsRead,
        };
      case 'cursor-script':
        return cursorResult(testCase, decoder, root);
      case 'sequence':
      case 'set':
        final cursor = operation == 'sequence'
            ? decoder.sequence(root)
            : decoder.set(root);
        return {
          'outcome': 'value',
          'elements_read': decoder.elementsRead,
          'remaining_offset': root.value.length - cursor.remaining.length,
        };
      case 'explicit':
        final child = decoder.explicit(root, testCase['tag_number'] as int);
        return {
          'outcome': 'value',
          'tag': tagProjection(child),
          'value_hex': hex(child.value),
          'depth': child.depth,
          'elements_read': decoder.elementsRead,
        };
      default:
        return primitiveResult(
          operation,
          root,
          limits,
          testCase['tag_number'] as int? ?? 0,
        );
    }
  } on Asn1Exception catch (error) {
    final scope = operation == 'explicit' && error.kind == Asn1ErrorKind.framing
        ? 'container-value'
        : 'operation-input';
    return errorProjection(error, scope);
  }
}

void main() {
  final document = loadFixture('der-asn1-v1');
  final upstream = loadFixture('der-tlv-v1');
  final cases = document['cases'] as List<dynamic>;

  test('pins the closed DER ASN.1 profile', () {
    expect(cases, hasLength(109));
    expect(document['error_ids'] as List<dynamic>, hasLength(22));
  });

  for (final raw in cases) {
    final testCase = raw as Map<String, dynamic>;
    test(testCase['id'] as String, () {
      final actual = runCase(document, upstream, testCase);
      expect(actual, testCase['expected']);
      if (testCase['redacted_input_hex'] case final String hostile) {
        expect(jsonEncode(actual), isNot(contains(hostile)));
      }
    });
  }

  test('limits use structural equality and reject negative values', () {
    expect(Asn1Limits(), Asn1Limits());
    expect(Asn1Limits().hashCode, Asn1Limits().hashCode);
    expect(() => Asn1Limits(maxDepth: -1), throwsArgumentError);
    expect(() => Asn1Limits(maxTotalElements: -1), throwsArgumentError);
    expect(() => Asn1Limits(maxOidArcs: -1), throwsArgumentError);
  });

  test('OID equality is exact and element views share the caller buffer', () {
    final input = Uint8List.fromList([0x06, 0x03, 0x2a, 0x03, 0x04]);
    final element = Asn1Decoder().decodeExact(input);
    final oid = decodeObjectIdentifier(element);
    expect(oid.equals([BigInt.one, BigInt.two, BigInt.from(3), BigInt.from(4)]),
        isTrue);
    expect(oid.equals([BigInt.one, BigInt.two, BigInt.from(3)]), isFalse);
    input[4] = 0x7f;
    expect(element.value.last, 0x7f);
  });

  test('OID accepts an unsigned 64-bit maximum arc', () {
    final input = materialize([
      {'hex': '060b2a81ffffffffffffffff7f'},
    ]);
    final oid = decodeObjectIdentifier(Asn1Decoder().decodeExact(input));
    expect(oid.arcs, [BigInt.one, BigInt.two, (BigInt.one << 64) - BigInt.one]);
  });

  test('structurally equal limits share a cursor budget', () {
    final first = Asn1Decoder(Asn1Limits());
    final root =
        first.decodeExact(Uint8List.fromList([0x30, 0x02, 0x05, 0x00]));
    final cursor = first.sequence(root);
    final second = Asn1Decoder(Asn1Limits());
    expect(cursor.read(second), isNotNull);
    expect(second.elementsRead, 1);
    expect(cursor.remaining, isEmpty);
  });

  test('public errors redact hostile payload bytes', () {
    final decoder = Asn1Decoder();
    final element = decoder.decodeExact(
      Uint8List.fromList([0x16, 0x02, 0x61, 0xff]),
    );
    try {
      decodeIA5String(element);
      fail('expected a non-ASCII IA5String error');
    } on Asn1Exception catch (error) {
      expect(error.toString(), isNot(contains('61ff')));
      expect(error.toString(), isNot(contains('255')));
    }
  });
}
