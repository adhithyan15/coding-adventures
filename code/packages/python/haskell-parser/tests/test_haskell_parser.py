from haskell_parser import create_haskell_parser, parse_haskell


def test_parser_root_is_file() -> None:
    ast = parse_haskell("x")
    assert ast.rule_name == "file"


def test_create_parser_returns_working_parser() -> None:
    parser = create_haskell_parser("let { x = y } in x", "2010")
    ast = parser.parse()
    assert ast.rule_name == "file"
