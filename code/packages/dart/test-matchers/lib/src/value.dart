/// The [`Value`] discriminated union: every Dart value, normalized for
/// bit-exact comparison.
///
/// This is the Dart lane of the TypeScript `Value` type from the reference
/// `test-matchers` package (TypeScript lane of TF01). A [`Value`] is what
/// [`toValue`] produces and what [`valuesEqual`] and
/// [`encodeValue`] consume.
///
/// Dart has no `undefined`, and its `int` is an exact 64-bit signed integer,
/// so the mapping differs slightly from TypeScript (see the package README
/// for the full table). The one deliberate cross-language break: there is no
/// `undefined` kind here — Dart has no such value.
library;

import 'dart:typed_data';

/// A Dart value, normalized into one of eleven comparison kinds.
///
/// Instances are immutable snapshots (except that [`BytesValue`] and the
/// [`ListValue`]/[`MapValue`] containers hold the references they were built
/// from, exactly like the TypeScript reference).
sealed class Value {
  const Value();
}

/// The Dart `null` value.
final class NullValue extends Value {
  const NullValue();
}

/// A Dart `bool`.
final class BoolValue extends Value {
  /// The boolean.
  final bool value;
  const BoolValue(this.value);
}

/// An exact integer: a Dart `int` or `BigInt`, rendered as decimal text.
///
/// Dart integers are exact, so unlike the TypeScript reference (whose `int`
/// kind only fires for integral floats) this kind covers real integer types.
final class IntValue extends Value {
  /// The decimal rendering (e.g. `"42"`, `"-7"`).
  final String text;
  const IntValue(this.text);
}

/// A Dart `double`, stored as its raw IEEE-754 bit pattern.
///
/// The bits are stored verbatim — including NaN payloads and the sign of
/// zero. Canonicalization (NaN payload → quiet NaN, −0 → +0) happens in
/// [`canonicalF64`], not here.
final class F64Value extends Value {
  /// The raw 64-bit pattern, as an unsigned [`BigInt`].
  final BigInt bits;
  const F64Value(this.bits);
}

/// A Dart `Uint8List`, shared by reference.
final class BytesValue extends Value {
  /// The bytes (shared reference, as in the TypeScript reference).
  final Uint8List value;
  const BytesValue(this.value);
}

/// A Dart `String`.
final class TextValue extends Value {
  /// The text.
  final String value;
  const TextValue(this.value);
}

/// A Dart `DateTime`, stored as whole milliseconds since the Unix epoch.
final class DateValue extends Value {
  /// Whole milliseconds since 1970-01-01T00:00:00Z.
  final int ms;
  const DateValue(this.ms);
}

/// A Dart `RegExp`: its source pattern plus its reconstructed flags.
///
/// The `i` and `m` flags come from [RegExp.isCaseSensitive] and
/// [RegExp.isMultiLine]; `g` and `u` are always present and `s` and `y`
/// always absent on this platform.
final class RegexpValue extends Value {
  /// The pattern source.
  final String source;

  /// The reconstructed flags, drawn from `"gimsuy"` (`"g"` always present).
  final String flags;
  const RegexpValue(this.source, this.flags);
}

/// A Dart `Error` or `Exception`: its class name plus its message.
final class ErrorValue extends Value {
  /// The runtime type name (e.g. `"StateError"`).
  final String name;

  /// The message — the error's [Object.toString] output.
  final String message;
  const ErrorValue(this.name, this.message);
}

/// A Dart `List` (or `Set`, in iteration order), converted element-wise.
final class ListValue extends Value {
  /// The converted elements, in order.
  final List<Value> items;
  const ListValue(this.items);
}

/// A Dart `Map`, converted entry-wise in iteration order.
///
/// Keys are themselves [`Value`]s, so maps with non-string keys compare by
/// structure, not by string coercion.
final class MapValue extends Value {
  /// The converted entries, in iteration order.
  final List<(Value, Value)> entries;
  const MapValue(this.entries);
}
