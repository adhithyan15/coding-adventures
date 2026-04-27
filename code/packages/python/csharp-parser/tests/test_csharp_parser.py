"""Tests for the C# parser package."""

from __future__ import annotations

import pytest

from lang_parser import ASTNode

from csharp_parser import create_csharp_parser, parse_csharp


def assert_parses_compilation_unit(
    source: str, version: str | None = None
) -> ASTNode:
    """Parse C# source and assert the grammar returns a compilation unit."""
    ast = parse_csharp(source, version)
    assert ast.rule_name == "compilation_unit"
    return ast


class TestClassDeclaration:
    def test_simple_class(self) -> None:
        assert_parses_compilation_unit("class Hello {}")

    def test_public_class(self) -> None:
        assert_parses_compilation_unit("public class Main {}")

    def test_namespace_declaration(self) -> None:
        assert_parses_compilation_unit("namespace MyApp { public class Greeter {} }")

    def test_method_in_class(self) -> None:
        assert_parses_compilation_unit("class Program { void Main() {} }")


class TestCreateCSharpParser:
    def test_creates_parser_with_parse_method(self) -> None:
        parser = create_csharp_parser("public class Foo {}")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result_as_parse_csharp(self) -> None:
        source = "public class Foo {}"
        ast_direct = parse_csharp(source)
        ast_factory = create_csharp_parser(source).parse()

        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)

    def test_factory_with_version(self) -> None:
        ast = create_csharp_parser("public class Foo {}", "8.0").parse()
        assert ast.rule_name == "compilation_unit"


@pytest.mark.parametrize(
    ("version", "source"),
    [
        ("1.0", "public class Foo {}"),
        ("2.0", "public class Foo {}"),
        ("3.0", "public class Foo {}"),
        ("4.0", "public class Foo {}"),
        ("5.0", "public class Foo {}"),
        ("6.0", "public class Foo {}"),
        ("7.0", "public class Foo {}"),
        ("8.0", "public class Foo {}"),
        ("9.0", "public class Foo {}"),
        ("10.0", "public class Foo {}"),
        ("11.0", "public class Foo {}"),
        ("12.0", "public class Foo {}"),
    ],
)
def test_versioned_class_declarations(version: str, source: str) -> None:
    assert_parses_compilation_unit(source, version)


@pytest.mark.parametrize("version", ["9.0", "10.0", "11.0", "12.0", ""])
def test_top_level_statements_supported_in_csharp_9_and_later(version: str) -> None:
    assert_parses_compilation_unit("int x = 1;", version)


def test_no_version_uses_default_grammar() -> None:
    assert_parses_compilation_unit("public class Foo {}")


def test_empty_string_uses_default_grammar() -> None:
    assert_parses_compilation_unit("public class Foo {}", "")


def test_unknown_version_raises_value_error() -> None:
    with pytest.raises(ValueError, match="Unknown C# version"):
        parse_csharp("public class Foo {}", "99.0")


def test_version_propagates_to_factory() -> None:
    ast = create_csharp_parser("public class Foo {}", "8.0").parse()
    assert ast.rule_name == "compilation_unit"
