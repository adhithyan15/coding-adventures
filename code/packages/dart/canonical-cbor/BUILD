dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95
python3 -c "import pathlib, re; spec = pathlib.Path('../../../specs/fixtures/canonical-cbor-v1/cases.json').read_text('utf-8').strip(); local = pathlib.Path('test/vectors_test.dart').read_text('utf-8'); m = re.search(r'const String canonicalCborCasesJson = r\x27\x27\x27([^\x27]*)\x27\x27\x27;', local); assert m is not None and m.group(1).strip() == spec, 'test/vectors_test.dart out of sync with cases.json'"
