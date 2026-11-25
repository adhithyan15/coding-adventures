/// Capability categories and actions: the cage taxonomy.
///
/// Fixed by spec: 11 categories and 17 actions, of which only 25 of the
/// 187 combinations are meaningful. The valid pairings live in
/// [isValidCombination], enforced by [Capability.new] and checked
/// exhaustively against the shared `taxonomy.json` fixture by the tests.
///
/// The last three categories (`rand`, `thread`, `ui`) name the
/// nondeterminism sources a unit test must control: ambient entropy,
/// in-process concurrency, and views.
library;

/// One of the eleven category types in the cage taxonomy.
enum Category {
  /// Filesystem.
  fs('fs'),

  /// Network.
  net('net'),

  /// Processes.
  proc('proc'),

  /// Environment variables.
  env('env'),

  /// Foreign function calls.
  ffi('ffi'),

  /// Clocks and sleeps.
  time('time'),

  /// Standard input.
  stdin('stdin'),

  /// Standard output.
  stdout('stdout'),

  /// Ambient entropy: the OS random source, or a generator seeded from it.
  rand('rand'),

  /// A concurrent unit of execution (thread, worker, actor) and messages.
  thread('thread'),

  /// A view: reading it, changing it, receiving input from it.
  ui('ui');

  /// Lower-case wire form. Stable across versions.
  final String wire;
  const Category(this.wire);

  /// Parses the wire form, or null for unknown categories.
  static Category? parse(String s) {
    for (final category in Category.values) {
      if (category.wire == s) return category;
    }
    return null;
  }

  @override
  String toString() => wire;
}

/// One of the seventeen action types in the cage taxonomy.
enum Action {
  /// Read.
  read('read'),

  /// Write.
  write('write'),

  /// Create.
  create('create'),

  /// Delete.
  delete('delete'),

  /// List.
  list('list'),

  /// Connect.
  connect('connect'),

  /// Listen.
  listen('listen'),

  /// DNS lookup.
  dns('dns'),

  /// Execute.
  exec('exec'),

  /// Fork.
  fork('fork'),

  /// Signal.
  signal('signal'),

  /// Foreign call.
  call('call'),

  /// Foreign library load.
  load('load'),

  /// Sleep.
  sleep('sleep'),

  /// Start a thread, worker, or actor (`thread` only).
  spawn('spawn'),

  /// Message across a thread boundary (`thread` only).
  message('message'),

  /// Receive user input from a view (`ui` only). Dispatching a synthetic
  /// event changes the view, so it is `ui:write`, not `ui:event`.
  event('event');

  /// Lower-case wire form. Stable across versions.
  final String wire;
  const Action(this.wire);

  /// Parses the wire form, or null for unknown actions.
  static Action? parse(String s) {
    for (final action in Action.values) {
      if (action.wire == s) return action;
    }
    return null;
  }

  @override
  String toString() => wire;
}

/// True iff [category] supports [action] per the cage taxonomy:
///
/// ```text
/// fs       read, write, create, delete, list
/// net      connect, listen, dns
/// proc     exec, fork, signal
/// env      read, write
/// ffi      call, load
/// time     read, sleep
/// stdin    read
/// stdout   write
/// rand     read
/// thread   spawn, message
/// ui       read, write, event
/// ```
bool isValidCombination(Category category, Action action) {
  switch (category) {
    case Category.fs:
      return action == Action.read ||
          action == Action.write ||
          action == Action.create ||
          action == Action.delete ||
          action == Action.list;
    case Category.net:
      return action == Action.connect ||
          action == Action.listen ||
          action == Action.dns;
    case Category.proc:
      return action == Action.exec ||
          action == Action.fork ||
          action == Action.signal;
    case Category.env:
      return action == Action.read || action == Action.write;
    case Category.ffi:
      return action == Action.call || action == Action.load;
    case Category.time:
      return action == Action.read || action == Action.sleep;
    case Category.stdin:
      return action == Action.read;
    case Category.stdout:
      return action == Action.write;
    case Category.rand:
      return action == Action.read;
    case Category.thread:
      return action == Action.spawn || action == Action.message;
    case Category.ui:
      return action == Action.read ||
          action == Action.write ||
          action == Action.event;
  }
}
