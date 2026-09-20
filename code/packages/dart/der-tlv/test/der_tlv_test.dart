import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_der_tlv/der_tlv.dart';
import 'package:test/test.dart';

Map<String, dynamic> loadFixture() {
  final candidates = [
    '../../../../specs/fixtures/der-tlv-v1/cases.json',
    '../../../specs/fixtures/der-tlv-v1/cases.json',
  ];
  for (final candidate in candidates) {
    final file = File(candidate);
    if (file.existsSync()) {
      return jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
    }
  }
  throw StateError('unable to locate DER TLV fixture');
}

Uint8List materialize(List<dynamic> segments) {
  final builder = BytesBuilder(copy: false);
  for (final raw in segments) {
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

DerLimits limits(Map<String, dynamic> document, Map<String, dynamic> testCase) {
  final values = <String, dynamic>{
    ...document['defaults'] as Map<String, dynamic>,
    ...?testCase['limits'] as Map<String, dynamic>?,
  };
  return DerLimits(
    maxInputLen: values['max_input_len'] as int,
    maxValueLen: values['max_value_len'] == 'host-max'
        ? BigInt.from(0x7fffffffffffffff)
        : BigInt.from(values['max_value_len'] as int),
    maxElements: values['max_elements'] as int,
    maxTagNumber: values['max_tag_number'] as int,
  );
}

Map<String, dynamic> elementResult(DerElement element, int offset) => {
      'outcome': 'element',
      'element_offset': offset,
      'tag': {
        'class': element.tag.tagClass.wireName,
        'constructed': element.tag.constructed,
        'number': element.tag.number,
      },
      'header_len': element.header.length,
      'encoded_len': element.encoded.length,
      'remainder_offset': offset + element.encoded.length,
    };

Map<String, dynamic> errorResult(DerException error) => {
      'outcome': 'error',
      'error_id': error.kind.id,
      'offset': error.offset,
    };

Map<String, dynamic> runDecode(
  Map<String, dynamic> testCase,
  Uint8List input,
  DerLimits configured,
) {
  try {
    if (testCase['operation'] == 'decode-one') {
      final result = decodeOne(input, configured);
      final projection = elementResult(result.element, 0);
      expect(projection['remainder_offset'],
          input.length - result.remainder.length);
      return projection;
    }
    return elementResult(decodeExact(input, configured), 0);
  } on DerException catch (error) {
    return errorResult(error);
  }
}

Map<String, dynamic> runCursor(
  Map<String, dynamic> testCase,
  Uint8List input,
  DerLimits configured,
) {
  final cursor = DerCursor(input, configured);
  final events = <Map<String, dynamic>>[];
  for (final action in testCase['actions'] as List<dynamic>) {
    if (action == 'finish') {
      try {
        cursor.finish();
        events.add({'outcome': 'finished'});
      } on DerException catch (error) {
        events.add(errorResult(error));
      }
      continue;
    }
    final offset = input.length - cursor.remaining.length;
    try {
      final element = cursor.read();
      events.add(
        element == null ? {'outcome': 'end'} : elementResult(element, offset),
      );
    } on DerException catch (error) {
      events.add(errorResult(error));
    }
  }
  return {
    'events': events,
    'elements_read': cursor.elementsRead,
    'remaining_offset': input.length - cursor.remaining.length,
  };
}

void main() {
  final fixture = loadFixture();
  final cases = fixture['cases'] as List<dynamic>;

  test('pins the closed DER TLV profile', () {
    expect(cases, hasLength(54));
    expect(fixture['error_ids'] as List<dynamic>, hasLength(17));
  });

  for (final raw in cases) {
    final testCase = raw as Map<String, dynamic>;
    test(testCase['id'] as String, () {
      final input = materialize(testCase['input'] as List<dynamic>);
      final configured = limits(fixture, testCase);
      final actual = testCase['operation'] == 'cursor'
          ? runCursor(testCase, input, configured)
          : runDecode(testCase, input, configured);
      expect(actual, testCase['expected']);
      if (testCase['redacted_input_hex'] case final String hostile) {
        expect(jsonEncode(actual), isNot(contains(hostile)));
      }
    });
  }

  test('element views share the caller buffer', () {
    final input = Uint8List.fromList([0x04, 0x01, 0x2a]);
    final element = decodeExact(input);
    input[2] = 0x7f;
    expect(element.value, [0x7f]);
  });

  test('negative limits are rejected', () {
    expect(() => DerLimits(maxElements: -1), throwsArgumentError);
    expect(() => DerLimits(maxTagNumber: 0x100000000), throwsArgumentError);
  });
}
