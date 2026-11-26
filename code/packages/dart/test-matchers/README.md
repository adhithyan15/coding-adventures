# coding_adventures_test_matchers (Dart)

Jest/Vitest-style test matchers with bit-exact deep equality. The Dart lane of
the TypeScript
[`test-matchers`](../../typescript/test-matchers/README.md)
reference package: same matchers, same messages, same comparison engine —
adapted only where the Dart type system forces a break (table below).

```dart
import 'package:coding_adventures_test_matchers/coding_adventures_test_matchers.dart';

expect(42).toBe(42);
expect({'a': 1, 'b': [2, 3]}).toEqual({'a': 1, 'b': [2, 3]});
expect(() => throw StateError('boom')).toThrow('boom');
await expect(Future.value(7)).resolves.toBe(7);
```

## API

Sync matchers on `expect(actual)` (`Expectation`); every matcher also exists
on `resolves`/`rejects` (`AsyncExpectation`), awaited first:

| Matcher | Rule |
|---|---|
| `toBe` | `Object.is` semantics: NaN equals NaN, −0.0 differs from +0.0, strings and `BigInt`s by value, otherwise `identical` |
| `toEqual` | bit-exact structural equality (`deepEqual`): NaN payloads and zero signs canonicalized, maps order-insensitive |
| `toStrictEqual` | `toEqual` plus erased-type equality (all `List`s share one kind, as in JS; other objects need the same runtime type) |
| `toBeTruthy` / `toBeFalsy` | identical to `true` / `false` (no coercion — see below) |
| `toBeNull` | identical to `null` |
| `toBeInstanceOf<T>()` | `actual is T` |
| `toBeNaN` | a NaN double |
| `toBeGreaterThan` / `OrEqual`, `toBeLessThan` / `OrEqual` | total ordering over num/`BigInt` pairs (exact `int`↔`BigInt`, lexicographic strings); anything else mismatches, never throws |
| `toBeCloseTo(expected, [precision = 2])` | within `10^-precision / 2` |
| `toContain` | substring / deep-equal element / deep-equal key |
| `toHaveLength` | duck-typed integer `length` |
| `toMatch(Pattern)` | substring (`String`) or match (`RegExp`) |
| `toHaveProperty(key, [value])` | `Map.containsKey` (null keys allowed) or integer `List` index, with optional deep-equal value check |
| `toThrow([expected])` | zero-arg closure throws; `expected` is a message substring, `RegExp`, `Error`/`Exception` instance (exact type + `toString()`), or `Type` (exact, except the `Error`/`Exception` roots which match any) |

Failures throw `AssertionError` (extending Dart's own, so both `on
AssertionError` spellings catch it) with `message`, `actual`, `expected`,
and `operator` fields. `.not` negates every matcher and prefixes the message
with `not `. Custom matchers build a `MatcherResult` and hand it to
`Expectation.assertResult`.

The comparison engine is public too: `toValue` (normalize to the `Value`
union), `valuesEqual`, `deepEqual`, `encodeValue` (diagnostics),
`numberToF64Bits` / `f64BitsToNumber` / `canonicalF64`, and `regexpFlags`.

## TypeScript → Dart porting table

| TypeScript | Dart | Notes |
|---|---|---|
| `toBeUndefined` / `toBeDefined` | — (dropped) | Dart has no `undefined` |
| truthy/falsy coercion | bool-only | `toBeTruthy` passes only for `true`; `1`, `"s"`, `{}`, … fail |
| `toBeInstanceOf(Ctor)` | `toBeInstanceOf<T>()` | type argument instead of constructor value |
| `toThrow(Ctor)` | `toThrow(Type)` | exact runtime type, except `Error`/`Exception` roots (Dart has no dynamic `instanceof`) |
| `1` vs `1.0` | distinct | one number type in JS; `int` ≠ `double` here, in both `toBe` and `toEqual` |
| plain objects → sorted-key maps | objects → `toString()` text | no reflection; class instances compare by rendering (type check still exact in `toStrictEqual`) |
| functions compare by source | closures compare by signature rendering | same-signature closures are indistinguishable |
| `toHaveProperty("0")` on arrays | int indices only | Dart lists are int-indexed |
| `Error.message` matching | `toString()` matching | Dart errors lack a uniform `.message` |
| `RegExp.flags` | reconstructed `gu`/`giu`/`gmu`/`gimu` | `g`+`u` fixed present, `s`+`y` fixed absent; `i`/`m` from accessors |

Everything else — matcher names, message texts, operator strings, `Value`
kinds, `[Circular]` collapse, the 64-level depth bound, map
order-insensitivity, the `toThrow` unknown-type fall-through, the
`toStrictEqual`-reports-`toEqual` delegation — mirrors the reference.

## Validation

- **Unit** (`test/matchers_test.dart`): mirrors the reference suite
  group-for-group, plus Dart-only branches. 55 tests.
- **Property** (`test/properties_test.dart`): reflexivity, symmetry, f64
  bit-exact round-trip, `canonicalF64` idempotence + exactness, membership,
  map order-insensitivity, `encodeValue` totality — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration bounded campaign over a
  hostile byte decoder (deep nesting past the 64 cap, NaN payloads, null map
  keys, invalid regex sources). It caught one real bug before release:
  `toHaveProperty` took `Object key` and threw `_TypeError` on null keys;
  the signature is now `Object?` with regression tests. Ad-hoc 60,000-iteration
  runs over three more seeds: zero crashes.
- **Coverage**: 100% lines, 100% functions, 99.6% branches (the one miss is a
  provably dead defensive arc, documented in `deep_equal.dart`).
