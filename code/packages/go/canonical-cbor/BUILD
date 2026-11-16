go test ./... -v -cover
python3 -c "import pathlib, re; spec = pathlib.Path('../../../specs/fixtures/canonical-cbor-v1/cases.json').read_text('utf-8').strip(); local = pathlib.Path('vectors_test.go').read_text('utf-8'); m = re.search(r'const CanonicalCborCasesJSON = \x60([^\x60]*)\x60', local); assert m is not None and m.group(1).strip() == spec, 'vectors_test.go out of sync with cases.json'"
