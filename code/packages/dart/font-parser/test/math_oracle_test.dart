/// MATH-table tests for `coding_adventures_font_parser`: the 10 oracle
/// tests (`code/packages/rust/font-parser/tests/math_oracle.rs`) checked
/// against values produced by fontTools from STIX Two Math, plus the
/// corruption-branch extras (zeroed offsets, disagreeing counts, truncated
/// subtables) built on a synthetic MATH table with full control over every
/// offset.
library;

import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_font_parser/font_parser.dart';
import 'package:test/test.dart';

import 'font_fixtures.dart';

void main() {
  group('oracle: MathConstants', () {
    test('constants match an independent implementation', () {
      final font = load(fontWithMathTable());
      expect(hasMathTable(font), isTrue);
      final c = mathConstants(font)!;
      // Every number here came from fontTools, not from us.
      expect(c.scriptPercentScaleDown, 70);
      expect(c.scriptScriptPercentScaleDown, 55);
      expect(c.delimitedSubFormulaMinHeight, 1325);
      expect(c.displayOperatorMinHeight, 1800);
      expect(c.mathLeading, 150);
      expect(c.axisHeight, 258);
      expect(c.accentBaseHeight, 480);
      expect(c.subscriptShiftDown, 210);
      expect(c.superscriptShiftUp, 360);
      expect(c.fractionNumeratorShiftUp, 585);
      expect(c.fractionDenominatorShiftDown, 585);
      expect(c.fractionRuleThickness, 68);
      expect(c.radicalVerticalGap, 85);
      expect(c.radicalRuleThickness, 68);
    });

    test('the MathValue stride is four bytes, not two', () {
      // With a two-byte stride, AxisHeight would read whatever sits at
      // the MathLeading device offset. Distinct constants collapsing to
      // one value is the signature of a wrong stride.
      final font = load(fontWithMathTable());
      final c = mathConstants(font)!;
      expect(c.axisHeight, isNot(c.mathLeading));
      expect(c.axisHeight, 258);
    });
  });

  group('oracle: italic corrections', () {
    test('corrections match the oracle', () {
      // fontTools: glyph 1229 (U+2113, script small l) -> 25
      //            glyph 3331 (U+1D453, italic f)      -> 20
      final font = load(fontWithMathTable());
      expect(italicCorrection(font, 1229), 25);
      expect(italicCorrection(font, 3331), 20);
    });

    test('a glyph with no entry reports null rather than zero', () {
      final font = load(fontWithMathTable());
      expect(italicCorrection(font, 0), isNull);
    });

    test('a font without a MATH table reports absence, not failure', () {
      final bytes =
          File('../../../fixtures/fonts/Inter-Regular.ttf').readAsBytesSync();
      final font = load(bytes);
      expect(hasMathTable(font), isFalse);
      expect(mathConstants(font), isNull);
      expect(italicCorrection(font, 1), isNull);
      expect(minConnectorOverlap(font), isNull);
      expect(glyphConstruction(font, 1, StretchAxis.vertical), isNull);
    });
  });

  group('oracle: MathVariants', () {
    test('a parenthesis has both variants and an assembly', () {
      final font = load(fontWithMathTable());
      final construction = glyphConstruction(font, 1064, StretchAxis.vertical)!;

      expect(construction.variants.length, 13);
      expect(construction.variants[0].glyphId, 1064);
      expect(construction.variants[0].advance, 933);
      expect(construction.variants[1].glyphId, 1301);
      expect(construction.variants[1].advance, 1187);
      // Variants increase in size; a renderer picks the first big
      // enough, so an unsorted read would pick the wrong one.
      final sizes = construction.variants.map((v) => v.advance).toList();
      for (var i = 1; i < sizes.length; i++) {
        expect(sizes[i - 1], lessThanOrEqualTo(sizes[i]));
      }

      expect(construction.assembly.length, 3);
      expect(construction.assembly[0].glyphId, 4862);
      expect(construction.assembly[0].startConnector, 0);
      expect(construction.assembly[0].endConnector, 250);
      expect(construction.assembly[0].fullAdvance, 1273);
      expect(construction.assembly[0].isExtender, isFalse);
      expect(construction.assembly[1].glyphId, 4861);
      expect(construction.assembly[1].isExtender, isTrue);
      expect(construction.assembly[2].glyphId, 4860);
      expect(construction.assembly[2].endConnector, 0);
      expect(construction.assembly[2].isExtender, isFalse);
    });

    test('a brace assembles from five pieces with two extenders', () {
      final font = load(fontWithMathTable());
      final construction = glyphConstruction(font, 1068, StretchAxis.vertical)!;
      expect(construction.assembly.map((p) => p.glyphId).toList(),
          [4874, 4875, 4873, 4875, 4872]);
      expect(construction.assembly.map((p) => p.isExtender).toList(),
          [false, true, false, true, false]);
      expect(construction.assembly[2].fullAdvance, 1947);
    });

    test('the minimum connector overlap is read', () {
      final font = load(fontWithMathTable());
      expect(minConnectorOverlap(font), 100);
    });

    test('a vertical-only glyph has no horizontal construction', () {
      final font = load(fontWithMathTable());
      expect(glyphConstruction(font, 1064, StretchAxis.horizontal), isNull);
    });

    test('a glyph with no construction reports absence', () {
      final font = load(fontWithMathTable());
      for (final axis in StretchAxis.values) {
        expect(glyphConstruction(font, 1, axis), isNull);
      }
    });

    test('a horizontal construction is found on the horizontal axis', () {
      // fontTools: glyph 746 (uni0305, overline) stretches horizontally
      // with 6 variants and a 2-part assembly. This is the only oracle
      // that walks the horizontal construction array, whose start depends
      // on the vertical count — the arithmetic has no vertical analogue.
      final font = load(fontWithMathTable());
      final construction =
          glyphConstruction(font, 746, StretchAxis.horizontal)!;
      expect(
          construction.variants.map((v) => (v.glyphId, v.advance)).toList(), [
        (746, 401),
        (1425, 451),
        (1426, 1166),
        (1427, 1941),
        (1428, 2561),
        (1429, 3073),
      ]);
      // Quirky but real: both parts are glyph 1425, the first an
      // extender, the second fixed. Verified against the raw table bytes
      // as well as fontTools.
      expect(construction.assemblyItalicsCorrection, 0);
      expect(construction.assembly.length, 2);
      expect(construction.assembly[0].glyphId, 1425);
      expect(construction.assembly[0].startConnector, 1000);
      expect(construction.assembly[0].endConnector, 1000);
      expect(construction.assembly[0].fullAdvance, 451);
      expect(construction.assembly[0].isExtender, isTrue);
      expect(construction.assembly[1].glyphId, 1425);
      expect(construction.assembly[1].isExtender, isFalse);
    });

    test('variants without an assembly read as variants-only', () {
      // fontTools: glyph 732 (uni0302, circumflex) has 6 horizontal
      // variants and no assembly — the largest variant is the limit.
      final font = load(fontWithMathTable());
      final construction =
          glyphConstruction(font, 732, StretchAxis.horizontal)!;
      expect(
          construction.variants.map((v) => (v.glyphId, v.advance)).toList(), [
        (732, 283),
        (1395, 574),
        (1396, 1003),
        (1397, 1496),
        (1398, 1932),
        (1399, 2385),
      ]);
      expect(construction.assembly, isEmpty);
      expect(construction.assemblyItalicsCorrection, 0);
    });
  });

  group('corrupt MATH', () {
    /// Locates the live STIX offsets inside a container: table base plus
    /// the three subtable offsets from the MATH header.
    (int, int, int, int) mathLayout(Uint8List container) {
      final mathAt = tableOffset(container, 'MATH');
      return (
        mathAt,
        readU16(container, mathAt + 4),
        readU16(container, mathAt + 6),
        readU16(container, mathAt + 8),
      );
    }

    test('zero constants offset names the missing subtable', () {
      var container = fontWithMathTable();
      container = patchU16(container, tableOffset(container, 'MATH') + 4, 0);
      expect(() => mathConstants(load(container)),
          throwsA(const TableNotFound('MATH.MathConstants')));
    });

    test('constants past the end throw BufferTooShort', () {
      var container = fontWithMathTable();
      final mathAt = tableOffset(container, 'MATH');
      container = patchU16(container, mathAt + 4, 0x7FFF);
      expect(() => mathConstants(load(container)),
          throwsA(const BufferTooShort()));
    });

    test('truncated bare constants throw BufferTooShort', () {
      var container = fontWithMathTable();
      final mathAt = tableOffset(container, 'MATH');
      // Bare fields read, the first MathValue does not.
      container =
          patchU16(container, mathAt + 4, container.length - mathAt - 8);
      expect(() => mathConstants(load(container)),
          throwsA(const BufferTooShort()));
    });

    test('MATH header past the end throws on every reader', () {
      // Point the MATH record itself at the last 4 bytes: every header
      // offset read runs past the end.
      var container = fontWithMathTable();
      container = setTableOffset(container, 'MATH', container.length - 4);
      final font = load(container);
      expect(hasMathTable(font), isTrue);
      expect(() => mathConstants(font), throwsA(const BufferTooShort()));
      expect(
          () => italicCorrection(font, 1229), throwsA(const BufferTooShort()));
      expect(() => minConnectorOverlap(font), throwsA(const BufferTooShort()));
      expect(() => glyphConstruction(font, 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));
    });

    test('zero glyph-info offset reads as no correction', () {
      var container = fontWithMathTable();
      container = patchU16(container, tableOffset(container, 'MATH') + 6, 0);
      expect(italicCorrection(load(container), 1229), isNull);
    });

    test('zero italics offset reads as no correction', () {
      var container = fontWithMathTable();
      final (mathAt, _, glyphInfoOff, _) = mathLayout(container);
      container = patchU16(container, mathAt + glyphInfoOff, 0);
      expect(italicCorrection(load(container), 1229), isNull);
    });

    test('italics past the end throws BufferTooShort', () {
      var container = fontWithMathTable();
      final (mathAt, _, glyphInfoOff, _) = mathLayout(container);
      container = patchU16(container, mathAt + glyphInfoOff, 0x7000);
      expect(() => italicCorrection(load(container), 1229),
          throwsA(const BufferTooShort()));
    });

    test('italics offset word past the end throws BufferTooShort', () {
      var container = fontWithMathTable();
      final mathAt = tableOffset(container, 'MATH');
      container =
          patchU16(container, mathAt + 6, container.length - mathAt - 1);
      expect(() => italicCorrection(load(container), 1229),
          throwsA(const BufferTooShort()));
    });

    test('coverage disagreeing with its values throws', () {
      // Zero the value count while the coverage still finds glyph 1229:
      // a corrupt font, not a missing glyph.
      var container = fontWithMathTable();
      final (mathAt, _, glyphInfoOff, _) = mathLayout(container);
      final glyphInfo = mathAt + glyphInfoOff;
      final italics = glyphInfo + readU16(container, glyphInfo);
      container = patchU16(container, italics + 2, 0);
      expect(() => italicCorrection(load(container), 1229),
          throwsA(const BufferTooShort()));
    });

    test('unknown coverage format throws', () {
      var container = fontWithMathTable();
      final (mathAt, _, glyphInfoOff, _) = mathLayout(container);
      final glyphInfo = mathAt + glyphInfoOff;
      final italics = glyphInfo + readU16(container, glyphInfo);
      final coverage = italics + readU16(container, italics);
      container = patchU16(container, coverage, 7);
      expect(() => italicCorrection(load(container), 1229),
          throwsA(const BufferTooShort()));
    });

    test('italic value past the end throws BufferTooShort', () {
      // Widen the value count and push 1229's range index far out: the
      // coverage hit is legitimate, but its value is not in the file.
      var container = fontWithMathTable();
      final (mathAt, _, glyphInfoOff, _) = mathLayout(container);
      final glyphInfo = mathAt + glyphInfoOff;
      final italics = glyphInfo + readU16(container, glyphInfo);
      final coverage = italics + readU16(container, italics);
      final rangeCount = readU16(container, coverage + 2);
      var range = -1;
      for (var i = 0; i < rangeCount; i++) {
        final rec = coverage + 4 + i * 6;
        if (readU16(container, rec) <= 1229 &&
            1229 <= readU16(container, rec + 2)) {
          range = rec;
        }
      }
      expect(range, isNot(-1));
      container = patchU16(container, italics + 2, 0xFFFF);
      container = patchU16(container, range + 4, 0x7000);
      expect(() => italicCorrection(load(container), 1229),
          throwsA(const BufferTooShort()));
    });

    test('zero variants offset reads as no variants', () {
      var container = fontWithMathTable();
      container = patchU16(container, tableOffset(container, 'MATH') + 8, 0);
      final font = load(container);
      expect(minConnectorOverlap(font), 0);
      expect(glyphConstruction(font, 1064, StretchAxis.vertical), isNull);
    });

    test('variants past the end throw BufferTooShort', () {
      var container = fontWithMathTable();
      final mathAt = tableOffset(container, 'MATH');
      container = patchU16(container, mathAt + 8, 0x7FFF);
      final font = load(container);
      expect(() => minConnectorOverlap(font), throwsA(const BufferTooShort()));
      expect(() => glyphConstruction(font, 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));
    });

    test('zero vertical coverage reads as no construction', () {
      var container = fontWithMathTable();
      final (mathAt, _, _, variantsOff) = mathLayout(container);
      container = patchU16(container, mathAt + variantsOff + 2, 0);
      expect(glyphConstruction(load(container), 1064, StretchAxis.vertical),
          isNull);
    });

    test('construction count disagreeing with coverage reads as absent', () {
      // Zero the vertical count while the coverage still finds 1064:
      // safer to report "no construction" than to read past the array.
      var container = fontWithMathTable();
      final (mathAt, _, _, variantsOff) = mathLayout(container);
      container = patchU16(container, mathAt + variantsOff + 6, 0);
      expect(glyphConstruction(load(container), 1064, StretchAxis.vertical),
          isNull);
    });

    test('zero construction offset reads as no construction', () {
      var container = fontWithMathTable();
      final (mathAt, _, _, variantsOff) = mathLayout(container);
      final variants = mathAt + variantsOff;
      // parenleft is coverage index 4 (see the oracle probe).
      container = patchU16(container, variants + 10 + 4 * 2, 0);
      expect(glyphConstruction(load(container), 1064, StretchAxis.vertical),
          isNull);
    });

    test('huge variant count throws BufferTooShort', () {
      var container = fontWithMathTable();
      final (mathAt, _, _, variantsOff) = mathLayout(container);
      final variants = mathAt + variantsOff;
      final construction = variants + readU16(container, variants + 10 + 8);
      container = patchU16(container, construction + 2, 3000);
      expect(
          () => glyphConstruction(load(container), 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));
    });

    test('huge part count throws BufferTooShort', () {
      var container = fontWithMathTable();
      final (mathAt, _, _, variantsOff) = mathLayout(container);
      final variants = mathAt + variantsOff;
      final construction = variants + readU16(container, variants + 10 + 8);
      final assembly = construction + readU16(container, construction);
      container = patchU16(container, assembly + 4, 3000);
      expect(
          () => glyphConstruction(load(container), 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));
    });

    test('assembly past the end throws BufferTooShort', () {
      var container = fontWithMathTable();
      final (mathAt, _, _, variantsOff) = mathLayout(container);
      final variants = mathAt + variantsOff;
      final construction = variants + readU16(container, variants + 10 + 8);
      container = patchU16(container, construction, 0x7000);
      expect(
          () => glyphConstruction(load(container), 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));
    });

    test('bare display-operator-min-height past the end throws', () {
      var container = fontWithMathTable();
      final mathAt = tableOffset(container, 'MATH');
      container =
          patchU16(container, mathAt + 4, container.length - mathAt - 5);
      expect(() => mathConstants(load(container)),
          throwsA(const BufferTooShort()));
    });

    test('italics count past the end throws BufferTooShort', () {
      var container = fontWithMathTable();
      final (mathAt, _, glyphInfoOff, _) = mathLayout(container);
      final glyphInfo = mathAt + glyphInfoOff;
      container =
          patchU16(container, glyphInfo, container.length - 3 - glyphInfo);
      expect(() => italicCorrection(load(container), 1229),
          throwsA(const BufferTooShort()));
    });

    test('italics coverage past the end throws BufferTooShort', () {
      var container = fontWithMathTable();
      final (mathAt, _, glyphInfoOff, _) = mathLayout(container);
      final glyphInfo = mathAt + glyphInfoOff;
      final italics = glyphInfo + readU16(container, glyphInfo);
      container = patchU16(container, italics, 0x7000);
      expect(() => italicCorrection(load(container), 1229),
          throwsA(const BufferTooShort()));
    });
  });

  group('truncated subtables', () {
    test('each variants header field past the end throws', () {
      // variants at len-3/5/7: vertCoverage, horizCoverage, vertCount
      // each fail in turn while the earlier fields still read.
      for (final tail in [3, 5, 7]) {
        var container = fontWithMathTable();
        final mathAt = tableOffset(container, 'MATH');
        container =
            patchU16(container, mathAt + 8, container.length - mathAt - tail);
        expect(
            () =>
                glyphConstruction(load(container), 1064, StretchAxis.vertical),
            throwsA(const BufferTooShort()),
            reason: 'tail $tail');
      }
    });

    test('horizontal count past the end throws BufferTooShort', () {
      var container = fontWithMathTable();
      final mathAt = tableOffset(container, 'MATH');
      container =
          patchU16(container, mathAt + 8, container.length - mathAt - 9);
      // Plant a readable header: nonzero coverages, zero counts. The
      // horizontal count read then runs past the end.
      container =
          plant(container, container.length - 9, [0, 100, 0, 1, 0, 1, 0, 0]);
      expect(
          () => glyphConstruction(load(container), 746, StretchAxis.horizontal),
          throwsA(const BufferTooShort()));
    });

    test('construction entry past the end throws BufferTooShort', () {
      // A format-2 coverage awarding 1064 a huge index, with a count to
      // match: the construction-array read runs past the end.
      var container = fontWithMathTable();
      final mathAt = tableOffset(container, 'MATH');
      final variants = mathAt + readU16(container, mathAt + 8);
      container = plant(container, container.length - 12,
          [0, 2, 0, 1, 0x04, 0x28, 0x04, 0x28, 0x70, 0x00]);
      container =
          patchU16(container, variants + 2, container.length - 12 - variants);
      container = patchU16(container, variants + 6, 0xFFFF);
      expect(
          () => glyphConstruction(load(container), 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));
    });

    test('construction fields past the end throw one at a time', () {
      // Construction at len-1: assembly offset unreadable. At len-3:
      // assembly reads, variant count does not.
      for (final tail in [1, 3]) {
        var container = fontWithMathTable();
        final mathAt = tableOffset(container, 'MATH');
        final variants = mathAt + readU16(container, mathAt + 8);
        container = patchU16(
            container, variants + 10 + 8, container.length - tail - variants);
        expect(
            () =>
                glyphConstruction(load(container), 1064, StretchAxis.vertical),
            throwsA(const BufferTooShort()),
            reason: 'tail $tail');
      }
    });

    test('variant record fields past the end throw one at a time', () {
      // Construction at len-5: the record's glyph id is unreadable. At
      // len-7: the id reads, the advance does not.
      for (final tail in [5, 7]) {
        var container = fontWithMathTable();
        final mathAt = tableOffset(container, 'MATH');
        final variants = mathAt + readU16(container, mathAt + 8);
        final target = container.length - tail;
        container = patchU16(container, variants + 10 + 8, target - variants);
        // Assembly offset + variant count always fit; the record's glyph
        // id only fits when the tail allows it.
        container = plant(
            container, target, tail == 5 ? [0, 0, 0, 1] : [0, 0, 0, 1, 0, 7]);
        expect(
            () =>
                glyphConstruction(load(container), 1064, StretchAxis.vertical),
            throwsA(const BufferTooShort()),
            reason: 'tail $tail');
      }
    });

    test('assembly part count past the end throws BufferTooShort', () {
      var container = fontWithMathTable();
      final mathAt = tableOffset(container, 'MATH');
      final variants = mathAt + readU16(container, mathAt + 8);
      final construction = variants + readU16(container, variants + 10 + 8);
      container = patchU16(
          container, construction, container.length - 3 - construction);
      expect(
          () => glyphConstruction(load(container), 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));
    });

    test('part record fields past the end throw one at a time', () {
      // Assembly at len-7/9/11/13/15: glyph id, start connector, end
      // connector, full advance, flags each fail in turn.
      for (final tail in [7, 9, 11, 13, 15]) {
        var container = fontWithMathTable();
        final mathAt = tableOffset(container, 'MATH');
        final variants = mathAt + readU16(container, mathAt + 8);
        final construction = variants + readU16(container, variants + 10 + 8);
        final target = container.length - tail;
        container = patchU16(container, construction, target - construction);
        // Italics MathValue (value + device offset) then partCount 1.
        container = plant(container, target, [0, 0, 0, 0, 0, 1]);
        expect(
            () =>
                glyphConstruction(load(container), 1064, StretchAxis.vertical),
            throwsA(const BufferTooShort()),
            reason: 'tail $tail');
      }
    });

    test('format-1 coverage fields past the end throw', () {
      // Coverage at len-2: format reads, count does not. At len-12 with
      // five claimed entries: the fifth entry read runs past the end.
      var container = fontWithMathTable();
      var mathAt = tableOffset(container, 'MATH');
      var variants = mathAt + readU16(container, mathAt + 8);
      container = plant(container, container.length - 2, [0, 1]);
      container =
          patchU16(container, variants + 2, container.length - 2 - variants);
      expect(
          () => glyphConstruction(load(container), 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));

      container = fontWithMathTable();
      mathAt = tableOffset(container, 'MATH');
      variants = mathAt + readU16(container, mathAt + 8);
      container = plant(container, container.length - 12,
          [0, 1, 0, 5, 0x23, 0x28, 0x23, 0x29, 0x23, 0x2A, 0x23, 0x2B]);
      container =
          patchU16(container, variants + 2, container.length - 12 - variants);
      expect(
          () => glyphConstruction(load(container), 1064, StretchAxis.vertical),
          throwsA(const BufferTooShort()));
    });

    test('format-2 coverage fields past the end throw', () {
      // Coverage at len-3: format reads, range count does not. With a
      // huge range count over one short range: the second range read
      // runs past the end.
      var container = fontWithMathTable();
      var mathAt = tableOffset(container, 'MATH');
      var glyphInfoOff = readU16(container, mathAt + 6);
      var glyphInfo = mathAt + glyphInfoOff;
      var italics = glyphInfo + readU16(container, glyphInfo);
      container = plant(container, container.length - 3, [0, 2]);
      container = patchU16(container, italics, container.length - 3 - italics);
      expect(() => italicCorrection(load(container), 1229),
          throwsA(const BufferTooShort()));

      container = fontWithMathTable();
      mathAt = tableOffset(container, 'MATH');
      glyphInfoOff = readU16(container, mathAt + 6);
      glyphInfo = mathAt + glyphInfoOff;
      italics = glyphInfo + readU16(container, glyphInfo);
      container = plant(container, container.length - 10,
          [0, 2, 0xFF, 0xFF, 0x07, 0xD0, 0x07, 0xD0, 0, 0]);
      container = patchU16(container, italics, container.length - 10 - italics);
      expect(() => italicCorrection(load(container), 0),
          throwsA(const BufferTooShort()));
    });

    test('format-2 range fields past the end throw one at a time', () {
      // Range record at len-1/3/5: start, end, startIndex each fail in
      // turn while the earlier fields still read.
      for (final tail in [1, 3, 5]) {
        var container = fontWithMathTable();
        final mathAt = tableOffset(container, 'MATH');
        final glyphInfoOff = readU16(container, mathAt + 6);
        final glyphInfo = mathAt + glyphInfoOff;
        final italics = glyphInfo + readU16(container, glyphInfo);
        // Coverage header (format 2, one range) immediately precedes the
        // range record.
        final cov = container.length - tail - 4;
        container = plant(container, cov, [0, 2, 0, 1]);
        container = patchU16(container, italics, cov - italics);
        expect(() => italicCorrection(load(container), 1229),
            throwsA(const BufferTooShort()),
            reason: 'tail $tail');
      }
    });
  });

  group('MATH arguments', () {
    test('out-of-range glyph ids throw ArgumentError', () {
      final font = load(fontWithMathTable());
      expect(() => italicCorrection(font, -1), throwsArgumentError);
      expect(() => italicCorrection(font, 0x10000), throwsArgumentError);
      expect(() => glyphConstruction(font, -1, StretchAxis.vertical),
          throwsArgumentError);
      expect(() => glyphConstruction(font, 0x10000, StretchAxis.horizontal),
          throwsArgumentError);
    });
  });

  group('MATH value semantics', () {
    const constants = MathConstants(
      scriptPercentScaleDown: 70,
      scriptScriptPercentScaleDown: 55,
      delimitedSubFormulaMinHeight: 1325,
      displayOperatorMinHeight: 1800,
      mathLeading: 150,
      axisHeight: 258,
      accentBaseHeight: 480,
      fractionRuleThickness: 68,
      fractionNumeratorShiftUp: 585,
      fractionDenominatorShiftDown: 585,
      radicalRuleThickness: 68,
      radicalVerticalGap: 85,
      superscriptShiftUp: 360,
      subscriptShiftDown: 210,
    );

    test('MathConstants compares by value', () {
      expect(mathConstants(load(fontWithMathTable())), constants);
      expect(constants.hashCode, constants.hashCode);
      expect(constants.toString(), contains('258'));
      expect(
          constants,
          isNot(const MathConstants(
            scriptPercentScaleDown: 70,
            scriptScriptPercentScaleDown: 55,
            delimitedSubFormulaMinHeight: 1325,
            displayOperatorMinHeight: 1800,
            mathLeading: 150,
            axisHeight: 259,
            accentBaseHeight: 480,
            fractionRuleThickness: 68,
            fractionNumeratorShiftUp: 585,
            fractionDenominatorShiftDown: 585,
            radicalRuleThickness: 68,
            radicalVerticalGap: 85,
            superscriptShiftUp: 360,
            subscriptShiftDown: 210,
          )));
      expect(constants, isNot('not constants'));
    });

    test('GlyphVariant compares by value', () {
      const a = GlyphVariant(glyphId: 1064, advance: 933);
      expect(a, const GlyphVariant(glyphId: 1064, advance: 933));
      expect(
          a.hashCode, const GlyphVariant(glyphId: 1064, advance: 933).hashCode);
      expect(a.toString(), contains('1064'));
      expect(a, isNot(const GlyphVariant(glyphId: 1064, advance: 934)));
      expect(a, isNot(const GlyphVariant(glyphId: 1065, advance: 933)));
    });

    test('AssemblyPart compares by value', () {
      const a = AssemblyPart(
          glyphId: 4862,
          startConnector: 0,
          endConnector: 250,
          fullAdvance: 1273,
          isExtender: false);
      expect(
          a,
          const AssemblyPart(
              glyphId: 4862,
              startConnector: 0,
              endConnector: 250,
              fullAdvance: 1273,
              isExtender: false));
      expect(
          a.hashCode,
          const AssemblyPart(
                  glyphId: 4862,
                  startConnector: 0,
                  endConnector: 250,
                  fullAdvance: 1273,
                  isExtender: false)
              .hashCode);
      expect(a.toString(), contains('4862'));
      expect(
          a,
          isNot(const AssemblyPart(
              glyphId: 4862,
              startConnector: 0,
              endConnector: 250,
              fullAdvance: 1273,
              isExtender: true)));
    });

    test('GlyphConstruction compares by value', () {
      GlyphConstruction build(
              {int correction = 0, int variantCount = 1, int partCount = 1}) =>
          GlyphConstruction(
            variants: List.generate(variantCount,
                (i) => GlyphVariant(glyphId: 1064 + i, advance: 933 + i * 100)),
            assembly: List.generate(
                partCount,
                (i) => AssemblyPart(
                    glyphId: 4862 + i,
                    startConnector: 0,
                    endConnector: 250,
                    fullAdvance: 1273,
                    isExtender: false)),
            assemblyItalicsCorrection: correction,
          );

      expect(build(), build());
      expect(build().hashCode, build().hashCode);
      expect(build().toString(), contains('1 variants'));
      // Each inequality axis: correction, variant length, variant
      // element, assembly length, assembly element, and foreign type.
      expect(build(), isNot(build(correction: 1)));
      expect(build(), isNot(build(variantCount: 2)));
      expect(
          build(),
          isNot(GlyphConstruction(
            variants: const [GlyphVariant(glyphId: 1, advance: 2)],
            assembly: build().assembly,
            assemblyItalicsCorrection: 0,
          )));
      expect(build(), isNot(build(partCount: 2)));
      expect(
          build(),
          isNot(GlyphConstruction(
            variants: build().variants,
            assembly: const [
              AssemblyPart(
                  glyphId: 1,
                  startConnector: 2,
                  endConnector: 3,
                  fullAdvance: 4,
                  isExtender: true)
            ],
            assemblyItalicsCorrection: 0,
          )));
      expect(build(), isNot('not a construction'));
    });
  });
}
