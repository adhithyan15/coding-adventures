// smoke.dart — prove the spreadsheet C ABI is callable from Dart via dart:ffi
// (the path Flutter uses), computing the same results as the other engines.
// Run by verify-native.sh with CAPI_LIB pointing at the built shared library.
import 'dart:ffi';
import 'dart:io';
import 'dart:convert';

final lib = DynamicLibrary.open(Platform.environment['CAPI_LIB']!);
final proc = DynamicLibrary.process(); // libc malloc/free for input buffers

final malloc = proc.lookupFunction<Pointer<Uint8> Function(IntPtr),
    Pointer<Uint8> Function(int)>('malloc');
final freeC = proc.lookupFunction<Void Function(Pointer<Uint8>),
    void Function(Pointer<Uint8>)>('free');

final scNew = lib.lookupFunction<Pointer<Void> Function(),
    Pointer<Void> Function()>('sc_session_new');
final scFree = lib.lookupFunction<Void Function(Pointer<Void>),
    void Function(Pointer<Void>)>('sc_session_free');
final scSet = lib.lookupFunction<
    Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>),
    Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>, Pointer<Uint8>)>('sc_set_cell');
final scGet = lib.lookupFunction<
    Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>),
    Pointer<Uint8> Function(Pointer<Void>, Pointer<Uint8>)>('sc_get_value');
final scStrFree = lib.lookupFunction<Void Function(Pointer<Uint8>),
    void Function(Pointer<Uint8>)>('sc_string_free');

Pointer<Uint8> cstr(String s) {
  final b = utf8.encode(s);
  final p = malloc(b.length + 1);
  for (var i = 0; i < b.length; i++) p[i] = b[i];
  p[b.length] = 0;
  return p;
}

String take(Pointer<Uint8> p) {
  if (p == nullptr) return '(null)';
  var len = 0;
  while (p[len] != 0) len++;
  final s = utf8.decode([for (var i = 0; i < len; i++) p[i]]);
  scStrFree(p);
  return s;
}

void set(Pointer<Void> s, String a1, String raw) {
  final a = cstr(a1), r = cstr(raw);
  take(scSet(s, a, r));
  freeC(a);
  freeC(r);
}

String value(Pointer<Void> s, String a1) {
  final a = cstr(a1);
  final out = take(scGet(s, a));
  freeC(a);
  return out;
}

void main() {
  final s = scNew();
  for (final e in [['B1','15'],['B2','8'],['B3','12'],['B4','4'],['B5','7']]) {
    set(s, e[0], e[1]);
  }
  set(s, 'B6', '=SUM(B1:B5)');
  set(s, 'B7', '=AVERAGE(B1:B5)');
  set(s, 'C1', '=1/0');

  var failures = 0;
  void check(String label, String got, String needle) {
    final ok = got.contains(needle);
    if (!ok) failures++;
    print('${ok ? "ok  " : "FAIL"}  $label: $got');
  }

  check('B6 SUM',        value(s, 'B6'), '"value":46');
  check('B7 AVERAGE',    value(s, 'B7'), '"value":9.2');
  check('C1 div-by-0',   value(s, 'C1'), '#DIV/0!');
  set(s, 'B1', '115');
  check('B6 after edit', value(s, 'B6'), '"value":146');

  scFree(s);
  print(failures == 0 ? '\nALL PASS' : '\n$failures FAILURE(S)');
  exit(failures == 0 ? 0 : 1);
}
