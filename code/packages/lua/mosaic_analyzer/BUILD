(cd ../mosaic_lexer && luarocks make --local coding-adventures-mosaic-lexer-0.1.0-1.rockspec)
(cd ../mosaic_parser && luarocks make --local coding-adventures-mosaic-parser-0.1.0-1.rockspec)
luarocks make --local coding-adventures-mosaic-analyzer-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
