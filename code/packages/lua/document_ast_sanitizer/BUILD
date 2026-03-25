eval $(luarocks path --bin)
cd ../../document_ast && luarocks make coding-adventures-document-ast-0.1.0-1.rockspec
cd ../document_ast_sanitizer && luarocks make coding-adventures-document-ast-sanitizer-0.1.0-1.rockspec
busted spec/ --verbose
