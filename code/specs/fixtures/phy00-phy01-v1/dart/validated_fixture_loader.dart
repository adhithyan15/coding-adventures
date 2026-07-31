import 'dart:convert';

const double _maximumTolerance = 1e-10;

Map<String, dynamic> decodeValidatedFixture(
  String encoded, {
  required String expectedSuite,
}) {
  final bytes = base64Decode(encoded);
  if (bytes.length > 1000000) {
    throw const FormatException('fixture exceeds byte limit');
  }
  final decoded = jsonDecode(utf8.decode(bytes, allowMalformed: false));
  if (decoded is! Map<String, dynamic>) {
    throw const FormatException('fixture root must be an object');
  }
  if (decoded.keys.toSet().difference({
    'schema_version',
    'suite',
    'summary',
    'cases',
  }).isNotEmpty) {
    throw const FormatException('fixture root contains an unknown field');
  }
  if (decoded['schema_version'] != 1 || decoded['suite'] != expectedSuite) {
    throw const FormatException('fixture schema version or suite mismatch');
  }
  final cases = decoded['cases'];
  if (cases is! List || cases.isEmpty || cases.length > 128) {
    throw const FormatException(
      'fixture cases must be a bounded nonempty list',
    );
  }
  _validateValue(decoded, 0);
  return decoded;
}

void _validateValue(Object? value, int depth) {
  if (depth > 32) {
    throw const FormatException('fixture nesting exceeds 32 levels');
  }
  if (value is double) {
    throw const FormatException(
      'native JSON floating-point values are forbidden',
    );
  }
  if (value is List) {
    for (final item in value) {
      _validateValue(item, depth + 1);
    }
    return;
  }
  if (value is! Map) {
    return;
  }

  if (value['kind'] == 'finite') {
    final decimal = value['decimal'];
    if (decimal is! String) {
      throw const FormatException('finite scalar requires a decimal string');
    }
    _decodeFinite(decimal);
  }
  if ((value['kind'] == 'absolute' || value['kind'] == 'relative')) {
    final decimal = value['tolerance'];
    if (decimal is! String) {
      throw const FormatException(
        'approximate comparison requires a tolerance',
      );
    }
    final tolerance = _decodeFinite(decimal);
    if (tolerance <= 0.0 || tolerance > _maximumTolerance) {
      throw const FormatException('tolerance is not positive and bounded');
    }
  }
  for (final entry in value.entries) {
    if (entry.key is! String) {
      throw const FormatException('fixture object keys must be strings');
    }
    _validateValue(entry.value, depth + 1);
  }
}

double _decodeFinite(String decimal) {
  final decoded = double.tryParse(decimal);
  if (decoded == null || !decoded.isFinite) {
    throw const FormatException('finite decimal is outside binary64 range');
  }
  final mantissa = decimal.split(RegExp('[eE]')).first;
  final digits = mantissa.replaceAll(RegExp(r'[-.]'), '');
  final mathematicallyZero = !digits.contains(RegExp('[1-9]'));
  if (decoded == 0.0 && !mathematicallyZero) {
    throw const FormatException('nonzero finite decimal underflows binary64');
  }
  return decoded;
}
