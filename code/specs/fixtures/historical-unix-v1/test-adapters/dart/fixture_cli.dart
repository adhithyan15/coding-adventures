// This executable belongs to the shared fixture, not the reusable package.
// It adapts a command argument to the package's semantic trace and emits JSON
// for replay_cli.py to compare with the canonical nine-case contract.
import 'dart:convert';

import '../../../../../packages/dart/unix-history-simulator/lib/unix_history.dart';

void main(List<String> args) {
  if (args.length != 1) {
    throw ArgumentError('expected exactly one shell command');
  }
  print(jsonEncode(runCommand(args.single)));
}
