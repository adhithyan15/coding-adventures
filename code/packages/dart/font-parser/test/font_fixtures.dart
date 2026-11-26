/// Shared font builders and byte-surgery helpers for the font-parser
/// tests: the synthetic kern font, the MATH container, the Inter/STIX
/// fixture loaders, and the patch/plant/table-offset tools the
/// corruption tests use to aim at exact byte addresses.
library;

import 'dart:io';
import 'dart:typed_data';

/// Loads Inter Regular from the shared fixtures directory.
Uint8List interRegular() =>
    File('../../../fixtures/fonts/Inter-Regular.ttf').readAsBytesSync();

/// The raw STIX Two Math `MATH` table (see `fixtures/PROVENANCE.md`).
Uint8List stixMath() =>
    File('test/fixtures/stix-two-math.MATH').readAsBytesSync();

/// Writes [v] big-endian into [buf].
void writeU16(BytesBuilder buf, int v) => buf.add([(v >> 8) & 0xFF, v & 0xFF]);

/// Writes signed [v] big-endian into [buf].
void writeI16(BytesBuilder buf, int v) => writeU16(buf, v & 0xFFFF);

/// Writes [v] big-endian into [buf].
void writeU32(BytesBuilder buf, int v) => buf.add([
      (v >> 24) & 0xFF,
      (v >> 16) & 0xFF,
      (v >> 8) & 0xFF,
      v & 0xFF,
    ]);

/// Reads a big-endian u16 at absolute [offset].
int readU16(Uint8List font, int offset) =>
    (font[offset] << 8) | font[offset + 1];

/// Builds a minimal synthetic font with a kern Format 0 subtable, mirroring
/// the reference's `build_synthetic_font_with_kern`.
///
/// Six tables (cmap, head, hhea, hmtx, kern, maxp); the cmap maps only the
/// U+FFFF sentinel; hmtx holds [numHMetrics] full records of (advance 600,
/// lsb 50) plus [trailingLsb]; the kern subtable carries [pairs] sorted by
/// composite key.
///
/// [nameStrings] adds a `name` table: each entry is (platformID,
/// encodingID, nameID, UTF-16BE bytes). [os2] adds an `OS/2` table with
/// (version, typoAscender, typoDescender, typoLineGap, xHeight, capHeight).
/// Both default to absent.
Uint8List buildSyntheticFontWithKern(
  List<(int, int, int)> pairs, {
  int numGlyphs = 5,
  int numHMetrics = 5,
  List<int> trailingLsb = const [],
  List<(int, int, int, List<int>)> nameStrings = const [],
  (int, int, int, int, int, int)? os2,
}) {
  // Assemble bodies first so optional tables just join the list.
  final head = BytesBuilder();
  writeU32(head, 0x00010000);
  writeU32(head, 0x00010000);
  writeU32(head, 0);
  writeU32(head, 0x5F0F3CF5);
  writeU16(head, 0);
  writeU16(head, 1000);
  head.add(List.filled(8, 0));
  head.add(List.filled(8, 0));
  for (var i = 0; i < 4; i++) {
    writeI16(head, 0);
  }
  writeU16(head, 0);
  writeU16(head, 8);
  writeI16(head, 2);
  writeI16(head, 0);
  writeI16(head, 0);

  final hhea = BytesBuilder();
  writeU32(hhea, 0x00010000);
  writeI16(hhea, 800);
  writeI16(hhea, -200);
  writeI16(hhea, 0);
  writeU16(hhea, 1000);
  writeI16(hhea, 0);
  writeI16(hhea, 0);
  writeI16(hhea, 0);
  writeI16(hhea, 1);
  writeI16(hhea, 0);
  writeI16(hhea, 0);
  for (var i = 0; i < 4; i++) {
    writeI16(hhea, 0);
  }
  writeI16(hhea, 0);
  writeU16(hhea, numHMetrics);

  final maxp = BytesBuilder();
  writeU32(maxp, 0x00005000);
  writeU16(maxp, numGlyphs);

  final cmap = BytesBuilder();
  writeU16(cmap, 0);
  writeU16(cmap, 1);
  writeU16(cmap, 3);
  writeU16(cmap, 1);
  writeU32(cmap, 12);
  writeU16(cmap, 4);
  writeU16(cmap, 24);
  writeU16(cmap, 0);
  writeU16(cmap, 2);
  writeU16(cmap, 2);
  writeU16(cmap, 0);
  writeU16(cmap, 0);
  writeU16(cmap, 0xFFFF);
  writeU16(cmap, 0);
  writeU16(cmap, 0xFFFF);
  writeI16(cmap, 1);
  writeU16(cmap, 0);

  final hmtx = BytesBuilder();
  for (var i = 0; i < numHMetrics; i++) {
    writeU16(hmtx, 600);
    writeI16(hmtx, 50);
  }
  for (final lsb in trailingLsb) {
    writeI16(hmtx, lsb);
  }

  final kern = BytesBuilder();
  writeU16(kern, 0);
  writeU16(kern, 1);
  writeU16(kern, 0);
  writeU16(kern, 6 + 8 + pairs.length * 6);
  writeU16(kern, 0x0001);
  writeU16(kern, pairs.length);
  writeU16(kern, 6);
  writeU16(kern, 0);
  writeU16(kern, 0);
  final sorted = pairs.toList()
    ..sort((a, b) => ((a.$1 << 16) | a.$2).compareTo((b.$1 << 16) | b.$2));
  for (final (l, r, v) in sorted) {
    writeU16(kern, l);
    writeU16(kern, r);
    writeI16(kern, v);
  }

  final bodies = <String, Uint8List>{
    'cmap': cmap.toBytes(),
    'head': head.toBytes(),
    'hhea': hhea.toBytes(),
    'hmtx': hmtx.toBytes(),
    'kern': kern.toBytes(),
    'maxp': maxp.toBytes(),
  };

  if (nameStrings.isNotEmpty) {
    final name = BytesBuilder();
    writeU16(name, 0);
    writeU16(name, nameStrings.length);
    writeU16(name, 6 + nameStrings.length * 12);
    var strOff = 0;
    final storage = BytesBuilder();
    for (final (platform, encoding, nameId, bytes) in nameStrings) {
      writeU16(name, platform);
      writeU16(name, encoding);
      writeU16(name, 0x0409); // language: en-US
      writeU16(name, nameId);
      writeU16(name, bytes.length);
      writeU16(name, strOff);
      storage.add(bytes);
      strOff += bytes.length;
    }
    name.add(storage.toBytes());
    bodies['name'] = name.toBytes();
  }

  final os2spec = os2;
  if (os2spec != null) {
    final table = BytesBuilder();
    writeU16(table, os2spec.$1); // version
    table.add(List.filled(66, 0)); // ... up to typoAscender at 68
    writeI16(table, os2spec.$2); // typoAscender at 68
    writeI16(table, os2spec.$3); // typoDescender at 70
    writeI16(table, os2spec.$4); // typoLineGap at 72
    table.add(List.filled(12, 0)); // ... up to xHeight at 86
    writeI16(table, os2spec.$5); // xHeight at 86
    writeI16(table, os2spec.$6); // capHeight at 88
    bodies['OS/2'] = table.toBytes();
  }

  final tags = bodies.keys.toList()..sort();
  final buf = BytesBuilder();
  writeU32(buf, 0x00010000);
  writeU16(buf, tags.length);
  writeU16(buf, 64);
  writeU16(buf, 2);
  writeU16(buf, 32);
  var off = 12 + tags.length * 16;
  final offsets = <String, int>{};
  for (final tag in tags) {
    offsets[tag] = off;
    off += bodies[tag]!.length;
  }
  for (final tag in tags) {
    buf.add(tag.codeUnits);
    writeU32(buf, 0);
    writeU32(buf, offsets[tag]!);
    writeU32(buf, bodies[tag]!.length);
  }
  for (final tag in tags) {
    buf.add(bodies[tag]!);
  }
  return buf.toBytes();
}

/// Wraps a bare MATH [body] in a minimal but genuine OpenType container.
///
/// The table *directory* lookup is exercised too rather than assumed.
/// `head` carries the magic `load` validates; the other required tables
/// are minimal, since nothing here reads them.
Uint8List fontWithMathTable({Uint8List? mathBody}) {
  final math = mathBody ?? stixMath();
  final tags = ['MATH', 'cmap', 'head', 'hhea', 'hmtx', 'maxp'];
  final head = Uint8List(54);
  head.setAll(12, [0x5F, 0x0F, 0x3C, 0xF5]);
  head.setAll(18, [0x03, 0xE8]); // unitsPerEm 1000
  final hhea = Uint8List(36);
  final maxp = Uint8List(6)..setAll(4, [0x13, 0x88]); // numGlyphs 5000
  final cmap = Uint8List(4);
  final hmtx = Uint8List(4);
  final bodies = [math, cmap, head, hhea, hmtx, maxp];

  final buf = BytesBuilder();
  buf.add([0, 1, 0, 0]);
  final n = tags.length;
  buf.add([(n >> 8) & 0xFF, n & 0xFF]);
  buf.add([0, 0, 0, 0, 0, 0]);
  var offset = 12 + 16 * tags.length;
  final records = <(int, int)>[];
  for (final body in bodies) {
    records.add((offset, body.length));
    offset += body.length;
    while (offset % 4 != 0) {
      offset++;
    }
  }
  for (var i = 0; i < tags.length; i++) {
    buf.add(tags[i].codeUnits);
    buf.add([0, 0, 0, 0]);
    final (off, len) = records[i];
    buf.add([
      (off >> 24) & 0xFF,
      (off >> 16) & 0xFF,
      (off >> 8) & 0xFF,
      off & 0xFF
    ]);
    buf.add([
      (len >> 24) & 0xFF,
      (len >> 16) & 0xFF,
      (len >> 8) & 0xFF,
      len & 0xFF
    ]);
  }
  for (final body in bodies) {
    buf.add(body);
    while (buf.length % 4 != 0) {
      buf.addByte(0);
    }
  }
  return buf.toBytes();
}

/// Reads the directory offset of table [tag] from [font].
int tableOffset(Uint8List font, String tag) {
  final numTables = (font[4] << 8) | font[5];
  final tagBytes = tag.codeUnits;
  for (var i = 0; i < numTables; i++) {
    final rec = 12 + i * 16;
    if (font[rec] == tagBytes[0] &&
        font[rec + 1] == tagBytes[1] &&
        font[rec + 2] == tagBytes[2] &&
        font[rec + 3] == tagBytes[3]) {
      return (font[rec + 8] << 24) |
          (font[rec + 9] << 16) |
          (font[rec + 10] << 8) |
          font[rec + 11];
    }
  }
  throw StateError('no $tag record');
}

/// Patches a big-endian u16 at absolute [offset] in a copy of [font].
Uint8List patchU16(Uint8List font, int offset, int v) {
  final out = Uint8List.fromList(font);
  out[offset] = (v >> 8) & 0xFF;
  out[offset + 1] = v & 0xFF;
  return out;
}

/// Patches a big-endian u32 at absolute [offset] in a copy of [font].
Uint8List patchU32(Uint8List font, int offset, int v) {
  final out = Uint8List.fromList(font);
  out[offset] = (v >> 24) & 0xFF;
  out[offset + 1] = (v >> 16) & 0xFF;
  out[offset + 2] = (v >> 8) & 0xFF;
  out[offset + 3] = v & 0xFF;
  return out;
}

/// Rewrites the directory offset of table [tag] in a copy of [font].
///
/// Throws [StateError] if the tag is absent (a test bug, not a font under
/// test).
Uint8List setTableOffset(Uint8List font, String tag, int offset) {
  final out = Uint8List.fromList(font);
  final numTables = (out[4] << 8) | out[5];
  final tagBytes = tag.codeUnits;
  for (var i = 0; i < numTables; i++) {
    final rec = 12 + i * 16;
    if (out[rec] == tagBytes[0] &&
        out[rec + 1] == tagBytes[1] &&
        out[rec + 2] == tagBytes[2] &&
        out[rec + 3] == tagBytes[3]) {
      out[rec + 8] = (offset >> 24) & 0xFF;
      out[rec + 9] = (offset >> 16) & 0xFF;
      out[rec + 10] = (offset >> 8) & 0xFF;
      out[rec + 11] = offset & 0xFF;
      return out;
    }
  }
  throw StateError('no $tag record');
}

/// Writes [bytes] at absolute [offset] in a copy of [font].
Uint8List plant(Uint8List font, int offset, List<int> bytes) {
  final out = Uint8List.fromList(font);
  out.setAll(offset, bytes);
  return out;
}

/// Encodes BMP [text] as UTF-16 BE bytes.
List<int> utf16be(String text) {
  final out = <int>[];
  for (final unit in text.codeUnits) {
    out.add((unit >> 8) & 0xFF);
    out.add(unit & 0xFF);
  }
  return out;
}
