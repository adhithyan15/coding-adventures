/// Unit tests for `coding_adventures_font_parser`: the 28 reference tests
/// (`code/packages/rust/font-parser/src/lib.rs`, tests module) plus the
/// extras this lane needs — the `FontFile.table`/`indexToLocFormat`/
/// `numGlyphs` surface the reference never tests, the error and value
/// semantics, the `ArgumentError` matrix, and one test per hostile branch
/// (truncated tables, disagreeing offsets, corrupt subtables) so the 95%
/// coverage gate measures a suite that actually walks the failure paths.
library;

import 'dart:typed_data';

import 'package:coding_adventures_font_parser/font_parser.dart';
import 'package:coding_adventures_font_parser/src/font_parser.dart' as fontlib;
import 'package:test/test.dart';

import 'font_fixtures.dart';

void main() {
  group('load', () {
    test('empty buffer throws BufferTooShort', () {
      expect(() => load(Uint8List(0)), throwsA(const BufferTooShort()));
    });

    test('wrong magic throws InvalidMagic', () {
      final buf = Uint8List(12 + 16);
      buf.setAll(0, [0xDE, 0xAD, 0xBE, 0xEF]);
      expect(() => load(buf), throwsA(const InvalidMagic()));
    });

    test('Inter Regular loads', () {
      expect(load(interRegular()), isA<FontFile>());
    });

    test('short buffer under 12 bytes throws BufferTooShort', () {
      expect(() => load(Uint8List.fromList([0, 1, 0, 0, 0])),
          throwsA(const BufferTooShort()));
    });

    test('missing required table throws TableNotFound naming it', () {
      // A valid magic with zero table records: head is the first miss.
      final buf = Uint8List(12);
      buf.setAll(0, [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
      expect(() => load(buf), throwsA(const TableNotFound('head')));
    });

    test('wrong head magic throws InvalidHeadMagic', () {
      final font = buildSyntheticFontWithKern(const []);
      final bad = patchU32(font, tableOffset(font, 'head') + 12, 0xDEADBEEF);
      expect(() => load(bad), throwsA(const InvalidHeadMagic()));
    });

    test('head magic past the end throws BufferTooShort', () {
      final font = buildSyntheticFontWithKern(const []);
      // Point head at the last byte: +12 is unreadable.
      final bad = setTableOffset(font, 'head', font.length - 1);
      expect(() => load(bad), throwsA(const BufferTooShort()));
    });

    test('truncated directory record reads as table absent', () {
      // numTables claims 6 but the buffer ends mid-record: the reference
      // indexes the offset field unchecked and panics here; this port
      // fails closed with TableNotFound.
      final font = buildSyntheticFontWithKern(const []);
      final cut = (Uint8List.fromList(font)).sublist(0, 12 + 16 + 8);
      expect(() => load(cut), throwsA(isA<TableNotFound>()));
    });

    test('OTTO magic loads like TrueType magic', () {
      final font = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      final otf = patchU32(font, 0, 0x4F54544F);
      final loaded = load(otf);
      expect(kerning(loaded, 1, 2), -140);
    });
  });

  group('fontMetrics', () {
    test('unitsPerEm is 2048', () {
      final m = fontMetrics(load(interRegular()));
      expect(m.unitsPerEm, 2048);
    });

    test('family name is Inter', () {
      final m = fontMetrics(load(interRegular()));
      expect(m.familyName, 'Inter');
    });

    test('subfamily name is Regular', () {
      final m = fontMetrics(load(interRegular()));
      expect(m.subfamilyName, 'Regular');
    });

    test('numGlyphs is nonzero', () {
      final m = fontMetrics(load(interRegular()));
      expect(m.numGlyphs, greaterThan(100));
    });

    test('ascender is positive', () {
      final m = fontMetrics(load(interRegular()));
      expect(m.ascender, greaterThan(0));
    });

    test('descender is negative or zero', () {
      final m = fontMetrics(load(interRegular()));
      expect(m.descender, lessThanOrEqualTo(0));
    });

    test('xHeight is some and positive', () {
      final m = fontMetrics(load(interRegular()));
      expect(m.xHeight, isNotNull);
      expect(m.xHeight!, greaterThan(0));
    });

    test('capHeight is some and positive', () {
      final m = fontMetrics(load(interRegular()));
      expect(m.capHeight, isNotNull);
      expect(m.capHeight!, greaterThan(0));
    });

    test('synthetic font reads its hhea values and unknown names', () {
      final m =
          fontMetrics(load(buildSyntheticFontWithKern(const [(1, 2, -140)])));
      expect(m.unitsPerEm, 1000);
      expect(m.ascender, 800);
      expect(m.descender, -200);
      expect(m.lineGap, 0);
      expect(m.numGlyphs, 5);
      // No name table: the documented fallback.
      expect(m.familyName, '(unknown)');
      expect(m.subfamilyName, '(unknown)');
      // No OS/2 table: hhea stands in, heights are null.
      expect(m.xHeight, isNull);
      expect(m.capHeight, isNull);
    });

    test('unreadable fields fall back instead of throwing', () {
      // Head keeps a readable magic but loses unitsPerEm; hhea and maxp
      // point past the end. fontMetrics must answer with defaults.
      var font = buildSyntheticFontWithKern(const []);
      // head+12 (magic) readable, head+18 (unitsPerEm) past the end.
      font = setTableOffset(font, 'head', font.length - 16);
      // Replant the magic at the new head+12 (the buffer's last 4 bytes).
      final out = Uint8List.fromList(font);
      out.setAll(out.length - 4, [0x5F, 0x0F, 0x3C, 0xF5]);
      font = Uint8List.fromList(out);
      font = setTableOffset(font, 'hhea', font.length - 5);
      font = setTableOffset(font, 'maxp', font.length - 5);
      final m = fontMetrics(load(font));
      expect(m.unitsPerEm, 1000);
      expect(m.ascender, 0);
      expect(m.descender, 0);
      expect(m.lineGap, 0);
      expect(m.numGlyphs, 0);
    });
  });

  group('glyphId', () {
    test('latin A maps to some glyph', () {
      final font = load(interRegular());
      expect(glyphId(font, 0x0041), isNotNull);
    });

    test('codepoint above FFFF is null', () {
      final font = load(interRegular());
      expect(glyphId(font, 0x10000), isNull);
    });

    test('unmapped high codepoint does not throw', () {
      // U+FFFF is a non-character; the sentinel segment resolves to glyph
      // 0, which reads as unmapped. Either way it must not throw.
      final font = load(interRegular());
      expect(() => glyphId(font, 0xFFFF), returnsNormally);
    });

    test('A and V map to different glyphs', () {
      final font = load(interRegular());
      expect(glyphId(font, 0x0041), isNot(equals(glyphId(font, 0x0056))));
    });

    test('space maps to some glyph', () {
      final font = load(interRegular());
      expect(glyphId(font, 0x0020), isNotNull);
    });

    test('synthetic sentinel-only cmap maps nothing', () {
      final font = load(buildSyntheticFontWithKern(const []));
      // Every BMP codepoint lands in the gap before the sentinel — except
      // U+FFFF itself, which resolves to glyph 0 (.notdef → null).
      expect(glyphId(font, 0x0041), isNull);
      expect(glyphId(font, 0xFFFF), isNull);
    });

    test('codepoint past all endCodes is null', () {
      // A single segment ending at 0x00FF: anything above misses every
      // segment (lo >= segCount).
      var font = buildSyntheticFontWithKern(const []);
      final cmapAt = tableOffset(font, 'cmap');
      font = patchU16(font, cmapAt + 12 + 14, 0x00FF); // endCode[0]
      font = patchU16(font, cmapAt + 12 + 18, 0x0000); // startCode[0]
      expect(glyphId(load(font), 0x0100), isNull);
    });

    test('non-Format-4 subtable is null', () {
      var font = buildSyntheticFontWithKern(const []);
      font = patchU16(font, tableOffset(font, 'cmap') + 12, 6);
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('unlisted platform falls back to platform 0', () {
      // Rewrite the encoding record to platform 0 / encoding 3: still
      // accepted as the fallback, still Format 4.
      var font = buildSyntheticFontWithKern(const []);
      final cmapAt = tableOffset(font, 'cmap');
      font = patchU16(font, cmapAt + 4, 0);
      font = patchU16(font, cmapAt + 6, 3);
      final loaded = load(font);
      expect(glyphId(loaded, 0x0041), isNull); // sentinel-only, but parsed
      expect(glyphId(loaded, 0xFFFF), isNull);
    });

    test('no usable subtable is null', () {
      var font = buildSyntheticFontWithKern(const []);
      final cmapAt = tableOffset(font, 'cmap');
      font = patchU16(font, cmapAt + 4, 1); // Macintosh: neither 3/1 nor 0
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('truncated cmap index is null, not a throw', () {
      var font = buildSyntheticFontWithKern(const []);
      font = setTableOffset(font, 'cmap', font.length - 1);
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('truncated encoding record is null', () {
      // cmap body keeps a 4-byte header (one subtable) but the 8-byte
      // record runs past the end: the scan must give up, not throw.
      var font = buildSyntheticFontWithKern(const []);
      font = setTableOffset(font, 'cmap', font.length - 6);
      final out = Uint8List.fromList(font);
      out.setAll(out.length - 6, [0, 0, 0, 1]);
      font = Uint8List.fromList(out);
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('idRangeOffset branch resolves through glyphIdArray', () {
      // Rewrite the synthetic segment to indirect lookup: start 0x0041,
      // end 0x0041, idRangeOffset 2. With segCount=1 the offset word
      // lives at sub+22, so the entry address is sub+24 — two bytes
      // into whatever body follows cmap. The point is the addressing,
      // not the value: it must read exactly there.
      var font = buildSyntheticFontWithKern(const []);
      final cmapAt = tableOffset(font, 'cmap');
      const sub = 12; // subtable starts 12 into the cmap body
      font = patchU16(font, cmapAt + sub + 14, 0x0041); // endCode[0]
      font = patchU16(font, cmapAt + sub + 18, 0x0041); // startCode[0]
      font = patchU16(font, cmapAt + sub + 22, 2); // idRangeOffset[0]
      final entry = cmapAt + sub + 24;
      final expected = (font[entry] << 8) | font[entry + 1];
      expect(expected, isNot(0)); // the test needs a nonzero witness
      expect(glyphId(load(font), 0x0041), expected);
    });

    test('idRangeOffset past the end is null', () {
      var font = buildSyntheticFontWithKern(const []);
      final cmapAt = tableOffset(font, 'cmap');
      const sub = 12;
      font = patchU16(font, cmapAt + sub + 14, 0x0041);
      font = patchU16(font, cmapAt + sub + 18, 0x0041);
      font = patchU16(font, cmapAt + sub + 22, 0x7FFF);
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('out-of-range codepoint throws ArgumentError', () {
      final font = load(buildSyntheticFontWithKern(const []));
      expect(() => glyphId(font, -1), throwsArgumentError);
      expect(() => glyphId(font, 0x100000000), throwsArgumentError);
    });

    test('encoding record fields past the end read null one at a time', () {
      // cmap at len-5: platform unreadable. At len-9: platform and
      // encoding read, the subtable offset does not. (len-6 covers the
      // encoding-only case in the truncated-record test above.)
      for (final tail in [5, 9]) {
        var font = buildSyntheticFontWithKern(const []);
        font = setTableOffset(font, 'cmap', font.length - tail);
        font = plant(font, font.length - tail, [0, 0, 0, 1]);
        expect(glyphId(load(font), 0x0041), isNull, reason: 'tail $tail');
      }
    });

    test('platform 3 with another encoding is not accepted', () {
      var font = buildSyntheticFontWithKern(const []);
      final cmapAt = tableOffset(font, 'cmap');
      font = patchU16(font, cmapAt + 4, 3);
      font = patchU16(font, cmapAt + 6, 2); // platform 3, encoding 2
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('second platform-0 record does not displace the first', () {
      // Two encoding records, both platform 0: the first still wins the
      // fallback slot and the second is skipped.
      var font = buildSyntheticFontWithKern(const []);
      final cmapAt = tableOffset(font, 'cmap');
      font = patchU16(font, cmapAt + 2, 2); // two records
      font = patchU16(font, cmapAt + 4, 0); // record 1: platform 0
      font = patchU16(font, cmapAt + 12, 0); // record 2: platform 0
      // Record 1's subtable (now format 0) is unusable either way.
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('segment count past the end is null', () {
      var font = buildSyntheticFontWithKern(const []);
      final cmapAt = tableOffset(font, 'cmap');
      // Subtable at len-2: format reads, segCountX2 does not. (The
      // subtable offset is a u32: patching only its first half would
      // point light-years past the end and fail at the format read.)
      font = patchU32(font, cmapAt + 8, font.length - 2 - cmapAt);
      font = plant(font, font.length - 2, [0, 4]);
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('segment entries past the end are null', () {
      // A huge segCountX2 over a one-segment subtable: the binary search
      // probes past the end and gives up instead of throwing.
      var font = buildSyntheticFontWithKern(const []);
      font = patchU16(font, tableOffset(font, 'cmap') + 12 + 6, 0x7FFE);
      expect(glyphId(load(font), 0x0041), isNull);
    });

    test('delta words past the end are null one at a time', () {
      // Mini cmaps with endCode == startCode == 0x41: the first runs out
      // at idDelta, the second fits idDelta but not idRangeOffset.
      for (final tail in [32, 34]) {
        var font = buildSyntheticFontWithKern(const []);
        font = setTableOffset(font, 'cmap', font.length - tail);
        final head = [
          0, 0, 0, 1, // version, numSubtables
          0, 3, 0, 1, 0, 0, 0, 12, // platform 3, encoding 1, offset 12
          0, 4, 0, 24, 0, 0, 0, 2, // format 4, length, language, segCountX2
          0, 2, 0, 0, 0, 0, // searchRange, entrySelector, rangeShift
          0, 0x41, 0, 0, 0, 0x41, // endCode[0], reservedPad, startCode[0]
        ];
        font = plant(font, font.length - tail, head);
        if (tail == 34) {
          font = plant(font, font.length - 2, [0, 1]); // idDelta
        }
        expect(glyphId(load(font), 0x0041), isNull, reason: 'tail $tail');
      }
    });

    test('startCode past the end is null', () {
      // A mini cmap planted at len-30: header, one platform 3/1 record,
      // and a subtable whose endCode reads but whose startCode does not.
      var font = buildSyntheticFontWithKern(const []);
      font = setTableOffset(font, 'cmap', font.length - 30);
      font = plant(font, font.length - 30, [
        0, 0, 0, 1, // version, numSubtables
        0, 3, 0, 1, 0, 0, 0, 12, // platform 3, encoding 1, offset 12
        0, 4, 0, 24, 0, 0, 0, 2, // format 4, length, language, segCountX2
        0, 2, 0, 0, 0, 0, // searchRange, entrySelector, rangeShift
        0xFF, 0xFF, 0, 0, // endCode[0], reservedPad
      ]);
      expect(glyphId(load(font), 0x0041), isNull);
    });
  });

  group('glyphMetrics', () {
    test('A has positive advance', () {
      final font = load(interRegular());
      final gm = glyphMetrics(font, glyphId(font, 0x0041)!);
      expect(gm!.advanceWidth, greaterThan(0));
    });

    test('out of range returns null', () {
      final font = load(interRegular());
      final m = fontMetrics(font);
      expect(glyphMetrics(font, m.numGlyphs), isNull);
    });

    test('advance is within reasonable bounds', () {
      final font = load(interRegular());
      final gm = glyphMetrics(font, glyphId(font, 0x0041)!)!;
      expect(gm.advanceWidth, inInclusiveRange(100, 2400));
    });

    test('synthetic full record reads advance and lsb', () {
      final font = load(buildSyntheticFontWithKern(const []));
      expect(glyphMetrics(font, 2),
          const GlyphMetrics(advanceWidth: 600, leftSideBearing: 50));
    });

    test('trailing glyphs share the last advance', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        numGlyphs: 7,
        numHMetrics: 5,
        trailingLsb: [-10, 20],
      ));
      expect(glyphMetrics(font, 5),
          const GlyphMetrics(advanceWidth: 600, leftSideBearing: -10));
      expect(glyphMetrics(font, 6),
          const GlyphMetrics(advanceWidth: 600, leftSideBearing: 20));
    });

    test('zero hMetrics with trailing glyphs is null, not a throw', () {
      // numberOfHMetrics 0 and numGlyphs 1: glyph 0 takes the shared
      // branch, which has no "last" advance to share. The reference
      // underflows `num_h_metrics - 1` here (debug panic); this port
      // answers null.
      final font = load(buildSyntheticFontWithKern(
        const [],
        numGlyphs: 1,
        numHMetrics: 0,
      ));
      expect(glyphMetrics(font, 0), isNull);
    });

    test('unreadable maxp or hhea is null', () {
      var font = buildSyntheticFontWithKern(const []);
      font = setTableOffset(font, 'maxp', font.length - 5);
      expect(glyphMetrics(load(font), 0), isNull);
      var font2 = buildSyntheticFontWithKern(const []);
      font2 = setTableOffset(font2, 'hhea', font2.length - 5);
      expect(glyphMetrics(load(font2), 0), isNull);
    });

    test('truncated hmtx record is null', () {
      // hmtx at len-3: advance reads, lsb does not. At len-1: the
      // advance itself is unreadable.
      for (final tail in [3, 1]) {
        var font = buildSyntheticFontWithKern(const []);
        font = setTableOffset(font, 'hmtx', font.length - tail);
        expect(glyphMetrics(load(font), 0), isNull, reason: 'tail $tail');
      }
    });

    test('shared branch with unreadable last advance is null', () {
      var font = buildSyntheticFontWithKern(
        const [],
        numGlyphs: 6,
        numHMetrics: 5,
        trailingLsb: [11],
      );
      font = setTableOffset(font, 'hmtx', font.length - 3);
      expect(glyphMetrics(load(font), 5), isNull);
    });

    test('truncated trailing lsb is null', () {
      // Replant hmtx at len-21 with five full records: the shared
      // advance (at +16) reads, the trailing lsb (at +20) does not.
      var font = buildSyntheticFontWithKern(
        const [],
        numGlyphs: 6,
        numHMetrics: 5,
        trailingLsb: [11],
      );
      font = setTableOffset(font, 'hmtx', font.length - 21);
      final records = <int>[];
      for (var i = 0; i < 5; i++) {
        records.addAll([0x02, 0x58, 0, 50]); // advance 600, lsb 50
      }
      font = plant(font, font.length - 21, records);
      expect(glyphMetrics(load(font), 5), isNull);
    });

    test('out-of-range glyph id throws ArgumentError', () {
      final font = load(buildSyntheticFontWithKern(const []));
      expect(() => glyphMetrics(font, -1), throwsArgumentError);
      expect(() => glyphMetrics(font, 0x10000), throwsArgumentError);
    });
  });

  group('kerning', () {
    test('Inter has no kern table so pairs read zero', () {
      // Inter v4 kerns via GPOS, out of scope here; the legacy table is
      // absent and 0 is the correct answer.
      final font = load(interRegular());
      final a = glyphId(font, 0x0041)!;
      final v = glyphId(font, 0x0056)!;
      expect(kerning(font, a, v), 0);
    });

    test('synthetic pair is found', () {
      final font =
          load(buildSyntheticFontWithKern(const [(1, 2, -140), (3, 4, 80)]));
      expect(kerning(font, 1, 2), -140);
    });

    test('synthetic second pair is found', () {
      final font =
          load(buildSyntheticFontWithKern(const [(1, 2, -140), (3, 4, 80)]));
      expect(kerning(font, 3, 4), 80);
    });

    test('absent pair returns zero', () {
      final font =
          load(buildSyntheticFontWithKern(const [(1, 2, -140), (3, 4, 80)]));
      expect(kerning(font, 1, 4), 0);
    });

    test('invalid glyph ids on a kern-less font read zero', () {
      final font = load(interRegular());
      expect(kerning(font, 0, 0), 0);
    });

    test('unsorted input still searches correctly', () {
      // The builder sorts by composite key; reversing the input must not
      // matter to the binary search.
      final font =
          load(buildSyntheticFontWithKern(const [(3, 4, 80), (1, 2, -140)]));
      expect(kerning(font, 1, 2), -140);
      expect(kerning(font, 3, 4), 80);
    });

    test('non-Format-0 subtable is skipped', () {
      var font = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      // coverage 0x0101: format 1, horizontal. The pair table is still
      // there, but a Format-0-only reader must not consult it.
      font = patchU16(font, tableOffset(font, 'kern') + 8, 0x0101);
      expect(kerning(load(font), 1, 2), 0);
    });

    test('unreadable kern header reads zero', () {
      var font = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      font = setTableOffset(font, 'kern', font.length - 3);
      expect(kerning(load(font), 1, 2), 0);
    });

    test('second subtable with unreadable nPairs stops the walk', () {
      // Drop maxp, claim two subtables, shrink the first length to 14:
      // the "second" subtable starts with exactly 6 bytes left, its
      // coverage reads Format 0, and its nPairs read fails — the walk
      // stops with 0 instead of throwing.
      var font = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      final kernAt = tableOffset(font, 'kern');
      font = (Uint8List.fromList(font)).sublist(0, kernAt + 24);
      font = patchU16(font, kernAt + 2, 2);
      font = patchU16(font, kernAt + 6, 14);
      font = patchU16(font, kernAt + 22, 0x0001); // Format 0 coverage
      expect(kerning(load(font), 9, 9), 0);
    });

    test('subtable walk past the end stops', () {
      // Three claimed subtables: the first misses, the second (maxp
      // bytes) is skipped as non-Format-0 with a huge length, and the
      // third starts past the end — the walk stops with 0.
      var font = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      final kernAt = tableOffset(font, 'kern');
      font = patchU16(font, kernAt + 2, 3);
      font = patchU16(font, kernAt + 28, 0x0100); // non-zero format
      expect(kerning(load(font), 9, 9), 0);
    });

    test('unreadable kern value reads zero', () {
      // Truncate so the matched pair's value bytes fall past the end
      // while left/right still read: the pair is found with value 0.
      var font = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      final kernAt = tableOffset(font, 'kern');
      font = (Uint8List.fromList(font)).sublist(0, kernAt + 14 + 5);
      expect(kerning(load(font), 1, 2), 0);
    });

    test('split pair entry stops the search', () {
      // Truncate mid-entry: the left glyph reads, the right does not.
      // The search stops with 0 instead of throwing.
      var font = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      final kernAt = tableOffset(font, 'kern');
      font = (Uint8List.fromList(font)).sublist(0, kernAt + 18 + 3);
      expect(kerning(load(font), 1, 2), 0);
    });

    test('truncated pair entry stops the search', () {
      var font = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      // Claim 300 pairs in a one-pair table: the search walks past the
      // end and must stop, not throw. (Whether it finds (1,2) first is
      // down to midpoint luck; zeros past the real pair compare less.)
      final kernAt = tableOffset(font, 'kern');
      font = patchU16(font, kernAt + 6, 300);
      expect(() => kerning(load(font), 1, 2), returnsNormally);
    });

    test('out-of-range glyph ids throw ArgumentError', () {
      final font = load(buildSyntheticFontWithKern(const [(1, 2, -140)]));
      expect(() => kerning(font, -1, 0), throwsArgumentError);
      expect(() => kerning(font, 0, 0x10000), throwsArgumentError);
    });
  });

  group('name table', () {
    test('Windows records win outright', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (0, 3, 1, utf16be('Fallback')),
          (3, 1, 1, utf16be('Test')),
          (3, 1, 2, utf16be('Bold')),
        ],
      ));
      final m = fontMetrics(font);
      expect(m.familyName, 'Test');
      expect(m.subfamilyName, 'Bold');
    });

    test('first platform-0 record is the fallback', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (0, 4, 1, utf16be('First')),
          (0, 4, 1, utf16be('Second')),
          (0, 4, 2, utf16be('Style')),
        ],
      ));
      final m = fontMetrics(font);
      expect(m.familyName, 'First');
      expect(m.subfamilyName, 'Style');
    });

    test('surrogate pairs decode to astral characters', () {
      // U+1D453 MATHEMATICAL ITALIC SMALL F: D835 DC53.
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 1, 1, [0x00, 0x41, 0xD8, 0x35, 0xDC, 0x53]),
        ],
      ));
      // Rune-exact: 'A' followed by U+1D453 (astral literals are
      // avoided; they mangle in transit through tooling).
      expect(fontMetrics(font).familyName.runes.toList(), [0x41, 0x1D453]);
    });

    test('unpaired surrogates become U+FFFD', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 1, 1, [0xD8, 0x34, 0x00, 0x41]), // high, then plain A
          (3, 1, 2, [0xDD, 0x53]), // lone low
        ],
      ));
      final m = fontMetrics(font);
      expect(m.familyName, '\u{FFFD}A');
      expect(m.subfamilyName, '\u{FFFD}');
    });

    test('trailing high surrogate at the end becomes U+FFFD', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 1, 1, [0x00, 0x42, 0xD8, 0x34]),
        ],
      ));
      expect(fontMetrics(font).familyName, 'B\u{FFFD}');
    });

    test('trailing odd byte is dropped', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 1, 1, [0x00, 0x43, 0xFF]),
        ],
      ));
      expect(fontMetrics(font).familyName, 'C');
    });

    test('absent nameID reads unknown', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 1, 0, utf16be('Copyright')),
        ],
      ));
      final m = fontMetrics(font);
      expect(m.familyName, '(unknown)');
      expect(m.subfamilyName, '(unknown)');
    });

    test('name records for other platforms are ignored', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (1, 0, 1, utf16be('Mac')),
        ],
      ));
      expect(fontMetrics(font).familyName, '(unknown)');
    });

    test('truncated name table reads unknown, not a throw', () {
      var font = buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 1, 1, utf16be('Test')),
        ],
      );
      font = setTableOffset(font, 'name', font.length - 5);
      expect(fontMetrics(load(font)).familyName, '(unknown)');
    });

    test('string bytes past the end read unknown', () {
      var font = buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 1, 1, utf16be('Test')),
        ],
      );
      // Point the first record's string offset far past the end.
      final nameAt = tableOffset(font, 'name');
      font = patchU16(font, nameAt + 6 + 10, 0x7FFF);
      expect(fontMetrics(load(font)).familyName, '(unknown)');
    });

    test('truncated record mid-scan reads unknown', () {
      var font = buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 1, 0, utf16be('Copyright')),
        ],
      );
      // Claim 200 records in a one-record table whose record does not
      // match: the scan runs past the end and must give up, not throw.
      font = patchU16(font, tableOffset(font, 'name') + 2, 200);
      expect(fontMetrics(load(font)).familyName, '(unknown)');
    });

    test('name record fields past the end read unknown one at a time', () {
      // name at len-7/9/13/15/17: platform, encoding, nameID, length,
      // string offset each fail in turn while the earlier fields read.
      // A further font at len-3 fails the count read itself.
      for (final tail in [7, 9, 13, 15, 17, 3]) {
        // Rewrite the name table's directory offset to the tail and plant
        // a header (format 0, count 1, stringOffset 0): the record scan
        // then fails at successive fields.
        var font = buildSyntheticFontWithKern(
          const [],
          nameStrings: [
            (3, 1, 0, utf16be('Copyright')),
          ],
        );
        final base = font.length - tail;
        font = setTableOffset(font, 'name', base);
        if (tail == 3) {
          // No room for the header either: count itself is unreadable.
          expect(fontMetrics(load(font)).familyName, '(unknown)',
              reason: 'tail $tail');
          continue;
        }
        font = plant(font, base, [0, 0, 0, 1, 0, 0]);
        expect(fontMetrics(load(font)).familyName, '(unknown)',
            reason: 'tail $tail');
      }
    });

    test('platform 3 with another encoding falls through', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        nameStrings: [
          (3, 2, 1, utf16be('No')),
          (3, 1, 1, utf16be('Yes')),
        ],
      ));
      expect(fontMetrics(font).familyName, 'Yes');
    });
  });

  group('OS/2', () {
    test('typographic values are preferred over hhea', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        os2: (4, 900, -300, 50, 500, 700),
      ));
      final m = fontMetrics(font);
      expect(m.ascender, 900);
      expect(m.descender, -300);
      expect(m.lineGap, 50);
      expect(m.xHeight, 500);
      expect(m.capHeight, 700);
    });

    test('version below 2 leaves heights null', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        os2: (1, 900, -300, 50, 500, 700),
      ));
      final m = fontMetrics(font);
      expect(m.ascender, 900);
      expect(m.xHeight, isNull);
      expect(m.capHeight, isNull);
    });

    test('unreadable OS/2 falls back to hhea', () {
      var font = buildSyntheticFontWithKern(
        const [],
        os2: (4, 900, -300, 50, 500, 700),
      );
      font = setTableOffset(font, 'OS/2', font.length - 1);
      final m = fontMetrics(load(font));
      expect(m.ascender, 800);
      expect(m.descender, -200);
      expect(m.lineGap, 0);
      expect(m.xHeight, isNull);
      expect(m.capHeight, isNull);
    });

    test('readable version with truncated heights leaves them null', () {
      var font = buildSyntheticFontWithKern(
        const [],
        os2: (4, 900, -300, 50, 500, 700),
      );
      // Replant: version 2 readable, typo fields readable garbage,
      // xHeight/capHeight past the end.
      font = setTableOffset(font, 'OS/2', font.length - 87);
      final out = Uint8List.fromList(font);
      out[out.length - 87] = 0;
      out[out.length - 86] = 2;
      font = Uint8List.fromList(out);
      final m = fontMetrics(load(font));
      expect(m.xHeight, isNull);
      expect(m.capHeight, isNull);
    });
  });

  group('byte readers', () {
    test('readU16 is big-endian', () {
      expect(fontlib.readU16([0x08, 0x00], 0), 0x0800);
    });

    test('readI16 reinterprets the sign', () {
      expect(fontlib.readI16([0xFE, 0x00], 0), -512);
      expect(fontlib.readI16([0x7F, 0xFF], 0), 32767);
    });

    test('short reads are null', () {
      expect(fontlib.readU16([0x08], 0), isNull);
      expect(fontlib.readI16([0x08], 0), isNull);
      expect(fontlib.readU32([0x00, 0x01, 0x00], 0), isNull);
      expect(fontlib.readU16([0x08, 0x00], -1), isNull);
      expect(fontlib.readU32([0x00, 0x01, 0x00, 0x00], -4), isNull);
    });

    test('readU32 is big-endian', () {
      expect(fontlib.readU32([0x00, 0x01, 0x00, 0x00], 0), 0x00010000);
    });
  });

  group('FontFile surface', () {
    test('data returns an equal, detached copy', () {
      final bytes = buildSyntheticFontWithKern(const [(1, 2, -140)]);
      final font = load(bytes);
      final data = font.data;
      expect(data, bytes);
      data[0] = 0xFF;
      // The font still loads its metrics: the copy is detached.
      expect(fontMetrics(font).unitsPerEm, 1000);
    });

    test('table returns the head bytes', () {
      final font = load(interRegular());
      final head = font.table('head'.codeUnits)!;
      final magic =
          (head[12] << 24) | (head[13] << 16) | (head[14] << 8) | head[15];
      expect(magic, 0x5F0F3CF5);
    });

    test('absent table is null', () {
      // Inter is a text font: no MATH table.
      final font = load(interRegular());
      expect(font.table('MATH'.codeUnits), isNull);
    });

    test('table range past the end is null', () {
      var font = buildSyntheticFontWithKern(const []);
      // Rewrite the head record's length to something enormous.
      final out = Uint8List.fromList(font);
      final numTables = (out[4] << 8) | out[5];
      for (var i = 0; i < numTables; i++) {
        final rec = 12 + i * 16;
        if (out[rec] == 0x68 /* h */ &&
            out[rec + 1] == 0x65 &&
            out[rec + 2] == 0x61 &&
            out[rec + 3] == 0x64) {
          out[rec + 12] = 0x7F;
          out[rec + 13] = 0xFF;
          out[rec + 14] = 0xFF;
          out[rec + 15] = 0xFF;
        }
      }
      font = Uint8List.fromList(out);
      expect(load(font).table('head'.codeUnits), isNull);
    });

    test('truncated directory mid-scan is null', () {
      // Enough bytes to load (all six records intact) is required, so
      // instead claim extra tables: the scan walks past the end.
      var font = buildSyntheticFontWithKern(const []);
      font = patchU16(font, 4, 60);
      expect(load(font).table('zzzz'.codeUnits), isNull);
    });

    test('mistagged table lookups keep scanning', () {
      // A tag whose first byte matches but later bytes differ must not
      // match: 'heaX' vs 'head'.
      final font = load(buildSyntheticFontWithKern(const []));
      expect(font.table('heaX'.codeUnits), isNull);
      expect(font.table('head'.codeUnits), isNotNull);
    });

    test('bad tag length throws ArgumentError', () {
      final font = load(buildSyntheticFontWithKern(const []));
      expect(() => font.table([0x68, 0x65, 0x61]), throwsArgumentError);
      expect(() => font.table([0x68, 0x65, 0x61, 0x64, 0x00]),
          throwsArgumentError);
    });

    test('indexToLocFormat reads head+50', () {
      final font = load(buildSyntheticFontWithKern(const []));
      expect(font.indexToLocFormat(), 0);
      final inter = load(interRegular());
      expect(inter.indexToLocFormat(), isIn([0, 1]));
    });

    test('indexToLocFormat past the end throws BufferTooShort', () {
      var font = buildSyntheticFontWithKern(const []);
      font = setTableOffset(font, 'head', font.length - 16);
      final out = Uint8List.fromList(font);
      out.setAll(out.length - 4, [0x5F, 0x0F, 0x3C, 0xF5]);
      font = Uint8List.fromList(out);
      expect(
          () => load(font).indexToLocFormat(), throwsA(const BufferTooShort()));
    });

    test('numGlyphs reads maxp+4', () {
      final font = load(buildSyntheticFontWithKern(
        const [],
        numGlyphs: 9,
        numHMetrics: 9,
      ));
      expect(font.numGlyphs(), 9);
    });

    test('numGlyphs past the end throws BufferTooShort', () {
      var font = buildSyntheticFontWithKern(const []);
      font = setTableOffset(font, 'maxp', font.length - 5);
      expect(() => load(font).numGlyphs(), throwsA(const BufferTooShort()));
    });
  });

  group('errors', () {
    test('every variant carries the reference message', () {
      expect(const InvalidMagic().toString(),
          'FontError: invalid sfntVersion magic');
      expect(const InvalidHeadMagic().toString(),
          'FontError: invalid head.magicNumber');
      expect(const TableNotFound('head').toString(),
          "FontError: required table 'head' not found");
      expect(const TableNotFound('MATH.MathConstants').toString(),
          "FontError: required table 'MATH.MathConstants' not found");
      expect(const BufferTooShort().toString(), 'FontError: buffer too short');
      expect(const UnsupportedCmapFormat().toString(),
          'FontError: no Format 4 cmap subtable for platform 3 encoding 1');
    });

    test('errors are Exceptions with value equality', () {
      expect(const InvalidMagic(), isA<Exception>());
      expect(const InvalidMagic(), const InvalidMagic());
      expect(const InvalidMagic().hashCode, const InvalidMagic().hashCode);
      expect(const TableNotFound('head'), const TableNotFound('head'));
      expect(const TableNotFound('head'), isNot(const TableNotFound('hmtx')));
      expect(const TableNotFound('head').hashCode,
          const TableNotFound('head').hashCode);
      expect(const BufferTooShort(), const BufferTooShort());
      expect(const InvalidMagic(), isNot(const BufferTooShort()));
      expect(const TableNotFound('head'), isNot(const InvalidMagic()));
      expect(const UnsupportedCmapFormat(), const UnsupportedCmapFormat());
      expect(const InvalidHeadMagic(), const InvalidHeadMagic());
      expect(
          const InvalidHeadMagic().hashCode, const InvalidHeadMagic().hashCode);
      expect(const BufferTooShort().hashCode, const BufferTooShort().hashCode);
      expect(const UnsupportedCmapFormat().hashCode,
          const UnsupportedCmapFormat().hashCode);
    });

    test('UnsupportedCmapFormat is defined but never thrown', () {
      // The port answers "no usable subtable" with null, like the
      // reference; this variant exists for vocabulary parity only.
      const err = UnsupportedCmapFormat();
      expect(err, isA<FontError>());
    });
  });

  group('value semantics', () {
    test('FontMetrics compares by value', () {
      const a = FontMetrics(
        unitsPerEm: 2048,
        ascender: 1900,
        descender: -500,
        lineGap: 0,
        xHeight: 1000,
        capHeight: 1400,
        numGlyphs: 2000,
        familyName: 'Inter',
        subfamilyName: 'Regular',
      );
      const b = FontMetrics(
        unitsPerEm: 2048,
        ascender: 1900,
        descender: -500,
        lineGap: 0,
        xHeight: 1000,
        capHeight: 1400,
        numGlyphs: 2000,
        familyName: 'Inter',
        subfamilyName: 'Regular',
      );
      expect(a, b);
      expect(a.hashCode, b.hashCode);
      expect(a.toString(), contains('Inter'));
      expect(
          a,
          isNot(const FontMetrics(
            unitsPerEm: 1000,
            ascender: 1900,
            descender: -500,
            lineGap: 0,
            xHeight: 1000,
            capHeight: 1400,
            numGlyphs: 2000,
            familyName: 'Inter',
            subfamilyName: 'Regular',
          )));
      expect(a, isNot('not metrics'));
    });

    test('GlyphMetrics compares by value', () {
      expect(const GlyphMetrics(advanceWidth: 600, leftSideBearing: 50),
          const GlyphMetrics(advanceWidth: 600, leftSideBearing: 50));
      expect(
          const GlyphMetrics(advanceWidth: 600, leftSideBearing: 50).hashCode,
          const GlyphMetrics(advanceWidth: 600, leftSideBearing: 50).hashCode);
      expect(const GlyphMetrics(advanceWidth: 600, leftSideBearing: 50),
          isNot(const GlyphMetrics(advanceWidth: 601, leftSideBearing: 50)));
      expect(
          const GlyphMetrics(advanceWidth: 600, leftSideBearing: 50).toString(),
          contains('600'));
      expect(const GlyphMetrics(advanceWidth: 600, leftSideBearing: 50),
          isNot('not metrics'));
    });
  });
}
