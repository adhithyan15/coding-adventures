import 'dart:async';
import 'dart:io';

import 'package:mosaic_task_app/mosaic_host.dart';

const _taskName = 'Native acceptance task';
const _persistedTaskName = 'Persisted native task';
const _due = '2026-01-09';
const _schedule = '2026-01-05 → 2026-01-05';

Never _fail(String assertion) =>
    throw StateError('Failed assertion: $assertion');

void _require(bool condition, String assertion) {
  if (!condition) _fail(assertion);
}

Map<String, Object?> _object(Object? value, String assertion) {
  if (value is! Map) _fail('$assertion was not an object');
  final object = Map<String, Object?>.from(value);
  _require(!object.containsKey('error'), '$assertion returned an error');
  return object;
}

Map<String, Object?> _props(Object? update, String assertion) =>
    _object(_object(update, assertion)['props'], '$assertion props');

List<List<Object?>> _rows(Map<String, Object?> props, String assertion) {
  final value = props['task-rows'];
  if (value is! List) _fail('$assertion task-rows was not a list');
  return value
      .map((row) => List<Object?>.from(row as List))
      .toList(growable: false);
}

Future<Map<String, Object?>> _dispatch(
  MosaicHost host,
  String name, [
  Map<String, Object?> payload = const <String, Object?>{},
]) async =>
    _props(
      await host.handleEvent(<String, Object?>{
        'name': name,
        'payload': payload,
      }),
      name,
    );

void _requireTask(List<List<Object?>> rows, String name) {
  _require(rows.length == 1, 'one task row');
  final row = rows.single;
  _require(row.length >= 4, 'task row projection width');
  _require(row[1] == name, 'task name projection');
  _require(row[2] == 'due $_due', 'task due projection');
  _require(row[3] == _schedule, 'Rust schedule start/finish projection');
}

Future<void> main() async {
  final restoredOnLaunch = Platform.environment['MOSAIC_EXPECT_RESTORED'] == '1';
  final host = MosaicHost.loadRequired();
  try {
    var props = _props(await host.props(), 'startup update');
    if (restoredOnLaunch) {
      _requireTask(_rows(props, 'restored startup'), _persistedTaskName);
      props = await _dispatch(host, 'onDeleteTask', <String, Object?>{'index': 0});
      _require(_rows(props, 'restored delete').isEmpty, 'delete restored task');
      stdout.writeln('TaskApp Flutter persisted-restart conformance passed');
      return;
    }

    _require(_rows(props, 'fresh startup').isEmpty, 'fresh task list');
    final before = host.snapshot();
    var rejected = false;
    try {
      await host.handleEvent(<String, Object?>{
        'name': 'onNewTaskNameChange',
        'payload': <String, Object?>{'value': 7},
      });
    } catch (_) {
      rejected = true;
    }
    _require(rejected, 'invalid input rejected');
    _require(host.snapshot().toString() == before.toString(), 'invalid input preserved state');

    await _dispatch(host, 'onNewTaskNameChange', <String, Object?>{'value': _taskName});
    await _dispatch(host, 'onNewTaskDueChange', <String, Object?>{'value': _due});
    props = await _dispatch(host, 'onAddTask');
    _require(_rows(props, 'created task').single[3] == '', 'Board mode hides schedule');
    props = await _dispatch(host, 'onToggleProjectComplexity');
    _requireTask(_rows(props, 'created task'), _taskName);

    props = await _dispatch(host, 'onToggleTask', <String, Object?>{'index': 0});
    _require(_rows(props, 'completed task').single[0] == '✓', 'complete task');
    _require(props['ring-percent'] == '100%', 'completion projection');
    props = await _dispatch(host, 'onToggleTask', <String, Object?>{'index': 0});
    _require(_rows(props, 'reopened task').single[0] == '○', 'reopen task');
    props = await _dispatch(host, 'onDeleteTask', <String, Object?>{'index': 0});
    _require(_rows(props, 'deleted task').isEmpty, 'delete task');

    await _dispatch(host, 'onNewTaskNameChange', <String, Object?>{'value': _persistedTaskName});
    await _dispatch(host, 'onNewTaskDueChange', <String, Object?>{'value': _due});
    props = await _dispatch(host, 'onAddTask');
    _requireTask(_rows(props, 'persisted task'), _persistedTaskName);
  } finally {
    host.dispose();
  }

  stdout.writeln('TaskApp Flutter native lifecycle conformance passed');
}
