bundle install --quiet
bundle exec standardrb --no-fix lib/ test/
bundle exec rake test
python3 -c "import pathlib, re; spec = pathlib.Path('../../../specs/fixtures/canonical-cbor-v1/cases.json').read_text('utf-8').strip(); local = pathlib.Path('test/test_vectors.rb').read_text('utf-8'); m = re.search(r'CANONICAL_CBOR_CASES_JSON = <<~\x27JSON\x27\n(.*?)\nJSON', local, re.DOTALL); assert m is not None and m.group(1).strip() == spec, 'test/test_vectors.rb out of sync with cases.json'"
