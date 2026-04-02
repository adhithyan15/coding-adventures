cd ../grammar_tools && luarocks make --local --no-manifest --no-deps coding-adventures-grammar-tools-0.1.0-1.rockspec
cd ../json_parser && luarocks make --local --no-manifest --no-deps coding-adventures-json-parser-0.1.0-1.rockspec
luarocks make --local --no-manifest --no-deps coding-adventures-json-value-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
