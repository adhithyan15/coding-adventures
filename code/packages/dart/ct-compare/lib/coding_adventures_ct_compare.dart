/// Constant-time byte comparison and selection (CT01).
///
/// ```dart
/// import 'package:coding_adventures_ct_compare/coding_adventures_ct_compare.dart';
///
/// final same = ctEq(expectedTag, receivedTag);
/// final picked = ctSelectBytes(accept, reject, ctEqBit(expectedTag, receivedTag));
/// ```
library coding_adventures_ct_compare;

export 'src/ct_compare.dart';
