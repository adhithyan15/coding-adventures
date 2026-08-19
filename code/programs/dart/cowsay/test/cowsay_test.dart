import 'dart:io';

import 'package:coding_adventures_cli_builder/cli_builder.dart';
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart'
    hide version;
import 'package:coding_adventures_paint_vm_ascii/coding_adventures_paint_vm_ascii.dart'
    hide version;
import 'package:cowsay/src/cowsay.dart';
import 'package:test/test.dart';

void writeCow(String dir, String name, String contents) {
  File('$dir${Platform.pathSeparator}$name.cow').writeAsStringSync(contents);
}

String resolveRepoRoot() => findRepoRoot(Directory.current.path);

CowsayInvocation baseInvocation({
  String message = 'hi',
  String eyes = 'oo',
  String tongue = '  ',
  List<String> activeModes = const [],
  bool noWrap = false,
  int width = 40,
  bool think = false,
  String cowFile = 'default',
}) =>
    CowsayInvocation(
      message: message,
      eyes: eyes,
      tongue: tongue,
      activeModes: activeModes,
      noWrap: noWrap,
      width: width,
      think: think,
      cowFile: cowFile,
    );

void main() {
  late Directory tempDir;
  late Directory tempOutsideDir;

  setUp(() {
    tempDir = Directory.systemTemp.createTempSync('cowsay_test_');
    tempOutsideDir = Directory.systemTemp.createTempSync('cowsay_test_outside_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
    tempOutsideDir.deleteSync(recursive: true);
  });

  // -------------------------------------------------------------------
  // wrapText
  // -------------------------------------------------------------------

  group('wrapText', () {
    test('does not wrap short text', () {
      expect(wrapText('hello', 40), ['hello']);
    });

    test('wraps long text at word boundaries', () {
      expect(wrapText('the quick brown fox jumps over', 10),
          ['the quick', 'brown fox', 'jumps over']);
    });

    test('returns an empty line for empty text', () {
      expect(wrapText('', 40), ['']);
    });

    test('keeps a single word longer than the width whole', () {
      expect(wrapText('supercalifragilisticexpialidocious', 5),
          ['supercalifragilisticexpialidocious']);
    });
  });

  // -------------------------------------------------------------------
  // formatBubble
  // -------------------------------------------------------------------

  group('formatBubble', () {
    test('returns empty string for no lines', () {
      expect(formatBubble([], false), '');
    });

    test('draws a single-line speech bubble', () {
      expect(formatBubble(['hi'], false), ' ____\n< hi >\n ----');
    });

    test('draws a single-line thought bubble', () {
      expect(formatBubble(['hi'], true), ' ____\n( hi )\n ----');
    });

    test('draws a multi-line speech bubble with slash pipe backslash borders', () {
      expect(
        formatBubble(['one', 'two', 'three'], false),
        ' _______\n/ one   \\\n| two   |\n\\ three /\n -------',
      );
    });

    test('draws a multi-line thought bubble with parens on every line', () {
      expect(formatBubble(['one', 'two'], true), ' _____\n( one )\n( two )\n -----');
    });
  });

  // -------------------------------------------------------------------
  // normalizeTwoChars
  // -------------------------------------------------------------------

  group('normalizeTwoChars', () {
    test('pads a one-character value', () {
      expect(normalizeTwoChars('o'), 'o ');
    });

    test('pads an empty value', () {
      expect(normalizeTwoChars(''), '  ');
    });

    test('leaves a two-character value unchanged', () {
      expect(normalizeTwoChars('oo'), 'oo');
    });

    test('truncates a longer value', () {
      expect(normalizeTwoChars('ooo'), 'oo');
    });
  });

  // -------------------------------------------------------------------
  // resolveEyesAndTongue
  // -------------------------------------------------------------------

  group('resolveEyesAndTongue', () {
    test('keeps base values when no modes are active', () {
      final result = resolveEyesAndTongue('oo', '  ', []);
      expect(result.eyes, 'oo');
      expect(result.tongue, '  ');
    });

    test('borg overrides eyes only', () {
      final result = resolveEyesAndTongue('oo', '  ', ['borg']);
      expect(result.eyes, '==');
      expect(result.tongue, '  ');
    });

    test('dead overrides both eyes and tongue', () {
      final result = resolveEyesAndTongue('oo', '  ', ['dead']);
      expect(result.eyes, 'XX');
      expect(result.tongue, 'U ');
    });

    test('stoned overrides both eyes and tongue', () {
      final result = resolveEyesAndTongue('oo', '  ', ['stoned']);
      expect(result.eyes, 'xx');
      expect(result.tongue, 'U ');
    });

    test('ignores an unknown mode', () {
      final result = resolveEyesAndTongue('oo', '  ', ['not-a-real-mode']);
      expect(result.eyes, 'oo');
      expect(result.tongue, '  ');
    });
  });

  // -------------------------------------------------------------------
  // loadCow
  // -------------------------------------------------------------------

  group('loadCow', () {
    test('loads the body between heredoc markers', () {
      writeCow(tempDir.path, 'default',
          '\$the_cow = <<EOC;\n  \$thoughts   ^__^\n   (\$eyes)\nEOC\n');
      expect(loadCow('default', tempDir.path), '  \$thoughts   ^__^\n   (\$eyes)\n');
    });

    test('falls back to default cow when the named cow is missing', () {
      writeCow(tempDir.path, 'default', '\$the_cow = <<EOC;\nfallback\nEOC\n');
      expect(loadCow('does-not-exist', tempDir.path), 'fallback\n');
    });

    test('falls back to default cow instead of escaping via traversal', () {
      writeCow(tempDir.path, 'default', '\$the_cow = <<EOC;\nfallback\nEOC\n');
      writeCow(tempOutsideDir.path, 'secret', '\$the_cow = <<EOC;\nSECRET\nEOC\n');
      writeCow(tempOutsideDir.path, 'outside', '\$the_cow = <<EOC;\nSECRET\nEOC\n');
      for (final malicious in [
        '../../../../../../etc/passwd',
        '..\\..\\..\\secret',
        '../outside',
      ]) {
        expect(loadCow(malicious, tempDir.path), 'fallback\n', reason: 'for input: $malicious');
      }
    });

    test('falls back to default cow instead of following a rooted path override', () {
      writeCow(tempDir.path, 'default', '\$the_cow = <<EOC;\nfallback\nEOC\n');
      File('${tempOutsideDir.path}${Platform.pathSeparator}win.cow')
          .writeAsStringSync('\$the_cow = <<EOC;\nSECRET\nEOC\n');
      final rootedName = '${tempOutsideDir.path}${Platform.pathSeparator}win';
      expect(loadCow(rootedName, tempDir.path), 'fallback\n');
    });
  });

  // -------------------------------------------------------------------
  // composeContent
  // -------------------------------------------------------------------

  group('composeContent', () {
    test('composes bubble and cow with substitutions', () {
      writeCow(tempDir.path, 'default', '\$the_cow = <<EOC;\n\$thoughts \$eyes \$tongue\nEOC\n');
      expect(
        composeContent(baseInvocation(), tempDir.path),
        ' ____\n< hi >\n ----\n\\ oo   \n',
      );
    });

    test('think mode uses o for thoughts and a paren bubble', () {
      writeCow(tempDir.path, 'default', '\$the_cow = <<EOC;\n\$thoughts \$eyes \$tongue\nEOC\n');
      final invocation = baseInvocation(think: true);
      expect(
        composeContent(invocation, tempDir.path),
        ' ____\n( hi )\n ----\no oo   \n',
      );
    });

    test('a mode flag overrides eyes and tongue in the cow template', () {
      writeCow(tempDir.path, 'default', '\$the_cow = <<EOC;\n\$thoughts \$eyes \$tongue\nEOC\n');
      final invocation = baseInvocation(activeModes: ['dead']);
      expect(
        composeContent(invocation, tempDir.path),
        ' ____\n< hi >\n ----\n\\ XX U \n',
      );
    });
  });

  // -------------------------------------------------------------------
  // buildScene
  // -------------------------------------------------------------------

  group('buildScene', () {
    test('creates one glyph_run per non-blank line with correct placements', () {
      final scene = buildScene('hi\n\nyo');
      final runs = scene.instructions.whereType<PaintGlyphRun>().toList();
      expect(runs.length, 2);
      expect(runs[0].glyphs.map((g) => g.glyphId).toList(), ['h'.codeUnitAt(0), 'i'.codeUnitAt(0)]);
      expect(runs[0].glyphs.map((g) => g.x).toList(), [0.0, scaleX]);
      expect(runs[1].glyphs.map((g) => g.glyphId).toList(), ['y'.codeUnitAt(0), 'o'.codeUnitAt(0)]);
      expect(runs[1].glyphs.map((g) => g.y).toList(), [2 * scaleY, 2 * scaleY]);
    });

    test('skips spaces rather than placing them', () {
      final scene = buildScene('a b');
      final runs = scene.instructions.whereType<PaintGlyphRun>().toList();
      expect(runs.length, 1);
      expect(runs[0].glyphs.length, 2);
    });

    test('covers all lines in the scene dimensions', () {
      final scene = buildScene('abc\nde');
      expect(scene.width, (3 * scaleX).toInt());
      expect(scene.height, (2 * scaleY).toInt());
    });
  });

  // -------------------------------------------------------------------
  // render round trip
  // -------------------------------------------------------------------

  group('render round trip', () {
    test('round-trips simple single-line text', () {
      final scene = buildScene('hi');
      final result = render(scene, AsciiOptions(scaleX: scaleX.toInt(), scaleY: scaleY.toInt()));
      expect(result, isA<PaintVmAsciiOk>());
      expect((result as PaintVmAsciiOk).text, 'hi');
    });

    test('round-trips a bubble and cow block trimming the trailing blank line', () {
      const content = ' ____\n< hi >\n ----\n\\   ^__^\n';
      final scene = buildScene(content);
      final result = render(scene, AsciiOptions(scaleX: scaleX.toInt(), scaleY: scaleY.toInt()));
      expect(result, isA<PaintVmAsciiOk>());
      expect((result as PaintVmAsciiOk).text, ' ____\n< hi >\n ----\n\\   ^__^');
    });
  });

  // -------------------------------------------------------------------
  // CLI glue
  // -------------------------------------------------------------------

  group('CLI glue', () {
    test('isListRequested is true when the flag is present', () {
      expect(isListRequested({'list': true}), isTrue);
    });

    test('isListRequested is false when the flag is absent', () {
      expect(isListRequested({}), isFalse);
    });

    test('resolveMessageFromArguments joins positional words', () {
      expect(resolveMessageFromArguments({
        'message': ['hello', 'there']
      }), 'hello there');
    });

    test('resolveMessageFromArguments returns null when arguments is empty', () {
      expect(resolveMessageFromArguments({}), isNull);
    });

    test('resolveMessageFromArguments returns null when the message list is empty', () {
      expect(resolveMessageFromArguments({'message': <Object?>[]}), isNull);
    });

    test('buildInvocation uses defaults when no flags are set', () {
      final invocation = buildInvocation('hi', {});
      expect(invocation.message, 'hi');
      expect(invocation.eyes, 'oo');
      expect(invocation.tongue, '  ');
      expect(invocation.cowFile, 'default');
      expect(invocation.noWrap, isFalse);
      expect(invocation.think, isFalse);
      expect(invocation.width, 40);
      expect(invocation.activeModes, isEmpty);
    });

    test('buildInvocation honors explicit flags', () {
      final flags = {
        'eyes': '^^',
        'tongue': 'vv',
        'cowfile': 'dragon',
        'nowrap': true,
        'think': true,
        'width': 20,
        'borg': true,
      };
      final invocation = buildInvocation('hi', flags);
      expect(invocation.eyes, '^^');
      expect(invocation.tongue, 'vv');
      expect(invocation.cowFile, 'dragon');
      expect(invocation.noWrap, isTrue);
      expect(invocation.think, isTrue);
      expect(invocation.width, 20);
      expect(invocation.activeModes, ['borg']);
    });

    test('buildInvocation rejects a negative width', () {
      expect(buildInvocation('hi', {'width': -5}).width, 1);
    });

    test('listCowFiles returns sorted basenames', () {
      writeCow(tempDir.path, 'tux', '');
      writeCow(tempDir.path, 'default', '');
      writeCow(tempDir.path, 'dragon', '');
      expect(listCowFiles(tempDir.path), ['default', 'dragon', 'tux']);
    });
  });

  // -------------------------------------------------------------------
  // CliBuilder argv convention
  // -------------------------------------------------------------------

  group('CliBuilder argv convention', () {
    // Regression test: this Dart CliBuilder's Parser.parse() DOES expect a
    // leading program-name placeholder (it reads argv.first as the program
    // name and errors if argv is empty), matching the C/Go convention.
    // Verified against the real Parser, not just hand-built flags/arguments
    // maps.
    test('does not drop the first token when a program-name placeholder is prepended', () {
      final repoRoot = resolveRepoRoot();
      final specPath =
          '$repoRoot${Platform.pathSeparator}code${Platform.pathSeparator}specs${Platform.pathSeparator}cowsay.json';

      final outcome1 = Parser.fromPath(specPath, ['cowsay', 'hello']).parse();
      expect(outcome1, isA<ParseResult>());
      expect(resolveMessageFromArguments((outcome1 as ParseResult).arguments), 'hello');

      final outcome2 = Parser.fromPath(specPath, ['cowsay', 'hello', 'world']).parse();
      expect(outcome2, isA<ParseResult>());
      expect(resolveMessageFromArguments((outcome2 as ParseResult).arguments), 'hello world');
    });
  });

  // -------------------------------------------------------------------
  // end-to-end golden output
  // -------------------------------------------------------------------

  group('end-to-end golden output', () {
    test('resolves the real cows directory', () {
      final repoRoot = resolveRepoRoot();
      final cowsDir =
          '$repoRoot${Platform.pathSeparator}code${Platform.pathSeparator}specs${Platform.pathSeparator}cows';
      expect(listCowFiles(cowsDir), contains('default'));
    });

    test('default cow speaking Hello World', () {
      final repoRoot = resolveRepoRoot();
      final cowsDir =
          '$repoRoot${Platform.pathSeparator}code${Platform.pathSeparator}specs${Platform.pathSeparator}cows';
      final invocation = baseInvocation(message: 'Hello, World!');
      final result = renderCowsay(invocation, cowsDir);
      expect(result, isA<PaintVmAsciiOk>());
      expect(
        (result as PaintVmAsciiOk).text,
        [
          ' _______________',
          '< Hello, World! >',
          ' ---------------',
          '        \\   ^__^',
          '         \\  (oo)\\_______',
          '            (__)\\       )\\/\\',
          '                ||----w |',
          '                ||     ||',
        ].join('\n'),
      );
    });

    test('borg mode thinking with the default cow', () {
      final repoRoot = resolveRepoRoot();
      final cowsDir =
          '$repoRoot${Platform.pathSeparator}code${Platform.pathSeparator}specs${Platform.pathSeparator}cows';
      final invocation = baseInvocation(message: 'beep', activeModes: ['borg'], think: true);
      final result = renderCowsay(invocation, cowsDir);
      expect(result, isA<PaintVmAsciiOk>());
      expect(
        (result as PaintVmAsciiOk).text,
        [
          ' ______',
          '( beep )',
          ' ------',
          '        o   ^__^',
          '         o  (==)\\_______',
          '            (__)\\       )\\/\\',
          '                ||----w |',
          '                ||     ||',
        ].join('\n'),
      );
    });
  });
}
