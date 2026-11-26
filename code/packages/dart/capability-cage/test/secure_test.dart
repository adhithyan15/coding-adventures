// Unit tests for the slice-3 secure wrappers: the 4 + 6 reference
// tests from `secure_env.rs`/`secure_file.rs` mirrored case-for-case,
// plus lane extras (wrong-action denial, live temp-dir round trips,
// denial-before-OS-touch, writeVar ordering, glob edges).

import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_capability_cage/coding_adventures_capability_cage.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

Uint8List _bytes(String s) => Uint8List.fromList(s.codeUnits);

Manifest _manifestWith(Category category, Action action, String target) =>
    Manifest([
      Capability(
        category: category,
        action: action,
        target: target,
        justification: 'test',
      ),
    ]);

void main() {
  group('secure env', () {
    test('readVar calls the backend when allowed', () async {
      final backend = TestBackend().withEnvResponse('TOKEN', 'secret');
      await runWithBackendAsync(backend, () async {
        final manifest = _manifestWith(Category.env, Action.read, 'TOKEN');

        final value = await readVar(manifest, 'TOKEN');

        expect(value, equals('secret'));
        expect(backend.calls(), equals([const ReadEnvCall('TOKEN')]));
      });
    });

    test('readVar denied without manifest entry', () async {
      final backend = TestBackend().withEnvResponse('TOKEN', 'secret');
      await runWithBackendAsync(backend, () async {
        try {
          await readVar(Manifest.empty(), 'TOKEN');
          fail('expected a CapabilityViolationError');
        } on CapabilityViolationError catch (e) {
          expect(e.category, equals(Category.env));
          expect(e.action, equals(Action.read));
          expect(e.target, equals('TOKEN'));
        }
        // The backend is never touched on denial.
        expect(
          backend.calls(),
          isEmpty,
          reason: 'backend was called on denial',
        );
      });
    });

    test('writeVar calls the backend when allowed', () async {
      final backend = TestBackend();
      await runWithBackendAsync(backend, () async {
        final manifest = _manifestWith(Category.env, Action.write, 'MODE');

        await writeVar(manifest, 'MODE', 'test');

        expect(
          backend.calls(),
          equals([const WriteEnvCall('MODE', 'test')]),
        );
      });
    });

    test('glob target matches a variable family', () async {
      final backend = TestBackend().withEnvResponse('APP_TOKEN', 'secret');
      await runWithBackendAsync(backend, () async {
        final manifest = _manifestWith(Category.env, Action.read, 'APP_*');

        expect(
          await readVar(manifest, 'APP_TOKEN'),
          equals('secret'),
        );
      });
    });

    // --- Lane extras ------------------------------------------------------

    test('read grant does not authorize writes', () async {
      final backend = TestBackend();
      await runWithBackendAsync(backend, () async {
        final manifest = _manifestWith(Category.env, Action.read, 'TOKEN');

        await expectLater(
          writeVar(manifest, 'TOKEN', 'x'),
          throwsA(isA<CapabilityViolationError>()),
        );
        expect(backend.calls(), isEmpty);
      });
    });

    test('live writeVar refuses after the manifest check', () async {
      // Allowed by the manifest: the failure comes from the platform
      // (dart:io cannot set process variables).
      final allowed = _manifestWith(Category.env, Action.write, 'MODE');
      await expectLater(
        writeVar(allowed, 'MODE', 'x'),
        throwsA(isA<UnsupportedError>()),
      );
      // Denied by the manifest: the violation wins, no backend contact.
      await expectLater(
        writeVar(Manifest.empty(), 'MODE', 'x'),
        throwsA(isA<CapabilityViolationError>()),
      );
    });
  });

  group('secure file', () {
    test('readFile calls the backend when allowed', () async {
      final backend =
          TestBackend().withResponse('./allowed.txt', _bytes('hello'));
      await runWithBackendAsync(backend, () async {
        final manifest =
            _manifestWith(Category.fs, Action.read, './allowed.txt');

        final bytes = await readFile(manifest, './allowed.txt');

        expect(bytes, equals(_bytes('hello')));
      });
    });

    test('readFile denied without manifest entry', () async {
      final backend = TestBackend();
      await runWithBackendAsync(backend, () async {
        try {
          await readFile(Manifest.empty(), './forbidden.txt');
          fail('expected a CapabilityViolationError');
        } on CapabilityViolationError catch (e) {
          expect(e.category, equals(Category.fs));
          expect(e.action, equals(Action.read));
          expect(e.target, equals('./forbidden.txt'));
        }
      });
    });

    test('readFile does not call the backend when denied', () async {
      final backend = TestBackend();
      await runWithBackendAsync(backend, () async {
        try {
          await readFile(Manifest.empty(), './forbidden.txt');
          fail('expected denial');
        } on CapabilityViolationError {
          // Expected.
        }
        expect(
          backend.calls(),
          isEmpty,
          reason: 'backend was called on denial',
        );
      });
    });

    test('write/create/delete round trip', () async {
      final backend = TestBackend();
      await runWithBackendAsync(backend, () async {
        Manifest manifestFor(Action action) =>
            _manifestWith(Category.fs, action, './out.txt');

        await writeFile(manifestFor(Action.write), './out.txt', _bytes('data'));
        await createFile(manifestFor(Action.create), './out.txt');
        await deleteFile(manifestFor(Action.delete), './out.txt');

        expect(
          backend.calls(),
          equals([
            WriteFileCall('./out.txt', _bytes('data')),
            const CreateFileCall('./out.txt'),
            const DeleteFileCall('./out.txt'),
          ]),
        );
      });
    });

    test('listDir with manifest', () async {
      final backend =
          TestBackend().withDirResponse('./tmp', ['a.txt', 'b.txt']);
      await runWithBackendAsync(backend, () async {
        final manifest = _manifestWith(Category.fs, Action.list, './tmp');

        expect(
          await listDir(manifest, './tmp'),
          equals(['a.txt', 'b.txt']),
        );
      });
    });

    test('glob target matches a specific path', () async {
      final backend =
          TestBackend().withResponse('./logs/2026.txt', _bytes('x'));
      await runWithBackendAsync(backend, () async {
        final manifest =
            _manifestWith(Category.fs, Action.read, './logs/*.txt');

        await expectLater(
          readFile(manifest, './logs/2026.txt'),
          completes,
        );
      });
    });

    // --- Lane extras ------------------------------------------------------

    test('live round trip through a temp dir', () async {
      final dir = await Directory.systemTemp.createTemp('cage-secure-');
      try {
        Manifest manifestFor(Action action) =>
            _manifestWith(Category.fs, action, '${dir.path}/*');

        final file = '${dir.path}/note.txt';
        final listManifest = _manifestWith(Category.fs, Action.list, dir.path);
        await createFile(manifestFor(Action.create), file);
        await writeFile(manifestFor(Action.write), file, _bytes('hello'));
        expect(
          await readFile(manifestFor(Action.read), file),
          equals(_bytes('hello')),
        );
        expect(
          await listDir(listManifest, dir.path),
          equals(['note.txt']),
        );
        await deleteFile(manifestFor(Action.delete), file);
        expect(await listDir(listManifest, dir.path), isEmpty);
      } finally {
        await dir.delete(recursive: true);
      }
    });

    test('denial touches no filesystem state', () async {
      final dir = await Directory.systemTemp.createTemp('cage-denied-');
      try {
        final file = '${dir.path}/nope.txt';
        // Nothing granted: create must fail before the backend runs.
        await expectLater(
          createFile(Manifest.empty(), file),
          throwsA(isA<CapabilityViolationError>()),
        );
        expect(File(file).existsSync(), isFalse);
      } finally {
        await dir.delete(recursive: true);
      }
    });

    test('NUL paths stop at the live backend guard', () async {
      // The '*' glob covers the string, so the manifest allows and the
      // wrapper delegates; the live backend's NUL guard (slice 2) is
      // the backstop that keeps the truncated OS act from escaping.
      final manifest = _manifestWith(Category.fs, Action.read, '*');
      final nul = String.fromCharCode(0);
      await expectLater(
        readFile(manifest, 'a${nul}b'),
        throwsA(
          isA<BackendError>().having(
            (e) => e.message,
            'message',
            'path contains a NUL byte',
          ),
        ),
      );
    });
  });

  group('async secure campaign', () {
    test('seeded scripts gate every call on the manifest', () async {
      await runSecureCampaign(scripts: 500, seed: BigInt.from(381));
    });

    test('second seed gates every call on the manifest', () async {
      await runSecureCampaign(scripts: 500, seed: BigInt.from(382));
    });
  });
}

final List<String> _securePathPool = [
  '',
  'a',
  './allowed.txt',
  './logs/2026.txt',
  '/abs/path',
  'C:/win/path',
  String.fromCharCode(0),
  'x' * 200,
  'sp ace/f',
  '../escape',
  'a/b/../c',
];

final List<String> _secureNamePool = [
  '',
  'TOKEN',
  'APP_TOKEN',
  'a',
  String.fromCharCode(0),
  'name with spaces',
  'PATH',
];

/// One scripted wrapper invocation with its expected triple.
class _SecureOp {
  final Category category;
  final Action action;
  final String target;

  const _SecureOp(this.category, this.action, this.target);
}

/// Seeded differential campaign over the secure wrappers.
///
/// Each script builds a random manifest (0-3 capabilities over hostile
/// targets, sometimes with `*` globs), installs a scripted [TestBackend]
/// in a zone, and invokes random wrappers. The model asserts the two
/// properties the wrappers own: (1) the denial/success verdict equals
/// `manifest.has` on the wrapper's triple, and the denial carries that
/// exact triple (this pins that each wrapper checks the RIGHT
/// category/action/target — e.g. [writeFile] reports [Action.write]);
/// (2) the backend log grows if and only if the call was allowed, with
/// scripted outputs returned verbatim. Only [CapabilityViolationError]
/// may escape.
Future<void> runSecureCampaign({
  required int scripts,
  required BigInt seed,
}) async {
  final rng = SplitMix64(seed);

  String randomTarget() {
    // Empty targets cannot construct a Capability (EmptyTarget); the
    // wrappers still exercise empty *request* paths below.
    while (true) {
      final pick = rng.bounded(14);
      final target = pick < _securePathPool.length
          ? _securePathPool[pick]
          : ['*', './logs/*', 'APP_*'][pick - _securePathPool.length];
      if (target.isNotEmpty) return target;
    }
  }

  for (var s = 0; s < scripts; s++) {
    final backend = TestBackend();
    final caps = <Capability>[];
    final capCount = rng.bounded(4);
    for (var i = 0; i < capCount; i++) {
      final fs = rng.bounded(2) == 0;
      final action = fs
          ? Action.values[rng.bounded(5)]
          : (rng.bounded(2) == 0 ? Action.read : Action.write);
      // Only taxonomy-valid pairs (fs read/write/create/delete/list,
      // env read/write); the Capability constructor enforces the rest.
      final category = fs ? Category.fs : Category.env;
      caps.add(
        Capability(
          category: category,
          action: action,
          target: randomTarget(),
          justification: 'campaign',
        ),
      );
    }
    // The manifest intentionally skips RWS validation: overlapping
    // test capabilities are the point, and enforcement reads `has`.
    final manifest = Manifest(caps);

    await runWithBackendAsync(backend, () async {
      final opCount = rng.bounded(8) + 1;
      for (var op = 0; op < opCount; op++) {
        final kind = rng.bounded(7);
        final path = _securePathPool[rng.bounded(_securePathPool.length)];
        final name = _secureNamePool[rng.bounded(_secureNamePool.length)];
        final expected = switch (kind) {
          0 => _SecureOp(Category.fs, Action.read, path),
          1 => _SecureOp(Category.fs, Action.write, path),
          2 => _SecureOp(Category.fs, Action.create, path),
          3 => _SecureOp(Category.fs, Action.delete, path),
          4 => _SecureOp(Category.fs, Action.list, path),
          5 => _SecureOp(Category.env, Action.read, name),
          _ => _SecureOp(Category.env, Action.write, name),
        };
        final allowed = manifest.has(
          expected.category,
          expected.action,
          expected.target,
        );
        final callsBefore = backend.calls().length;

        try {
          switch (kind) {
            case 0:
              await readFile(manifest, path);
            case 1:
              await writeFile(manifest, path, Uint8List.fromList([1, 2]));
            case 2:
              await createFile(manifest, path);
            case 3:
              await deleteFile(manifest, path);
            case 4:
              await listDir(manifest, path);
            case 5:
              await readVar(manifest, name);
            default:
              await writeVar(manifest, name, 'v');
          }
          if (!allowed) {
            throw StateError(
              'script $s op $op: ${expected.category}:${expected.action}:'
              '${expected.target} succeeded but manifest denies',
            );
          }
          if (backend.calls().length != callsBefore + 1) {
            throw StateError('script $s op $op: allowed call not logged');
          }
        } on CapabilityViolationError catch (e) {
          if (allowed) {
            throw StateError(
              'script $s op $op: manifest allows but wrapper denied',
            );
          }
          // The denial must name the wrapper's own triple exactly.
          if (e.category != expected.category ||
              e.action != expected.action ||
              e.target != expected.target) {
            throw StateError(
              'script $s op $op: denial names ${e.category}:${e.action}:'
              '${e.target}, want ${expected.category}:${expected.action}:'
              '${expected.target}',
            );
          }
          if (backend.calls().length != callsBefore) {
            throw StateError('script $s op $op: denied call reached backend');
          }
        }
      }
    });
  }
}
