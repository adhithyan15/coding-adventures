/// cowsay — routed through paint-vm-ascii (Dart port).
///
/// Ninth language in the cowsay-through-paint-vm-ascii rollout (after
/// csharp, fsharp, perl, haskell, java, kotlin). Everything up through
/// composing the bubble+cow text block is ordinary string formatting,
/// ported unchanged from the reference implementation at
/// `code/programs/go/cowsay/main.go`. The one thing that's different from
/// that reference: instead of printing the composed text directly,
/// [buildScene] converts it into a `PaintScene` of `glyph_run`
/// instructions (one glyph placement per non-space character, positioned
/// on an 8x16 character grid), and [render] turns that scene back into the
/// terminal string we print. This is also the PR that built
/// `coding_adventures_paint_vm_ascii` from scratch, implementing the full
/// P2D02 contract — see that package's own CHANGELOG.
library cowsay;

import 'dart:io';

import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart'
    hide version;
import 'package:coding_adventures_paint_vm_ascii/coding_adventures_paint_vm_ascii.dart'
    hide version;

/// paint-vm-ascii's documented default scale factors (`P2D02-paint-vm-ascii.md`).
const double scaleX = 8.0;
const double scaleY = 16.0;

/// The resolved set of inputs needed to render one cowsay invocation.
final class CowsayInvocation {
  final String message;
  final String eyes;
  final String tongue;
  final List<String> activeModes;
  final bool noWrap;
  final int width;
  final bool think;
  final String cowFile;

  const CowsayInvocation({
    required this.message,
    required this.eyes,
    required this.tongue,
    required this.activeModes,
    required this.noWrap,
    required this.width,
    required this.think,
    required this.cowFile,
  });
}

final class EyesAndTongue {
  final String eyes;
  final String tongue;
  const EyesAndTongue(this.eyes, this.tongue);
}

final class _ModeOverride {
  final String eyes;
  final String? tongue;
  const _ModeOverride(this.eyes, this.tongue);
}

const Map<String, _ModeOverride> _modeOverrides = {
  'borg': _ModeOverride('==', null),
  'dead': _ModeOverride('XX', 'U '),
  'greedy': _ModeOverride(r'$$', null),
  'paranoid': _ModeOverride('@@', null),
  'stoned': _ModeOverride('xx', 'U '),
  'tired': _ModeOverride('--', null),
  'wired': _ModeOverride('OO', null),
  'youthful': _ModeOverride('..', null),
};

/// Order matches `code/specs/cowsay.json`'s "modes" mutually-exclusive group.
const List<String> modeFlagIds = [
  'borg',
  'dead',
  'greedy',
  'paranoid',
  'stoned',
  'tired',
  'wired',
  'youthful',
];

// ---------------------------------------------------------------------------
// Rendering core (ported from code/programs/go/cowsay/main.go)
// ---------------------------------------------------------------------------

/// Splits text into lines no longer than [width], breaking on word
/// boundaries. A single word longer than the width is kept whole (never
/// split mid-word).
List<String> wrapText(String text, int width) {
  if (text.length <= width) return [text];

  final words = text.split(' ').where((w) => w.isNotEmpty).toList();
  if (words.isEmpty) return [''];

  final lines = <String>[];
  var current = StringBuffer();
  for (final word in words) {
    if (current.length + word.length + 1 <= width) {
      if (current.isNotEmpty) current.write(' ');
      current.write(word);
    } else {
      if (current.isNotEmpty) lines.add(current.toString());
      current = StringBuffer(word);
    }
  }
  if (current.isNotEmpty) lines.add(current.toString());
  return lines;
}

/// Draws the speech/thought bubble around the given lines. A single line
/// gets `"< ... >"` (or `"( ... )"` for a thought bubble); multiple lines
/// get `"/ ... \"`, `"| ... |"`, `"\ ... /"` (or `"( ... )"` on every line
/// for a thought bubble).
String formatBubble(List<String> lines, bool isThink) {
  if (lines.isEmpty) return '';

  final maxLen = lines.map((l) => l.length).reduce((a, b) => a > b ? a : b);
  final borderTop = ' ${'_' * (maxLen + 2)}';
  final borderBottom = ' ${'-' * (maxLen + 2)}';

  final List<String> body;
  if (lines.length == 1) {
    final start = isThink ? '(' : '<';
    final end = isThink ? ')' : '>';
    body = ['$start ${lines[0].padRight(maxLen)} $end'];
  } else {
    final n = lines.length;
    body = [
      for (var i = 0; i < n; i++)
        () {
          final String start;
          final String end;
          if (isThink) {
            start = '(';
            end = ')';
          } else if (i == 0) {
            start = '/';
            end = '\\';
          } else if (i == n - 1) {
            start = '\\';
            end = '/';
          } else {
            start = '|';
            end = '|';
          }
          return '$start ${lines[i].padRight(maxLen)} $end';
        }(),
    ];
  }

  return [borderTop, ...body, borderBottom].join('\n');
}

/// Pads or truncates a mode string (eyes/tongue) to exactly two characters,
/// matching cowsay's convention that eyes/tongue are always a 2-char glyph.
String normalizeTwoChars(String value) {
  if (value.length < 2) return value.padRight(2);
  if (value.length > 2) return value.substring(0, 2);
  return value;
}

/// Applies mode shortcuts (--borg, --dead, etc.) on top of the base
/// eyes/tongue flag values, then normalizes both to two characters. Modes
/// are mutually exclusive per cowsay.json, but this accepts any set for
/// robustness.
EyesAndTongue resolveEyesAndTongue(
    String baseEyes, String baseTongue, List<String> activeModes) {
  var eyes = baseEyes;
  var tongue = baseTongue;
  for (final mode in activeModes) {
    final override = _modeOverrides[mode];
    if (override == null) continue;
    eyes = override.eyes;
    if (override.tongue != null) tongue = override.tongue!;
  }
  return EyesAndTongue(normalizeTwoChars(eyes), normalizeTwoChars(tongue));
}

/// Walks up from [startDir] looking for CLAUDE.md, the repo-root sentinel
/// file. CLAUDE.md (not code/specs/cowsay.json itself) is used
/// deliberately — it's a more robust marker than reaching for the very
/// file being located, and this exact fix was called out as a lesson from
/// a prior, reverted cowsay Lua port's CI pathing problems (PR #1535).
String findRepoRoot(String startDir) {
  var dir = _normalizeAbsolutePath(startDir);
  for (var i = 0; i < 24; i++) {
    if (File('$dir${Platform.pathSeparator}CLAUDE.md').existsSync()) return dir;
    final parent = File(dir).parent.path;
    if (parent == dir) return _normalizeAbsolutePath(startDir);
    dir = parent;
  }
  return _normalizeAbsolutePath(startDir);
}

String _normalizeAbsolutePath(String path) {
  final absolute = File(path).absolute.path;
  final uri = Uri.file(absolute, windows: Platform.isWindows);
  return uri.normalizePath().toFilePath(windows: Platform.isWindows);
}

/// The last path segment of [value], treating both `/` and `\` as
/// separators regardless of host platform. Mirrors `Path.fileName` in the
/// JVM ports: directory components (including any number of `..`) are
/// discarded entirely, leaving only the final segment.
String _basenameOf(String value) {
  final normalized = value.replaceAll('\\', '/');
  final segments = normalized.split('/').where((s) => s.isNotEmpty);
  return segments.isEmpty ? '' : segments.last;
}

/// Whether [value] is rooted: a POSIX absolute path (`/...`), a
/// Windows-style rooted path (`\...`), or a Windows drive-qualified path
/// (`C:\...`, `C:/...`).
bool _looksRooted(String value) {
  if (value.isEmpty) return false;
  if (value.startsWith('/') || value.startsWith('\\')) return true;
  if (value.length >= 2 && value[1] == ':') {
    final drive = value.codeUnitAt(0);
    final isLetter =
        (drive >= 0x41 && drive <= 0x5a) || (drive >= 0x61 && drive <= 0x7a);
    if (isLetter) return true;
  }
  return false;
}

final RegExp _cowBodyPattern = RegExp(r'<<EOC;\n(.*?)EOC', dotAll: true);

/// Loads a .cow template's body from [cowsDir], falling back to
/// default.cow when the requested file doesn't exist. The template is a
/// Perl heredoc (`$the_cow = <<EOC; ... EOC`); only the body between the
/// heredoc markers is returned.
///
/// [cowName] comes from the user-supplied -f/--file flag, so it is treated
/// as untrusted: only a bare filename (no directory separators, no
/// rooted/absolute path) is accepted, and the resolved path is verified to
/// stay inside [cowsDir] before it's read — otherwise this falls back to
/// default.cow instead of reading an arbitrary file the caller pointed at
/// via `".."`, a rooted override, or similar (mirrors the fix applied to
/// every other port's loadCow after `/security-review`). The `.cow` suffix
/// is appended in the same string as the extracted basename (not via a
/// separate path-join step), so even a literal `".."` basename becomes the
/// harmless single filename segment `"...cow"` rather than a parent-directory
/// reference. Any exception while resolving or checking the candidate path
/// (e.g. a malformed path Dart's IO layer rejects) is treated the same as
/// "not found": fall back to default.cow.
String loadCow(String cowName, String cowsDir) {
  final cowsRootRaw = _normalizeAbsolutePath(cowsDir);
  final cowsRoot = cowsRootRaw.endsWith(Platform.pathSeparator)
      ? cowsRootRaw.substring(0, cowsRootRaw.length - 1)
      : cowsRootRaw;

  String? candidate;
  try {
    final safeName = _basenameOf(cowName);
    final rooted = _looksRooted(cowName);
    if (safeName.isNotEmpty && !rooted) {
      candidate =
          _normalizeAbsolutePath('$cowsRoot${Platform.pathSeparator}$safeName.cow');
    }
  } catch (_) {
    candidate = null;
  }

  var withinCowsDir = false;
  var candidateExists = false;
  if (candidate != null) {
    withinCowsDir = candidate == cowsRoot ||
        candidate.startsWith('$cowsRoot${Platform.pathSeparator}');
    try {
      candidateExists = withinCowsDir && File(candidate).existsSync();
    } catch (_) {
      candidateExists = false;
    }
  }

  final cowPath = (candidate != null && withinCowsDir && candidateExists)
      ? candidate
      : '$cowsRoot${Platform.pathSeparator}default.cow';

  final contents = File(cowPath).readAsStringSync();
  final match = _cowBodyPattern.firstMatch(contents);
  return match?.group(1) ?? contents;
}

/// Composes the full bubble+cow text block for one invocation —
/// everything up to (but not including) the paint-vm-ascii render step.
String composeContent(CowsayInvocation invocation, String cowsDir) {
  final eyesAndTongue = resolveEyesAndTongue(
      invocation.eyes, invocation.tongue, invocation.activeModes);

  final lines = <String>[];
  for (final rawLine in invocation.message.split('\n')) {
    if (rawLine.isEmpty) {
      lines.add('');
    } else if (invocation.noWrap) {
      lines.add(rawLine);
    } else {
      lines.addAll(wrapText(rawLine, invocation.width));
    }
  }

  final thoughts = invocation.think ? 'o' : '\\';
  final bubble = formatBubble(lines, invocation.think);

  final cowTemplate = loadCow(invocation.cowFile, cowsDir);
  final cow = cowTemplate
      .replaceAll(r'$eyes', eyesAndTongue.eyes)
      .replaceAll(r'$tongue', eyesAndTongue.tongue)
      .replaceAll(r'$thoughts', thoughts)
      .replaceAll('\\\\', '\\');

  return '$bubble\n$cow';
}

/// Converts a composed text block into a [PaintScene]: one `glyph_run`
/// instruction per line, one glyph placement per non-space character. See
/// `code/specs/cowsay-paintvm-pipeline.md` §3 for the full contract,
/// including why glyphId is a literal Unicode code point here (an
/// ASCII-backend-only relaxation of the general PaintGlyphRun contract).
PaintScene buildScene(String text) {
  final normalized = text.replaceAll('\r\n', '\n');
  final lines = normalized.split('\n');

  var maxWidth = 0;
  for (final line in lines) {
    if (line.length > maxWidth) maxWidth = line.length;
  }

  final instructions = <PaintInstruction>[];
  for (var row = 0; row < lines.length; row++) {
    final line = lines[row];
    final glyphs = <PaintGlyphPlacement>[];
    for (var col = 0; col < line.length; col++) {
      final ch = line.codeUnitAt(col);
      if (ch == 0x20) continue;
      glyphs.add(PaintGlyphPlacement(
        glyphId: ch,
        x: col * scaleX,
        y: row * scaleY,
      ));
    }
    if (glyphs.isNotEmpty) {
      instructions.add(paintGlyphRun(
        glyphs: glyphs,
        fontRef: 'terminal-mono',
        fontSize: scaleY,
        fill: '#000000',
      ));
    }
  }

  final width = ((maxWidth < 1 ? 1 : maxWidth) * scaleX).toInt();
  final height = ((lines.length < 1 ? 1 : lines.length) * scaleY).toInt();
  return PaintScene(
    width: width,
    height: height,
    background: 'transparent',
    instructions: instructions,
    metadata: const {},
  );
}

/// End-to-end: compose the bubble+cow text, build a [PaintScene] from it,
/// and render that scene through paint-vm-ascii.
PaintVmAsciiResult renderCowsay(CowsayInvocation invocation, String cowsDir) {
  final content = composeContent(invocation, cowsDir);
  final scene = buildScene(content);
  return render(
    scene,
    AsciiOptions(scaleX: scaleX.toInt(), scaleY: scaleY.toInt()),
  );
}

// ---------------------------------------------------------------------------
// CLI glue — the bridge between CliBuilder's flags/arguments maps and this
// module's typed invocation. Kept in this file (rather than bin/cowsay.dart)
// so it's directly unit-testable without spawning a process or driving a
// real Parser.
// ---------------------------------------------------------------------------

bool isListRequested(Map<String, Object?> flags) => flags['list'] == true;

/// Cow file basenames under [cowsDir], sorted ordinally.
List<String> listCowFiles(String cowsDir) {
  final names = Directory(cowsDir)
      .listSync()
      .whereType<File>()
      .map((f) => f.uri.pathSegments.last)
      .where((name) => name.endsWith('.cow'))
      .map((name) => name.substring(0, name.length - '.cow'.length))
      .toList();
  names.sort();
  return names;
}

/// Resolves the message from the parsed "message" positional argument.
/// Returns null when no message was given on argv — the caller should fall
/// back to stdin.
String? resolveMessageFromArguments(Map<String, Object?> arguments) {
  final parts = arguments['message'] as List<Object?>?;
  if (parts == null || parts.isEmpty) return null;
  return parts.map((p) => p.toString()).join(' ');
}

/// Builds a [CowsayInvocation] from a resolved message and the parsed
/// flags map, applying cowsay.json's documented defaults for any flag that
/// wasn't explicitly set.
CowsayInvocation buildInvocation(String message, Map<String, Object?> flags) {
  final eyes = flags['eyes'] as String? ?? 'oo';
  final tongue = flags['tongue'] as String? ?? '  ';
  final cowFile = flags['cowfile'] as String? ?? 'default';
  final noWrap = flags['nowrap'] == true;
  final think = flags['think'] == true;

  final rawWidth = flags['width'];
  final width = rawWidth is int ? _clampWidth(rawWidth) : 40;

  final activeModes = modeFlagIds.where((id) => flags[id] == true).toList();

  return CowsayInvocation(
    message: message,
    eyes: eyes,
    tongue: tongue,
    activeModes: activeModes,
    noWrap: noWrap,
    width: width,
    think: think,
    cowFile: cowFile,
  );
}

int _clampWidth(int value) {
  if (value < 1) return 1;
  return value;
}
