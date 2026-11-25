import 'dart:math';
import 'package:coding_adventures_starlark_compiler/starlark_compiler.dart';
import 'package:test/test.dart';

void main() {
  group('Opcode Enumeration & Properties', () {
    test('defines exactly 61 opcodes', () {
      expect(Op.values.length, equals(61));
      expect(expectedOpcodeCount, equals(61));
    });

    test('all opcodes have valid 8-bit unsigned byte values', () {
      for (final op in Op.values) {
        expect(op.byteValue, inInclusiveRange(0x01, 0xFF));
        expect(opByte(op), equals(op.byteValue));
        expect(op.symbol, isNotEmpty);
        expect(opSymbol(op), equals(op.symbol));
        expect(op.toString(), equals(op.symbol));
      }
    });

    test('all opcodes match their category high nibbles', () {
      for (final op in Op.values) {
        final expectedNibble = (op.byteValue >> 4) & 0x0F;
        expect(op.category.nibble, equals(expectedNibble),
            reason: 'Op ${op.symbol} (0x${op.byteValue.toRadixString(16)}) '
                'nibble mismatch with category ${op.category.id}');
        expect(opCategory(op), equals(op.category));
      }
    });
  });

  group('OpCategory Enumeration & Properties', () {
    test('defines exactly 12 categories', () {
      expect(OpCategory.values.length, equals(12));
    });

    test('categories expose snake_case and camelCase identifiers', () {
      const expectedPairs = <OpCategory, (String, String)>{
        OpCategory.stack: ('stack', 'stack'),
        OpCategory.variable: ('variable', 'variable'),
        OpCategory.arithmetic: ('arithmetic', 'arithmetic'),
        OpCategory.comparison: ('comparison', 'comparison'),
        OpCategory.controlFlow: ('control_flow', 'controlFlow'),
        OpCategory.function_: ('function', 'function'),
        OpCategory.collection: ('collection', 'collection'),
        OpCategory.subscriptAttribute: (
          'subscript_attribute',
          'subscriptAttribute'
        ),
        OpCategory.iteration: ('iteration', 'iteration'),
        OpCategory.module: ('module', 'module'),
        OpCategory.io: ('io', 'io'),
        OpCategory.vmControl: ('vm_control', 'vmControl'),
      };

      for (final category in OpCategory.values) {
        final expected = expectedPairs[category]!;
        expect(category.id, equals(expected.$1));
        expect(category.camelName, equals(expected.$2));
        expect(category.toString(), equals(category.id));
      }
    });
  });

  group('Public Provider Functions & Defensive Copying', () {
    test('allOps returns exactly 61 opcodes in canonical order', () {
      final ops = allOps();
      expect(ops.length, equals(61));
      expect(ops, equals(Op.values));

      // Assert defensive copy
      ops.clear();
      expect(ops, isEmpty);
      expect(allOps().length, equals(61));
    });

    test('opFromByte resolves all 61 assigned opcodes', () {
      for (final op in Op.values) {
        final resolved = opFromByte(op.byteValue);
        expect(resolved, equals(op));
      }
    });

    test('opFromByte returns null for unassigned byte holes and out of range',
        () {
      expect(opFromByte(-1), isNull);
      expect(opFromByte(256), isNull);
      expect(opFromByte(1000), isNull);
      expect(opFromByte(0x00), isNull); // 0x00 is unassigned hole
      expect(opFromByte(0x07), isNull); // Hole in stack group
      expect(opFromByte(0x16), isNull); // Hole in variable group
      expect(opFromByte(0xFE), isNull); // Hole before halt
    });

    test('opCategoryFromByte returns correct category or null for holes', () {
      expect(opCategoryFromByte(0x01), equals(OpCategory.stack));
      expect(opCategoryFromByte(0x20), equals(OpCategory.arithmetic));
      expect(opCategoryFromByte(0xFF), equals(OpCategory.vmControl));
      expect(opCategoryFromByte(0x00), isNull);
      expect(opCategoryFromByte(-1), isNull);
      expect(opCategoryFromByte(256), isNull);
    });

    test('opFromSymbol resolves all 61 symbols and returns null for unknown',
        () {
      for (final op in Op.values) {
        expect(opFromSymbol(op.symbol), equals(op));
      }
      expect(opFromSymbol('Unknown'), isNull);
      expect(opFromSymbol(''), isNull);
      expect(opFromSymbol('loadconst'), isNull); // case sensitive
    });
  });

  group('Operator Mappings & Fresh-Map Ownership', () {
    test('binaryOpMap contains 12 operators and enforces fresh-map ownership',
        () {
      final map = binaryOpMap();
      expect(map.length, equals(12));
      expect(map['+'], equals(Op.add));
      expect(map['-'], equals(Op.sub));
      expect(map['*'], equals(Op.mul));
      expect(map['/'], equals(Op.div));
      expect(map['//'], equals(Op.floorDiv));
      expect(map['%'], equals(Op.mod));
      expect(map['**'], equals(Op.power));
      expect(map['&'], equals(Op.bitAnd));
      expect(map['|'], equals(Op.bitOr));
      expect(map['^'], equals(Op.bitXor));
      expect(map['<<'], equals(Op.lShift));
      expect(map['>>'], equals(Op.rShift));

      // Test mutation isolation
      map.clear();
      expect(binaryOpMap().length, equals(12));
      expect(binaryOpcode('+'), equals(Op.add));
    });

    test('compareOpMap contains 8 operators and enforces fresh-map ownership',
        () {
      final map = compareOpMap();
      expect(map.length, equals(8));
      expect(map['=='], equals(Op.cmpEq));
      expect(map['!='], equals(Op.cmpNe));
      expect(map['<'], equals(Op.cmpLt));
      expect(map['>'], equals(Op.cmpGt));
      expect(map['<='], equals(Op.cmpLe));
      expect(map['>='], equals(Op.cmpGe));
      expect(map['in'], equals(Op.cmpIn));
      expect(map['not in'], equals(Op.cmpNotIn));

      map['=='] = Op.halt;
      expect(compareOpMap()['=='], equals(Op.cmpEq));
      expect(compareOpcode('=='), equals(Op.cmpEq));
    });

    test(
        'augmentedAssignMap contains 12 operators and enforces fresh-map ownership',
        () {
      final map = augmentedAssignMap();
      expect(map.length, equals(12));
      expect(map['+='], equals(Op.add));
      expect(map['-='], equals(Op.sub));
      expect(map['*='], equals(Op.mul));
      expect(map['/='], equals(Op.div));
      expect(map['//='], equals(Op.floorDiv));
      expect(map['%='], equals(Op.mod));
      expect(map['**='], equals(Op.power));
      expect(map['&='], equals(Op.bitAnd));
      expect(map['|='], equals(Op.bitOr));
      expect(map['^='], equals(Op.bitXor));
      expect(map['<<='], equals(Op.lShift));
      expect(map['>>='], equals(Op.rShift));

      map.remove('+=');
      expect(augmentedAssignMap().containsKey('+='), isTrue);
      expect(augmentedAssignOpcode('+='), equals(Op.add));
    });

    test('unaryOpMap contains 2 operators and enforces fresh-map ownership',
        () {
      final map = unaryOpMap();
      expect(map.length, equals(2));
      expect(map['-'], equals(Op.negate));
      expect(map['~'], equals(Op.bitNot));

      map.clear();
      expect(unaryOpMap().length, equals(2));
      expect(unaryOpcode('-'), equals(Op.negate));
    });

    test('individual opcode lookups return null for unknown operators', () {
      expect(binaryOpcode('@'), isNull);
      expect(binaryOpcode('++'), isNull);
      expect(compareOpcode('IN'), isNull);
      expect(compareOpcode('not  in'), isNull);
      expect(augmentedAssignOpcode(':='), isNull);
      expect(unaryOpcode('+'), isNull);
      expect(unaryOpcode('not'), isNull);
    });
  });

  group('Standing Rule #20 Diagnostics & Exception Hierarchy', () {
    test('InvalidByteException complies with closed diagnostic invariants', () {
      const ex = InvalidByteException();
      expect(ex.code, equals(StarlarkCompilerErrorCode.invalidByte));
      expect(ex.code.id, equals('invalid-byte'));
      expect(ex.message, equals(StarlarkCompilerMessages.invalidByte));
      expect(ex.toString(), contains('invalid-byte'));
    });

    test('UnknownOpcodeException complies with closed diagnostic invariants',
        () {
      const ex = UnknownOpcodeException();
      expect(ex.code, equals(StarlarkCompilerErrorCode.unknownOpcode));
      expect(ex.code.id, equals('unknown-opcode'));
      expect(ex.message, equals(StarlarkCompilerMessages.unknownOpcode));
      expect(ex.toString(), contains('unknown-opcode'));
    });

    test('UnknownOperatorException complies with closed diagnostic invariants',
        () {
      const ex = UnknownOperatorException();
      expect(ex.code, equals(StarlarkCompilerErrorCode.unknownOperator));
      expect(ex.code.id, equals('unknown-operator'));
      expect(ex.message, equals(StarlarkCompilerMessages.unknownOperator));
      expect(ex.toString(), contains('unknown-operator'));
    });

    test('InvalidCorpusException factory variants comply with Rule #20', () {
      const ex1 = InvalidCorpusException.byteLimitExceeded();
      expect(ex1.code, equals(StarlarkCompilerErrorCode.invalidCorpus));
      expect(ex1.code.id, equals('invalid-corpus'));
      expect(ex1.message,
          equals(StarlarkCompilerMessages.corpusByteLimitExceeded));

      const ex2 = InvalidCorpusException.malformedTransport();
      expect(ex2.code, equals(StarlarkCompilerErrorCode.invalidCorpus));
      expect(ex2.message,
          equals(StarlarkCompilerMessages.corpusMalformedTransport));

      const ex3 = InvalidCorpusException.schemaViolation();
      expect(ex3.code, equals(StarlarkCompilerErrorCode.invalidCorpus));
      expect(
          ex3.message, equals(StarlarkCompilerMessages.corpusSchemaViolation));
    });

    test('centralized messages include redacted and invalid placeholders', () {
      expect(StarlarkCompilerMessages.redacted, equals('<redacted>'));
      expect(StarlarkCompilerMessages.invalid, equals('<invalid>'));
    });
  });

  group('5-Class Security Resilience Showcase', () {
    test('Class 1: Bounded Memory & Input Envelope Protection', () {
      expect(maxCorpusBytes, equals(131072));
      expect(maxJsonNestingDepth, equals(8));
      expect(expectedHoleCount, equals(195));
      expect(expectedDecodeCount, equals(256));
    });

    test('Class 2: Lexical, Character & Encoding Integrity', () {
      expect(minSurrogateCodePoint, equals(0xD800));
      expect(maxSurrogateCodePoint, equals(0xDFFF));
      expect(minHighSurrogate, equals(0xD800));
      expect(maxHighSurrogate, equals(0xDBFF));
      expect(minLowSurrogate, equals(0xDC00));
      expect(maxLowSurrogate, equals(0xDFFF));
    });

    test('Class 3: Deterministic Fresh-State Isolation', () {
      final list1 = allOps();
      final list2 = allOps();
      expect(identical(list1, list2), isFalse);

      final map1 = binaryOpMap();
      final map2 = binaryOpMap();
      expect(identical(map1, map2), isFalse);
    });

    test('Class 4: Closed Diagnostic Catalog & Payload Blindness', () {
      const exceptions = <StarlarkCompilerException>[
        InvalidByteException(),
        UnknownOpcodeException(),
        UnknownOperatorException(),
        InvalidCorpusException.byteLimitExceeded(),
        InvalidCorpusException.malformedTransport(),
        InvalidCorpusException.schemaViolation(),
      ];
      for (final ex in exceptions) {
        expect(ex.message, isNotEmpty);
        expect(ex.message.contains('%s'), isFalse);
        expect(ex.message.contains('%d'), isFalse);
        expect(ex.message.contains('<script>'), isFalse);
      }
    });

    test('Class 5: Adversarial Transport & Mutation Resilience', () {
      expect(minByteValue, equals(0x00));
      expect(maxByteValue, equals(0xFF));
      expect(stp01SchemaVersion, equals(1));
      expect(stp01Profile, equals('standalone-provider-v1'));
    });
  });

  group('200-Case Deterministic PRNG Property Test', () {
    test('cross-validates random opcode, category, and operator queries', () {
      final rng = Random(0x5741524C41524B); // 'STARLARK'

      for (var i = 0; i < 200; i++) {
        // 1. Random byte query
        final queryByte = rng.nextInt(300) - 20; // range [-20, 279]
        final op = opFromByte(queryByte);
        final cat = opCategoryFromByte(queryByte);

        if (queryByte < 0 || queryByte > 255) {
          expect(op, isNull);
          expect(cat, isNull);
        } else if (op != null) {
          expect(op.byteValue, equals(queryByte));
          expect(cat, equals(op.category));
          expect(opByte(op), equals(queryByte));
          expect(opSymbol(op), equals(op.symbol));
          expect(opCategory(op), equals(op.category));
          expect(opFromSymbol(op.symbol), equals(op));
        } else {
          expect(cat, isNull);
        }

        // 2. Random operator query
        const allOpSymbols = [
          '+',
          '-',
          '*',
          '/',
          '//',
          '%',
          '**',
          '&',
          '|',
          '^',
          '<<',
          '>>',
          '==',
          '!=',
          '<',
          '>',
          '<=',
          '>=',
          'in',
          'not in',
          '+=',
          '-=',
          '*=',
          '/=',
          '//=',
          '%=',
          '**=',
          '&=',
          '|=',
          '^=',
          '<<=',
          '>>=',
          '-',
          '~',
          '@',
          '++',
          ':=',
          'not',
          'unknown',
          '',
        ];
        final opSym = allOpSymbols[rng.nextInt(allOpSymbols.length)];
        final bOp = binaryOpcode(opSym);
        final cOp = compareOpcode(opSym);
        final aOp = augmentedAssignOpcode(opSym);
        final uOp = unaryOpcode(opSym);

        if (bOp != null) {
          expect(binaryOpMap()[opSym], equals(bOp));
        }
        if (cOp != null) {
          expect(compareOpMap()[opSym], equals(cOp));
        }
        if (aOp != null) {
          expect(augmentedAssignMap()[opSym], equals(aOp));
        }
        if (uOp != null) {
          expect(unaryOpMap()[opSym], equals(uOp));
        }
      }
    });
  });
}
