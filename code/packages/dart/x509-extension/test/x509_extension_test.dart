import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_der_asn1/der_asn1.dart';
import 'package:coding_adventures_der_tlv/der_tlv.dart';
import 'package:coding_adventures_x509_extension/x509_extension.dart';
import 'package:test/test.dart';

Map<String, dynamic> fixture(String name) {
  for (final candidate in [
    '../../../specs/fixtures/$name/cases.json',
    '../../../../specs/fixtures/$name/cases.json',
  ]) {
    final file = File(candidate);
    if (file.existsSync()) {
      return jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
    }
  }
  throw StateError('unable to locate $name fixture');
}

Uint8List materialize(Object? raw) {
  final output = BytesBuilder(copy: false);
  for (final item in raw! as List<dynamic>) {
    final segment = item as Map<String, dynamic>;
    if (segment['hex'] case final String value) {
      output.add([
        for (var index = 0; index < value.length; index += 2)
          int.parse(value.substring(index, index + 2), radix: 16),
      ]);
    } else {
      output.add(
        Uint8List(segment['count'] as int)
          ..fillRange(
            0,
            segment['count'] as int,
            int.parse(segment['repeat_hex'] as String, radix: 16),
          ),
      );
    }
  }
  return output.takeBytes();
}

Object? merged(
  Map<String, dynamic> defaults,
  Map<String, dynamic> overrides,
  String name,
) =>
    overrides.containsKey(name) ? overrides[name] : defaults[name];

Asn1Limits limits(
  Map<String, dynamic> upstream,
  Map<String, dynamic> testCase,
) {
  final defaults = upstream['defaults'] as Map<String, dynamic>;
  final overrides =
      testCase['limits'] as Map<String, dynamic>? ?? <String, dynamic>{};
  final derDefaults = defaults['der'] as Map<String, dynamic>;
  final derOverrides =
      overrides['der'] as Map<String, dynamic>? ?? <String, dynamic>{};
  final maxValue = merged(derDefaults, derOverrides, 'max_value_len');
  return Asn1Limits(
    der: DerLimits(
      maxInputLen: merged(derDefaults, derOverrides, 'max_input_len') as int,
      maxValueLen: maxValue == 'host-max'
          ? BigInt.parse('9223372036854775807')
          : BigInt.from(maxValue! as int),
      maxElements: merged(derDefaults, derOverrides, 'max_elements') as int,
      maxTagNumber:
          merged(derDefaults, derOverrides, 'max_tag_number') as int,
    ),
    maxDepth: merged(defaults, overrides, 'max_depth') as int,
    maxTotalElements:
        merged(defaults, overrides, 'max_total_elements') as int,
    maxOidArcs: merged(defaults, overrides, 'max_oid_arcs') as int,
  );
}

String hex(Uint8List value) =>
    value.map((octet) => octet.toRadixString(16).padLeft(2, '0')).join();

Map<String, dynamic> attempt(Asn1Decoder decoder, Asn1Element root) {
  try {
    final value = decodeX509Extension(decoder, root);
    return {
      'outcome': 'value',
      'extension_id_arcs_decimal': [
        for (final arc in value.extensionId.arcs) arc.toString(),
      ],
      'critical': value.critical,
      'extension_value_hex': hex(value.extensionValue),
      'elements_read': decoder.elementsRead,
    };
  } on X509ExtensionError catch (error) {
    return {
      'outcome': 'error',
      'error_id': error.kind.id,
      if (error.asn1Kind case final Asn1ErrorKind kind)
        'asn1_error_id': kind.id,
      if (error.framingKind case final DerErrorKind kind)
        'framing_error_id': kind.id,
      'offset': error.offset,
      'offset_scope': 'extension-element',
      'elements_read': decoder.elementsRead,
    };
  }
}

void main() {
  final contract = fixture('x509-extension-v1');
  final upstream = fixture('der-asn1-v1');

  test('consumes every closed x509-extension-v1 case', () {
    final cases =
        (contract['cases'] as List<dynamic>).cast<Map<String, dynamic>>();
    expect(cases, hasLength(48));
    for (final testCase in cases) {
      final decoder = Asn1Decoder(limits(upstream, testCase));
      final root = decoder.decodeExact(materialize(testCase['input']));
      final actual = testCase['operation'] == 'extension-script'
          ? {
              'outcome': 'script',
              'events': [
                for (final _ in testCase['actions'] as List<dynamic>)
                  attempt(decoder, root),
              ],
            }
          : attempt(decoder, root);
      expect(actual, testCase['expected'], reason: testCase['id'] as String);
      if (testCase['redacted_input_hex'] case final String hostile) {
        expect(jsonEncode(actual), isNot(contains(hostile)));
      }
    }
  });

  test('snapshots identifier and opaque bytes', () {
    final decoder = Asn1Decoder();
    final input = materialize([
      {'hex': '30090603551d1104023000'},
    ]);
    final value = decodeX509Extension(decoder, decoder.decodeExact(input));
    input.fillRange(0, input.length, 0xff);
    final opaque = value.extensionValue..fillRange(0, 2, 0xff);
    final encoded = value.extensionId.encoded..fillRange(0, 3, 0xff);
    expect(opaque, isNot(value.extensionValue));
    expect(encoded, isNot(value.extensionId.encoded));
    expect(hex(value.extensionValue), '3000');
    expect(value.extensionId.arcs.map((arc) => arc.toString()),
        ['2', '5', '29', '17']);
    expect(() => value.extensionId.arcs.add(BigInt.zero), throwsUnsupportedError);
  });
}
