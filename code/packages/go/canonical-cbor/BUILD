go test ./... -v -cover
python3 -c "import pathlib; spec = pathlib.Path('../../../specs/fixtures/canonical-cbor-v1/cases.json').read_text('utf-8').strip(); local = pathlib.Path('vectors_test.go').read_text('utf-8'); assert spec in local, 'vectors_test.go out of sync with cases.json'"
