// Unit tests for the slice-2 backends: the 6 reference tests from
// `backend.rs` mirrored case-for-case, plus lane extras (zone scoping,
// nesting, exception safety, async propagation, a live OpenBackend
// temp-dir round trip, first-match scripted lookups, and call-log
// snapshots).

import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_capability_cage/coding_adventures_capability_cage.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

Uint8List _bytes(String s) => Uint8List.fromList(s.codeUnits);

void main() {
  group('backend', () {
    test('deny-all refuses everything', () async {
      const b = DenyAllBackend();
      await expectLater(
        b.readFile('./x'),
        throwsA(
          isA<BackendError>().having(
            (e) => e.message,
            'message',
            'DenyAllBackend refused fs.read',
          ),
        ),
      );
      await expectLater(
        b.writeFile('./x', _bytes('y')),
        throwsA(
          isA<BackendError>().having(
            (e) => e.message,
            'message',
            'DenyAllBackend refused fs.write',
          ),
        ),
      );
      await expectLater(
        b.createFile('./x'),
        throwsA(
          isA<BackendError>().having(
            (e) => e.message,
            'message',
            'DenyAllBackend refused fs.create',
          ),
        ),
      );
      await expectLater(
        b.deleteFile('./x'),
        throwsA(
          isA<BackendError>().having(
            (e) => e.message,
            'message',
            'DenyAllBackend refused fs.delete',
          ),
        ),
      );
      await expectLater(
        b.listDir('./x'),
        throwsA(
          isA<BackendError>().having(
            (e) => e.message,
            'message',
            'DenyAllBackend refused fs.list',
          ),
        ),
      );
      await expectLater(
        b.readEnv('TOKEN'),
        throwsA(
          isA<BackendError>().having(
            (e) => e.message,
            'message',
            'DenyAllBackend refused env.read',
          ),
        ),
      );
      await expectLater(
        b.writeEnv('TOKEN', 'secret'),
        throwsA(
          isA<BackendError>().having(
            (e) => e.message,
            'message',
            'DenyAllBackend refused env.write',
          ),
        ),
      );
    });

    test('test backend records and returns scripted response', () async {
      final b = TestBackend().withResponse('./foo', _bytes('hello'));
      final bytes = await b.readFile('./foo');

      expect(bytes, equals(_bytes('hello')));
      expect(b.calls(), equals([const ReadFileCall('./foo')]));
    });

    test('test backend unscripted read returns empty', () async {
      final b = TestBackend();
      final bytes = await b.readFile('./missing');

      expect(bytes, isEmpty);
    });

    test('test backend records writes', () async {
      final b = TestBackend();
      await b.writeFile('./x', _bytes('data'));
      await b.createFile('./y');
      await b.deleteFile('./z');

      expect(
        b.calls(),
        equals([
          WriteFileCall('./x', _bytes('data')),
          const CreateFileCall('./y'),
          const DeleteFileCall('./z'),
        ]),
      );
    });

    test('test backend dir listing', () async {
      final b = TestBackend().withDirResponse('./dir', ['a.txt', 'b.txt']);
      final entries = await b.listDir('./dir');

      expect(entries, equals(['a.txt', 'b.txt']));
    });

    test('test backend records env access', () async {
      final b = TestBackend().withEnvResponse('TOKEN', 'secret');
      expect(await b.readEnv('TOKEN'), equals('secret'));
      await b.writeEnv('MODE', 'test');

      expect(
        b.calls(),
        equals([
          const ReadEnvCall('TOKEN'),
          const WriteEnvCall('MODE', 'test'),
        ]),
      );
    });

    // --- Lane extras ------------------------------------------------------

    test('default ambient backend is open', () {
      expect(currentBackend(), isA<OpenBackend>());
    });

    test('zone override installs and restores', () {
      final testBackend = TestBackend();
      expect(currentBackend(), isA<OpenBackend>());
      runWithBackend(testBackend, () {
        expect(identical(currentBackend(), testBackend), isTrue);
      });
      expect(currentBackend(), isA<OpenBackend>());
    });

    test('zone overrides nest with restoration', () {
      final outer = TestBackend();
      final inner = TestBackend();
      runWithBackend(outer, () {
        expect(identical(currentBackend(), outer), isTrue);
        runWithBackend(inner, () {
          expect(identical(currentBackend(), inner), isTrue);
        });
        expect(identical(currentBackend(), outer), isTrue);
      });
      expect(currentBackend(), isA<OpenBackend>());
    });

    test('zone override restores when the body throws', () {
      expect(
        () => runWithBackend(TestBackend(), () {
          expect(currentBackend(), isA<TestBackend>());
          throw StateError('boom');
        }),
        throwsStateError,
      );
      expect(currentBackend(), isA<OpenBackend>());
    });

    test('async override flows across awaits and restores', () async {
      final testBackend = TestBackend();
      await runWithBackendAsync(testBackend, () async {
        expect(identical(currentBackend(), testBackend), isTrue);
        await Future<void>.delayed(Duration.zero);
        expect(identical(currentBackend(), testBackend), isTrue);
        // The ambient backend is usable for scripted IO inside the zone.
        testBackend.withResponse('./z', _bytes('zz'));
        expect(
          await currentBackend().readFile('./z'),
          equals(_bytes('zz')),
        );
      });
      expect(currentBackend(), isA<OpenBackend>());
    });

    test('open backend round-trips a temp dir', () async {
      const b = OpenBackend();
      final dir = await Directory.systemTemp.createTemp('cage-open-');
      try {
        final file = '${dir.path}/note.txt';
        await b.createFile(file);
        // create_new semantics: creating twice fails.
        await expectLater(
            b.createFile(file), throwsA(isA<FileSystemException>()));
        await b.writeFile(file, _bytes('hello'));
        expect(await b.readFile(file), equals(_bytes('hello')));
        expect(await b.listDir(dir.path), equals(['note.txt']));
        await b.deleteFile(file);
        expect(await b.listDir(dir.path), isEmpty);
      } finally {
        await dir.delete(recursive: true);
      }
    });

    test('open backend reads the live environment', () async {
      const b = OpenBackend();
      // PATH is set in every CI/dev environment this package runs in.
      expect(await b.readEnv('PATH'), equals(Platform.environment['PATH']));
      expect(
        await b.readEnv('CODING_ADVENTURES_DEFINITELY_ABSENT_VAR'),
        isNull,
      );
    });

    test('open backend env writes are unsupported by the platform', () async {
      const b = OpenBackend();
      await expectLater(
        b.writeEnv('X', 'y'),
        throwsA(isA<UnsupportedError>()),
      );
    });

    test('scripted lookups return the first match', () async {
      final b = TestBackend()
          .withResponse('./dup', _bytes('first'))
          .withResponse('./dup', _bytes('second'))
          .withDirResponse('./dupdir', ['a'])
          .withDirResponse('./dupdir', ['b'])
          .withEnvResponse('DUP', 'one')
          .withEnvResponse('DUP', 'two');

      expect(await b.readFile('./dup'), equals(_bytes('first')));
      expect(await b.listDir('./dupdir'), equals(['a']));
      expect(await b.readEnv('DUP'), equals('one'));
    });

    test('unscripted dir and env reads return empty', () async {
      final b = TestBackend();
      expect(await b.listDir('./nothing'), isEmpty);
      expect(await b.readEnv('NOTHING'), isNull);
    });

    test('call log snapshots are independent copies', () async {
      final b = TestBackend();
      await b.readFile('./a');
      final snapshot = b.calls();
      await b.readFile('./b');

      expect(snapshot, equals([const ReadFileCall('./a')]));
      expect(
        b.calls(),
        equals([const ReadFileCall('./a'), const ReadFileCall('./b')]),
      );
    });

    test('backend errors support value equality', () {
      expect(
        const BackendError('x'),
        equals(const BackendError('x')),
      );
      expect(
        const BackendError('x').hashCode,
        equals(const BackendError('x').hashCode),
      );
      expect(
        const BackendError('x'),
        isNot(equals(const BackendError('y'))),
      );
      expect(const BackendError('x').toString(), equals('x'));
    });

    test('call records render and hash consistently', () {
      final calls = [
        const ReadFileCall('./a'),
        WriteFileCall('./a', _bytes('xy')),
        const CreateFileCall('./a'),
        const DeleteFileCall('./a'),
        const ListDirCall('./a'),
        const ReadEnvCall('N'),
        const WriteEnvCall('N', 'v'),
      ];
      final names = [
        'ReadFileCall',
        'WriteFileCall',
        'CreateFileCall',
        'DeleteFileCall',
        'ListDirCall',
        'ReadEnvCall',
        'WriteEnvCall',
      ];
      for (var i = 0; i < calls.length; i++) {
        expect(calls[i].toString(), contains(names[i]));
        // Twins hash equally (WriteFileCall twin shares bytes).
        final twin = i == 1 ? WriteFileCall('./a', _bytes('xy')) : calls[i];
        expect(calls[i].hashCode, equals(twin.hashCode));
      }
      // Distinct variants with the same target still differ.
      expect(
        const ReadFileCall('./a'),
        isNot(equals(const ListDirCall('./a'))),
      );
    });

    test('open backend rejects NUL paths fail-closed', () async {
      const b = OpenBackend();
      final nul = String.fromCharCode(0);
      // dart:io silently truncates at NUL (writing 'a<NUL>b' creates
      // file 'a'); the backend rejects before any OS call, like the
      // reference's std::fs, so no truncated act can escape a manifest
      // check on the full string.
      final trapped = throwsA(
        isA<BackendError>().having(
          (e) => e.message,
          'message',
          'path contains a NUL byte',
        ),
      );
      await expectLater(b.readFile('a${nul}b'), trapped);
      await expectLater(b.writeFile('a${nul}b', _bytes('x')), trapped);
      await expectLater(b.createFile('a${nul}b'), trapped);
      await expectLater(b.deleteFile('a${nul}b'), trapped);
      await expectLater(b.listDir('a${nul}b'), trapped);
    });
  });

  group('async backend campaign', () {
    test('seeded scripts agree with the model', () async {
      await runAsyncCampaign(scripts: 500, seed: BigInt.from(291));
    });

    test('second seed agrees with the model', () async {
      await runAsyncCampaign(scripts: 500, seed: BigInt.from(292));
    });
  });
}

final List<String> _pathPool = [
  '',
  'a',
  './input.txt',
  '/abs/path',
  'C:/win/path',
  String.fromCharCode(0),
  'x' * 200,
  'dir with spaces/f',
  'uni-' + String.fromCharCode(0xE9) + String.fromCharCode(0x1F600),
  '../escape',
  'a/b/../c',
];

final List<String> _namePool = [
  '',
  'TOKEN',
  'a',
  String.fromCharCode(0),
  'name with spaces',
  'uni-' + String.fromCharCode(0x1F600),
  'PATH',
];

/// Independent model: scripted tables with first-match lookup, the
/// expected call log, and the expected audit keys.
class _BackendModel {
  final List<(String, Uint8List)> files = [];
  final List<(String, List<String>)> dirs = [];
  final List<(String, String)> env = [];
  final List<TestBackendCall> log = [];
  final List<String> auditKeys = [];

  Uint8List readFile(String path) {
    log.add(ReadFileCall(path));
    for (final r in files) {
      if (r.$1 == path) return r.$2;
    }
    return Uint8List(0);
  }

  List<String> listDir(String path) {
    log.add(ListDirCall(path));
    for (final r in dirs) {
      if (r.$1 == path) return r.$2;
    }
    return [];
  }

  String? readEnv(String name) {
    log.add(ReadEnvCall(name));
    for (final r in env) {
      if (r.$1 == name) return r.$2;
    }
    return null;
  }
}

/// Seeded differential campaign over the async backend API.
///
/// The in-lane `fuzz` engine drives synchronous targets only, so this
/// campaign rolls its own deterministic loop on the `pbt` SplitMix64:
/// each script decodes hostile ops (scripted responses interleaved with
/// reads, writes, creates, deletes, listings, env access, denials, and
/// zone checks) and an independent model predicts every output, the
/// full call log, and the audit keys. Only [BackendError] with the
/// exact reference message may escape, and only from [DenyAllBackend].
Future<void> runAsyncCampaign({
  required int scripts,
  required BigInt seed,
}) async {
  final rng = SplitMix64(seed);

  Uint8List randomBytes() {
    final len = rng.bounded(33);
    final buf = Uint8List(len);
    for (var i = 0; i < len; i++) {
      buf[i] = rng.bounded(256);
    }
    return buf;
  }

  String randomPath() => _pathPool[rng.bounded(_pathPool.length)];
  String randomName() => _namePool[rng.bounded(_namePool.length)];

  List<String> randomEntries() => List<String>.generate(
        rng.bounded(5),
        (_) => randomPath(),
      );

  for (var s = 0; s < scripts; s++) {
    final backend = TestBackend();
    final model = _BackendModel();
    final sink = VecAuditSink();
    final deny = const DenyAllBackend();
    final opCount = rng.bounded(10) + 1;

    for (var op = 0; op < opCount; op++) {
      final kind = rng.bounded(10);
      switch (kind) {
        case 0:
          final path = randomPath();
          final data = randomBytes();
          backend.withResponse(path, data);
          model.files.add((path, data));
        case 1:
          final path = randomPath();
          final entries = randomEntries();
          backend.withDirResponse(path, entries);
          model.dirs.add((path, entries));
        case 2:
          final name = randomName();
          final value = randomPath();
          backend.withEnvResponse(name, value);
          model.env.add((name, value));
        case 3:
          final path = randomPath();
          final want = model.readFile(path);
          final got = await runWithBackendAsync(backend, () async {
            if (!identical(currentBackend(), backend)) {
              throw StateError('ambient backend mismatch');
            }
            return currentBackend().readFile(path);
          });
          if (!_equalBytes(got, want)) {
            throw StateError('script $s op $op: read mismatch');
          }
          Operation<Uint8List>.allowed(Category.fs, Action.read, path, got)
              .recordTo(sink);
          model.auditKeys.add('fs:read:$path');
        case 4:
          final path = randomPath();
          final data = randomBytes();
          await backend.writeFile(path, data);
          model.log.add(WriteFileCall(path, data));
          Operation<Null>(
            record: OperationRecord.allowed(Category.fs, Action.write, path),
          ).recordTo(sink);
          model.auditKeys.add('fs:write:$path');
        case 5:
          final path = randomPath();
          if (rng.bounded(2) == 0) {
            await backend.createFile(path);
            model.log.add(CreateFileCall(path));
          } else {
            await backend.deleteFile(path);
            model.log.add(DeleteFileCall(path));
          }
        case 6:
          final path = randomPath();
          final want = model.listDir(path);
          final got = await backend.listDir(path);
          if (got.length != want.length) {
            throw StateError('script $s op $op: listing length mismatch');
          }
          for (var i = 0; i < want.length; i++) {
            if (got[i] != want[i]) {
              throw StateError('script $s op $op: listing mismatch');
            }
          }
        case 7:
          final name = randomName();
          if (rng.bounded(2) == 0) {
            final want = model.readEnv(name);
            final got = await backend.readEnv(name);
            if (got != want) {
              throw StateError('script $s op $op: env mismatch');
            }
          } else {
            final value = randomPath();
            await backend.writeEnv(name, value);
            model.log.add(WriteEnvCall(name, value));
          }
        case 8:
          // DenyAllBackend: exact error, no log, denied audit record.
          final path = randomPath();
          final error = await deny.readFile(path).then<BackendError?>(
              (_) => null,
              onError: (Object e) => e as BackendError?);
          if (error == null ||
              error.message != 'DenyAllBackend refused fs.read') {
            throw StateError('script $s op $op: wrong denial: $error');
          }
          Operation.denied(const CapabilityViolationError(
            category: Category.fs,
            action: Action.read,
            target: './denied',
            message: 'denied',
          )).recordTo(sink);
          model.auditKeys.add('fs:read:./denied');
        default:
          // Pure queries never throw and never touch the log.
          backend.calls();
          sink.records();
          if (!identical(currentBackend(), currentBackend())) {
            throw StateError('ambient backend unstable');
          }
      }
    }

    final calls = backend.calls();
    if (calls.length != model.log.length) {
      throw StateError(
        'script $s: log length ${calls.length} != ${model.log.length}',
      );
    }
    for (var i = 0; i < calls.length; i++) {
      if (calls[i] != model.log[i]) {
        throw StateError('script $s: log[$i] ${calls[i]} != ${model.log[i]}');
      }
    }
    final keys = sink.records().map((r) => r.capabilityKey()).toList();
    if (keys.length != model.auditKeys.length) {
      throw StateError('script $s: audit length mismatch');
    }
    for (var i = 0; i < keys.length; i++) {
      if (keys[i] != model.auditKeys[i]) {
        throw StateError('script $s: audit[$i] mismatch');
      }
    }
  }
}

bool _equalBytes(Uint8List a, Uint8List b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
