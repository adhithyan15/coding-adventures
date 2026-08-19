/// cowsay (Dart) — entry point.
///
/// Thin CLI wiring: load and parse `code/specs/cowsay.json` via
/// CliBuilder, resolve the parsed flags/arguments into a
/// [CowsayInvocation], and hand off to [renderCowsay] for the actual
/// formatting + paint-vm-ascii render. See
/// `code/specs/cowsay-paintvm-pipeline.md` for the design.
///
/// CliBuilder's `Parser.parse()` follows the C/Go argv convention where
/// index 0 is the program name (`argv.first` in `Parser.parse()`); Dart's
/// `args: List<String>` passed to `main` does NOT include the program name
/// — passing it straight through would silently drop the first real CLI
/// token, the same pitfall documented for every other port (see
/// lessons.md, "C#"/"Haskell"/"Java"/"Kotlin" sections).
///
/// Explicitly forces UTF-8 encoding on stdout/stderr and writes output with
/// a literal `\n` (never `print`/`writeln`, which translate a trailing
/// newline to `Platform.lineTerminator` — CRLF on Windows) so output is
/// always LF-only, matching the fix applied to every JVM port after their
/// own default-encoding/newline surprises on Windows (see lessons.md).
library cowsay_bin;

import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_cli_builder/cli_builder.dart';
import 'package:coding_adventures_paint_vm_ascii/coding_adventures_paint_vm_ascii.dart'
    hide version;
import 'package:cowsay/src/cowsay.dart';

Future<void> main(List<String> args) async {
  stdout.encoding = utf8;
  stderr.encoding = utf8;

  final repoRoot = findRepoRoot(Directory.current.path);
  final specPath = '$repoRoot${Platform.pathSeparator}code${Platform.pathSeparator}'
      'specs${Platform.pathSeparator}cowsay.json';
  final cowsDir = '$repoRoot${Platform.pathSeparator}code${Platform.pathSeparator}'
      'specs${Platform.pathSeparator}cows';

  final argv = ['cowsay', ...args];

  try {
    final outcome = Parser.fromPath(specPath, argv).parse();
    switch (outcome) {
      case HelpResult(:final text):
        stdout.write('$text\n');
      case VersionResult(:final version):
        stdout.write('$version\n');
      case ParseResult():
        await _run(outcome, cowsDir);
      default:
        throw StateError('unexpected parse outcome: $outcome');
    }
  } on CliBuilderError catch (error) {
    stderr.write('${error.message}\n');
    await stderr.flush();
    exit(1);
  } on FileSystemException catch (error) {
    // Reading the CLI spec, a .cow template, or the cows directory listing
    // can all fail with a FileSystemException (missing file, permissions,
    // a broken repo-root discovery). Report it the same way a
    // CliBuilderError is reported, rather than letting a raw stack trace
    // reach the user.
    stderr.write('${error.message}: ${error.path}\n');
    await stderr.flush();
    exit(1);
  }

  await stdout.flush();
}

Future<void> _run(ParseResult result, String cowsDir) async {
  final flags = result.flags;
  final arguments = result.arguments;

  if (isListRequested(flags)) {
    for (final name in listCowFiles(cowsDir)) {
      stdout.write('$name\n');
    }
    return;
  }

  var message = resolveMessageFromArguments(arguments);
  if (message == null) {
    if (stdin.hasTerminal) return;
    final bytes = await stdin.fold<List<int>>(
        <int>[], (accumulated, chunk) => accumulated..addAll(chunk));
    // allowMalformed: true replaces an invalid byte sequence with U+FFFD
    // instead of throwing a FormatException — matching the JVM ports, whose
    // `String(bytes, StandardCharsets.UTF_8)` never throws on malformed
    // input either. A CLI's stdin is not a boundary worth hard-failing on.
    message = utf8.decode(bytes, allowMalformed: true).trim();
  }

  if (message.isEmpty) return;

  final invocation = buildInvocation(message, flags);
  final renderResult = renderCowsay(invocation, cowsDir);
  switch (renderResult) {
    case PaintVmAsciiOk(:final text):
      stdout.write('$text\n');
    case PaintVmAsciiErr(:final error):
      stderr.write('$error\n');
      await stderr.flush();
      exit(1);
  }
}
