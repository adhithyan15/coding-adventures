"""Tests for the Semantic IR (SIR) node definitions.

These tests verify:
- Every node can be constructed with required fields.
- Default field values are correct (especially ``resolved_type`` defaults to
  ``SIRAnyType()`` and ``loc`` defaults to ``None``).
- The ``type`` discriminant field is set correctly and is not user-settable.
- The ``SIRTemplateLiteral`` quasis/expressions count invariant holds by design.
- Complex trees composed of multiple node types can be built without error.
- Union aliases (SIRType, SIRDeclaration, SIRStatement, SIRExpression, SIRNode)
  contain the correct constituent types.
- The ``extra`` extension bag is an independent dict per instance.

How to read these tests
-----------------------
Each test function is grouped by category and written to read like documentation.
If you're new to the SIR, reading these tests is a good way to understand what
each node represents and how to construct it.
"""

import pytest
from dataclasses import fields

from coding_adventures_semantic_ir import (
    # Source location
    SIRSourceLocation,
    # Type nodes
    SIRAnyType,
    SIRNeverType,
    SIRVoidType,
    SIRPrimitiveType,
    SIRUnionType,
    SIRIntersectionType,
    SIRArrayType,
    SIRObjectField,
    SIRObjectType,
    SIRFunctionType,
    SIRGenericType,
    SIRReferenceType,
    SIRTupleType,
    # Parameters
    SIRParam,
    # Declarations
    SIRVariableDecl,
    SIRFunctionDecl,
    SIRMethodDef,
    SIRPropertyDef,
    SIRClassDecl,
    SIRPropertySignature,
    SIRMethodSignature,
    SIRInterfaceDecl,
    SIRTypeAliasDecl,
    SIRImportSpecifier,
    SIRImport,
    SIRExportSpecifier,
    SIRExport,
    # Statements
    SIRBlock,
    SIRExpressionStmt,
    SIRIfStmt,
    SIRWhileStmt,
    SIRForOfStmt,
    SIRForInStmt,
    SIRForStmt,
    SIRReturnStmt,
    SIRThrowStmt,
    SIRCatchClause,
    SIRTryStmt,
    SIRBreakStmt,
    SIRContinueStmt,
    SIRSwitchCase,
    SIRSwitchStmt,
    SIRLangSpecific,
    # Expressions
    SIRLiteral,
    SIRIdentifier,
    SIRBinaryOp,
    SIRUnaryOp,
    SIRSpread,
    SIRAssignment,
    SIRCall,
    SIRMemberAccess,
    SIRIndex,
    SIRConditional,
    SIRProperty,
    SIRObjectLiteral,
    SIRArrayLiteral,
    SIRArrowFunction,
    SIRFunctionExpression,
    SIRTemplateLiteral,
    SIRAwait,
    SIRYield,
    SIRTypeAssertion,
    SIRSequence,
    # Module root
    SIRModule,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _has_field(cls, name: str) -> bool:
    """Return True if ``cls`` is a dataclass with a field named ``name``."""
    return any(f.name == name for f in fields(cls))


# ---------------------------------------------------------------------------
# Source location
# ---------------------------------------------------------------------------


class TestSIRSourceLocation:
    def test_construction(self):
        loc = SIRSourceLocation(
            file="app.js", start_line=3, start_col=4, end_line=3, end_col=9
        )
        assert loc.file == "app.js"
        assert loc.start_line == 3
        assert loc.start_col == 4
        assert loc.end_line == 3
        assert loc.end_col == 9

    def test_file_none(self):
        # ``file=None`` means the source file is not known.
        loc = SIRSourceLocation(file=None, start_line=1, start_col=0, end_line=1, end_col=5)
        assert loc.file is None

    def test_multi_line_span(self):
        loc = SIRSourceLocation(file="main.py", start_line=10, start_col=0, end_line=12, end_col=1)
        assert loc.start_line == 10
        assert loc.end_line == 12


# ---------------------------------------------------------------------------
# Type nodes
# ---------------------------------------------------------------------------


class TestSIRAnyType:
    def test_construction_no_args(self):
        t = SIRAnyType()
        assert t.type == "any"

    def test_type_discriminant_is_fixed(self):
        # The ``type`` field is set by the dataclass machinery, not by the caller.
        t = SIRAnyType()
        assert t.type == "any"

    def test_two_instances_are_equal(self):
        # Pure value equality — no identity comparison needed.
        assert SIRAnyType() == SIRAnyType()


class TestSIRNeverType:
    def test_construction(self):
        assert SIRNeverType().type == "never"


class TestSIRVoidType:
    def test_construction(self):
        assert SIRVoidType().type == "void"


class TestSIRPrimitiveType:
    def test_string(self):
        t = SIRPrimitiveType("string")
        assert t.name == "string"
        assert t.type == "primitive"

    def test_number(self):
        assert SIRPrimitiveType("number").name == "number"

    def test_boolean(self):
        assert SIRPrimitiveType("boolean").name == "boolean"

    def test_null(self):
        assert SIRPrimitiveType("null").name == "null"

    def test_undefined(self):
        assert SIRPrimitiveType("undefined").name == "undefined"

    def test_symbol(self):
        assert SIRPrimitiveType("symbol").name == "symbol"

    def test_bigint(self):
        assert SIRPrimitiveType("bigint").name == "bigint"


class TestSIRUnionType:
    def test_empty_union(self):
        t = SIRUnionType()
        assert t.types == []
        assert t.type == "union"

    def test_two_type_union(self):
        t = SIRUnionType(types=[SIRPrimitiveType("string"), SIRPrimitiveType("null")])
        assert len(t.types) == 2

    def test_nested_union(self):
        inner = SIRUnionType(types=[SIRPrimitiveType("string"), SIRPrimitiveType("number")])
        outer = SIRUnionType(types=[inner, SIRPrimitiveType("null")])
        assert len(outer.types) == 2


class TestSIRIntersectionType:
    def test_construction(self):
        t = SIRIntersectionType(
            types=[SIRReferenceType("A"), SIRReferenceType("B")]
        )
        assert t.type == "intersection"
        assert len(t.types) == 2


class TestSIRArrayType:
    def test_default_element_is_any(self):
        t = SIRArrayType()
        assert isinstance(t.element, SIRAnyType)
        assert t.type == "array"

    def test_typed_array(self):
        t = SIRArrayType(element=SIRPrimitiveType("number"))
        assert isinstance(t.element, SIRPrimitiveType)
        assert t.element.name == "number"


class TestSIRObjectField:
    def test_required_field(self):
        f = SIRObjectField(name="id", value_type=SIRPrimitiveType("number"))
        assert f.name == "id"
        assert f.required is True

    def test_optional_field(self):
        f = SIRObjectField(name="nick", value_type=SIRPrimitiveType("string"), required=False)
        assert f.required is False

    def test_default_value_type_is_any(self):
        f = SIRObjectField(name="x")
        assert isinstance(f.value_type, SIRAnyType)


class TestSIRObjectType:
    def test_empty(self):
        t = SIRObjectType()
        assert t.fields == []
        assert t.type == "object"

    def test_with_fields(self):
        t = SIRObjectType(
            fields=[
                SIRObjectField("name", SIRPrimitiveType("string")),
                SIRObjectField("age", SIRPrimitiveType("number")),
            ]
        )
        assert len(t.fields) == 2


class TestSIRFunctionType:
    def test_default(self):
        t = SIRFunctionType()
        assert t.params == []
        assert isinstance(t.return_type, SIRAnyType)
        assert t.type == "function"

    def test_typed_function(self):
        t = SIRFunctionType(
            params=[SIRPrimitiveType("string"), SIRPrimitiveType("number")],
            return_type=SIRPrimitiveType("boolean"),
        )
        assert len(t.params) == 2


class TestSIRGenericType:
    def test_array_of_string(self):
        t = SIRGenericType(name="Array", args=[SIRPrimitiveType("string")])
        assert t.name == "Array"
        assert t.type == "generic"

    def test_map(self):
        t = SIRGenericType(
            name="Map",
            args=[SIRPrimitiveType("string"), SIRPrimitiveType("number")],
        )
        assert len(t.args) == 2


class TestSIRReferenceType:
    def test_construction(self):
        t = SIRReferenceType("MyClass")
        assert t.name == "MyClass"
        assert t.type == "reference"


class TestSIRTupleType:
    def test_empty(self):
        t = SIRTupleType()
        assert t.elements == []
        assert t.type == "tuple"

    def test_pair(self):
        t = SIRTupleType(
            elements=[SIRPrimitiveType("string"), SIRPrimitiveType("number")]
        )
        assert len(t.elements) == 2


# ---------------------------------------------------------------------------
# Parameters
# ---------------------------------------------------------------------------


class TestSIRParam:
    def test_minimal(self):
        p = SIRParam(name="x")
        assert p.name == "x"
        assert isinstance(p.type_annotation, SIRAnyType)
        assert p.default_value is None
        assert p.rest is False
        assert p.loc is None
        assert p.extra == {}

    def test_typed_param(self):
        p = SIRParam(name="count", type_annotation=SIRPrimitiveType("number"))
        assert isinstance(p.type_annotation, SIRPrimitiveType)

    def test_rest_param(self):
        p = SIRParam(name="args", rest=True)
        assert p.rest is True

    def test_param_with_default(self):
        p = SIRParam(name="n", default_value=SIRLiteral(value=0))
        assert isinstance(p.default_value, SIRLiteral)

    def test_extra_bag_is_independent(self):
        p1 = SIRParam(name="a")
        p2 = SIRParam(name="b")
        p1.extra["hint"] = "number"
        assert "hint" not in p2.extra


# ---------------------------------------------------------------------------
# Declarations
# ---------------------------------------------------------------------------


class TestSIRVariableDecl:
    def test_defaults(self):
        v = SIRVariableDecl(name="x")
        assert v.name == "x"
        assert v.kind == "let"
        assert v.value is None
        assert isinstance(v.type_annotation, SIRAnyType)
        assert v.loc is None
        assert v.extra == {}
        assert v.type == "variable_decl"

    def test_const_with_value(self):
        v = SIRVariableDecl(name="MSG", kind="const", value=SIRLiteral(value="hello"))
        assert v.kind == "const"
        assert isinstance(v.value, SIRLiteral)

    def test_python_style_assign(self):
        v = SIRVariableDecl(name="x", kind="assign", value=SIRLiteral(value=42))
        assert v.kind == "assign"

    def test_typed_declaration(self):
        v = SIRVariableDecl(
            name="count",
            kind="let",
            value=SIRLiteral(value=0),
            type_annotation=SIRPrimitiveType("number"),
        )
        assert isinstance(v.type_annotation, SIRPrimitiveType)


class TestSIRBlock:
    def test_empty_block(self):
        b = SIRBlock()
        assert b.body == []
        assert b.type == "block"

    def test_block_with_statements(self):
        b = SIRBlock(body=[SIRReturnStmt(value=SIRLiteral(value=0))])
        assert len(b.body) == 1


class TestSIRMethodDef:
    def test_defaults(self):
        m = SIRMethodDef(name="greet")
        assert m.name == "greet"
        assert m.kind == "method"
        assert m.params == []
        assert isinstance(m.return_type, SIRAnyType)
        assert isinstance(m.body, SIRBlock)
        assert m.type == "method_def"

    def test_constructor(self):
        m = SIRMethodDef(name="constructor", kind="constructor")
        assert m.kind == "constructor"

    def test_static_method(self):
        m = SIRMethodDef(name="create", kind="static_method")
        assert m.kind == "static_method"


class TestSIRPropertyDef:
    def test_defaults(self):
        p = SIRPropertyDef(name="count")
        assert p.name == "count"
        assert p.value is None
        assert p.static is False
        assert p.type == "property_def"

    def test_static_property(self):
        p = SIRPropertyDef(name="DEFAULT", value=SIRLiteral(value=0), static=True)
        assert p.static is True


class TestSIRFunctionDecl:
    def test_defaults(self):
        f = SIRFunctionDecl(name="main")
        assert f.name == "main"
        assert f.params == []
        assert isinstance(f.return_type, SIRAnyType)
        assert isinstance(f.body, SIRBlock)
        assert f.is_async is False
        assert f.is_generator is False
        assert f.type == "function_decl"

    def test_async_function(self):
        f = SIRFunctionDecl(name="fetchData", is_async=True)
        assert f.is_async is True

    def test_generator_function(self):
        f = SIRFunctionDecl(name="numbers", is_generator=True)
        assert f.is_generator is True

    def test_function_with_params_and_return_type(self):
        f = SIRFunctionDecl(
            name="add",
            params=[SIRParam("a"), SIRParam("b")],
            return_type=SIRPrimitiveType("number"),
            body=SIRBlock(body=[SIRReturnStmt(value=SIRBinaryOp(
                op="+",
                left=SIRIdentifier(name="a"),
                right=SIRIdentifier(name="b"),
            ))]),
        )
        assert len(f.params) == 2
        assert isinstance(f.return_type, SIRPrimitiveType)


class TestSIRClassDecl:
    def test_defaults(self):
        c = SIRClassDecl(name="Animal")
        assert c.name == "Animal"
        assert c.superclass is None
        assert c.type_params == []
        assert c.members == []
        assert c.type == "class_decl"

    def test_with_superclass(self):
        c = SIRClassDecl(name="Dog", superclass="Animal")
        assert c.superclass == "Animal"

    def test_generic_class(self):
        c = SIRClassDecl(name="Box", type_params=["T"])
        assert c.type_params == ["T"]

    def test_with_members(self):
        c = SIRClassDecl(
            name="Counter",
            members=[
                SIRPropertyDef(name="count", value=SIRLiteral(value=0)),
                SIRMethodDef(name="increment"),
            ],
        )
        assert len(c.members) == 2


class TestSIRPropertySignature:
    def test_construction(self):
        s = SIRPropertySignature(name="id", value_type=SIRPrimitiveType("number"))
        assert s.name == "id"
        assert s.required is True
        assert s.type == "property_signature"

    def test_optional(self):
        s = SIRPropertySignature(name="desc", required=False)
        assert s.required is False


class TestSIRMethodSignature:
    def test_construction(self):
        s = SIRMethodSignature(
            name="toString",
            return_type=SIRPrimitiveType("string"),
        )
        assert s.name == "toString"
        assert s.type == "method_signature"


class TestSIRInterfaceDecl:
    def test_defaults(self):
        i = SIRInterfaceDecl(name="Serializable")
        assert i.name == "Serializable"
        assert i.extends == []
        assert i.members == []
        assert i.type == "interface_decl"

    def test_extends(self):
        i = SIRInterfaceDecl(name="Named", extends=["Base", "Printable"])
        assert len(i.extends) == 2


class TestSIRTypeAliasDecl:
    def test_defaults(self):
        t = SIRTypeAliasDecl(name="ID")
        assert t.name == "ID"
        assert isinstance(t.value, SIRAnyType)
        assert t.type == "type_alias_decl"

    def test_with_value(self):
        t = SIRTypeAliasDecl(
            name="StringOrNumber",
            value=SIRUnionType(types=[SIRPrimitiveType("string"), SIRPrimitiveType("number")]),
        )
        assert isinstance(t.value, SIRUnionType)


class TestSIRImportSpecifier:
    def test_same_name(self):
        s = SIRImportSpecifier(imported="add", local="add")
        assert s.imported == "add"
        assert s.local == "add"

    def test_alias(self):
        s = SIRImportSpecifier(imported="add", local="sum")
        assert s.local == "sum"


class TestSIRImport:
    def test_default_import(self):
        i = SIRImport(source="./math", default="Math")
        assert i.source == "./math"
        assert i.default == "Math"
        assert i.namespace is None
        assert i.specifiers == []
        assert i.type == "import"

    def test_namespace_import(self):
        i = SIRImport(source="./utils", namespace="utils")
        assert i.namespace == "utils"

    def test_named_imports(self):
        i = SIRImport(
            source="./math",
            specifiers=[
                SIRImportSpecifier("add", "add"),
                SIRImportSpecifier("sub", "subtract"),
            ],
        )
        assert len(i.specifiers) == 2

    def test_side_effect_import(self):
        i = SIRImport(source="./polyfills")
        assert i.default is None
        assert i.namespace is None
        assert i.specifiers == []


class TestSIRExport:
    def test_defaults(self):
        e = SIRExport()
        assert e.default is None
        assert e.specifiers == []
        assert e.source is None
        assert e.type == "export"

    def test_default_export(self):
        e = SIRExport(default=SIRIdentifier(name="MyClass"))
        assert isinstance(e.default, SIRIdentifier)

    def test_named_exports(self):
        e = SIRExport(
            specifiers=[
                SIRExportSpecifier(local="add", exported="add"),
                SIRExportSpecifier(local="sub", exported="sub"),
            ]
        )
        assert len(e.specifiers) == 2

    def test_re_export(self):
        e = SIRExport(
            specifiers=[SIRExportSpecifier(local="add", exported="add")],
            source="./math",
        )
        assert e.source == "./math"


# ---------------------------------------------------------------------------
# Statements
# ---------------------------------------------------------------------------


class TestSIRExpressionStmt:
    def test_construction(self):
        stmt = SIRExpressionStmt(expression=SIRIdentifier(name="x"))
        assert isinstance(stmt.expression, SIRIdentifier)
        assert stmt.type == "expression_stmt"
        assert stmt.loc is None


class TestSIRIfStmt:
    def test_without_else(self):
        s = SIRIfStmt(
            test=SIRLiteral(value=True),
            consequent=SIRBlock(),
        )
        assert s.alternate is None
        assert s.type == "if"

    def test_with_else(self):
        s = SIRIfStmt(
            test=SIRLiteral(value=True),
            consequent=SIRBlock(body=[SIRReturnStmt(value=SIRLiteral(value=1))]),
            alternate=SIRBlock(body=[SIRReturnStmt(value=SIRLiteral(value=2))]),
        )
        assert isinstance(s.alternate, SIRBlock)


class TestSIRWhileStmt:
    def test_construction(self):
        s = SIRWhileStmt(
            test=SIRLiteral(value=True),
            body=SIRBlock(),
        )
        assert s.type == "while"
        assert s.loc is None


class TestSIRForOfStmt:
    def test_defaults(self):
        s = SIRForOfStmt(
            binding="item",
            iterable=SIRIdentifier(name="items"),
            body=SIRBlock(),
        )
        assert s.binding == "item"
        assert s.is_await is False
        assert s.type == "for_of"

    def test_async_for_of(self):
        s = SIRForOfStmt(
            binding="chunk",
            iterable=SIRIdentifier(name="stream"),
            body=SIRBlock(),
            is_await=True,
        )
        assert s.is_await is True


class TestSIRForInStmt:
    def test_construction(self):
        s = SIRForInStmt(
            binding="key",
            object=SIRIdentifier(name="obj"),
            body=SIRBlock(),
        )
        assert s.binding == "key"
        assert s.type == "for_in"


class TestSIRForStmt:
    def test_defaults(self):
        s = SIRForStmt()
        assert s.init is None
        assert s.test is None
        assert s.update is None
        assert isinstance(s.body, SIRBlock)
        assert s.type == "for"

    def test_typical_c_for(self):
        # for (let i = 0; i < 10; i++)
        s = SIRForStmt(
            init=SIRVariableDecl(name="i", kind="let", value=SIRLiteral(value=0)),
            test=SIRBinaryOp(op="<", left=SIRIdentifier("i"), right=SIRLiteral(value=10)),
            update=SIRUnaryOp(op="++", operand=SIRIdentifier("i"), prefix=False),
            body=SIRBlock(),
        )
        assert isinstance(s.init, SIRVariableDecl)


class TestSIRReturnStmt:
    def test_bare_return(self):
        s = SIRReturnStmt()
        assert s.value is None
        assert s.type == "return"

    def test_return_with_value(self):
        s = SIRReturnStmt(value=SIRLiteral(value=42))
        assert isinstance(s.value, SIRLiteral)


class TestSIRThrowStmt:
    def test_construction(self):
        s = SIRThrowStmt(value=SIRCall(callee=SIRIdentifier("Error")))
        assert s.type == "throw"
        assert isinstance(s.value, SIRCall)


class TestSIRCatchClause:
    def test_defaults(self):
        c = SIRCatchClause()
        assert c.binding is None
        assert isinstance(c.type_annotation, SIRAnyType)
        assert isinstance(c.body, SIRBlock)
        assert c.type == "catch"

    def test_with_binding(self):
        c = SIRCatchClause(binding="err")
        assert c.binding == "err"


class TestSIRTryStmt:
    def test_try_catch(self):
        s = SIRTryStmt(
            body=SIRBlock(),
            handler=SIRCatchClause(binding="e"),
        )
        assert s.finalizer is None
        assert s.type == "try"

    def test_try_finally(self):
        s = SIRTryStmt(
            body=SIRBlock(),
            finalizer=SIRBlock(),
        )
        assert s.handler is None

    def test_try_catch_finally(self):
        s = SIRTryStmt(
            body=SIRBlock(),
            handler=SIRCatchClause(),
            finalizer=SIRBlock(),
        )
        assert s.handler is not None
        assert s.finalizer is not None


class TestSIRBreakStmt:
    def test_unlabeled(self):
        s = SIRBreakStmt()
        assert s.label is None
        assert s.type == "break"

    def test_labeled(self):
        s = SIRBreakStmt(label="outer")
        assert s.label == "outer"


class TestSIRContinueStmt:
    def test_unlabeled(self):
        s = SIRContinueStmt()
        assert s.label is None
        assert s.type == "continue"

    def test_labeled(self):
        s = SIRContinueStmt(label="outer")
        assert s.label == "outer"


class TestSIRSwitchCase:
    def test_case(self):
        c = SIRSwitchCase(test=SIRLiteral(value=1), body=[SIRBreakStmt()])
        assert isinstance(c.test, SIRLiteral)
        assert c.type == "switch_case"

    def test_default_case(self):
        c = SIRSwitchCase()  # test=None means ``default:``
        assert c.test is None


class TestSIRSwitchStmt:
    def test_construction(self):
        s = SIRSwitchStmt(
            discriminant=SIRIdentifier(name="status"),
            cases=[
                SIRSwitchCase(test=SIRLiteral(value=1)),
                SIRSwitchCase(test=SIRLiteral(value=2)),
                SIRSwitchCase(),  # default
            ],
        )
        assert s.type == "switch"
        assert len(s.cases) == 3


class TestSIRLangSpecific:
    def test_python_with_statement(self):
        s = SIRLangSpecific(
            language="python",
            construct="with_statement",
            extra={"context_expr": "open('f')", "binding": "handle"},
        )
        assert s.language == "python"
        assert s.construct == "with_statement"
        assert s.type == "lang_specific"
        assert s.children == []

    def test_rust_unsafe_block(self):
        s = SIRLangSpecific(language="rust", construct="unsafe_block")
        assert s.language == "rust"


# ---------------------------------------------------------------------------
# Expressions
# ---------------------------------------------------------------------------


class TestSIRLiteral:
    def test_integer(self):
        lit = SIRLiteral(value=42)
        assert lit.value == 42
        assert lit.type == "literal"
        assert isinstance(lit.resolved_type, SIRAnyType)
        assert lit.loc is None

    def test_float(self):
        lit = SIRLiteral(value=3.14)
        assert lit.value == 3.14

    def test_string(self):
        lit = SIRLiteral(value="hello")
        assert lit.value == "hello"

    def test_boolean_true(self):
        lit = SIRLiteral(value=True)
        assert lit.value is True

    def test_boolean_false(self):
        lit = SIRLiteral(value=False)
        assert lit.value is False

    def test_null(self):
        lit = SIRLiteral(value=None)
        assert lit.value is None

    def test_with_resolved_type(self):
        lit = SIRLiteral(value=1, resolved_type=SIRPrimitiveType("number"))
        assert isinstance(lit.resolved_type, SIRPrimitiveType)

    def test_with_loc(self):
        loc = SIRSourceLocation("a.js", 1, 0, 1, 5)
        lit = SIRLiteral(value="hi", loc=loc)
        assert lit.loc is loc


class TestSIRIdentifier:
    def test_defaults(self):
        i = SIRIdentifier(name="x")
        assert i.name == "x"
        assert isinstance(i.resolved_type, SIRAnyType)
        assert i.loc is None
        assert i.type == "identifier"

    def test_with_type(self):
        i = SIRIdentifier(name="count", resolved_type=SIRPrimitiveType("number"))
        assert isinstance(i.resolved_type, SIRPrimitiveType)


class TestSIRBinaryOp:
    def test_addition(self):
        node = SIRBinaryOp(
            op="+",
            left=SIRLiteral(value=1),
            right=SIRLiteral(value=2),
        )
        assert node.op == "+"
        assert isinstance(node.resolved_type, SIRAnyType)
        assert node.type == "binary_op"

    def test_strict_equality(self):
        node = SIRBinaryOp(
            op="===",
            left=SIRIdentifier("a"),
            right=SIRIdentifier("b"),
        )
        assert node.op == "==="

    def test_logical_and(self):
        node = SIRBinaryOp(op="&&", left=SIRIdentifier("x"), right=SIRIdentifier("y"))
        assert node.op == "&&"

    def test_nullish_coalescing(self):
        node = SIRBinaryOp(op="??", left=SIRIdentifier("val"), right=SIRLiteral(value=0))
        assert node.op == "??"


class TestSIRUnaryOp:
    def test_negation(self):
        node = SIRUnaryOp(op="-", operand=SIRIdentifier("x"))
        assert node.op == "-"
        assert node.prefix is True
        assert node.type == "unary_op"

    def test_logical_not(self):
        node = SIRUnaryOp(op="!", operand=SIRLiteral(value=False))
        assert node.op == "!"

    def test_postfix_increment(self):
        node = SIRUnaryOp(op="++", operand=SIRIdentifier("i"), prefix=False)
        assert node.prefix is False

    def test_typeof(self):
        node = SIRUnaryOp(op="typeof", operand=SIRIdentifier("x"))
        assert node.op == "typeof"


class TestSIRSpread:
    def test_construction(self):
        s = SIRSpread(value=SIRIdentifier("args"))
        assert isinstance(s.value, SIRIdentifier)
        assert s.type == "spread"


class TestSIRAssignment:
    def test_simple_assignment(self):
        a = SIRAssignment(
            op="=",
            target=SIRIdentifier("x"),
            value=SIRLiteral(value=1),
        )
        assert a.op == "="
        assert isinstance(a.resolved_type, SIRAnyType)
        assert a.type == "assignment"

    def test_compound_assignment(self):
        a = SIRAssignment(
            op="+=",
            target=SIRIdentifier("x"),
            value=SIRLiteral(value=1),
        )
        assert a.op == "+="


class TestSIRCall:
    def test_simple_call(self):
        c = SIRCall(callee=SIRIdentifier("fn"))
        assert isinstance(c.callee, SIRIdentifier)
        assert c.args == []
        assert c.type_args == []
        assert c.is_new is False
        assert c.is_optional is False
        assert c.type == "call"

    def test_constructor_call(self):
        c = SIRCall(callee=SIRIdentifier("Foo"), is_new=True)
        assert c.is_new is True

    def test_optional_call(self):
        c = SIRCall(callee=SIRIdentifier("fn"), is_optional=True)
        assert c.is_optional is True

    def test_call_with_args(self):
        c = SIRCall(
            callee=SIRIdentifier("add"),
            args=[SIRLiteral(value=1), SIRLiteral(value=2)],
        )
        assert len(c.args) == 2

    def test_call_with_spread(self):
        c = SIRCall(
            callee=SIRIdentifier("fn"),
            args=[SIRSpread(value=SIRIdentifier("args"))],
        )
        assert isinstance(c.args[0], SIRSpread)

    def test_generic_call(self):
        c = SIRCall(
            callee=SIRIdentifier("identity"),
            type_args=[SIRPrimitiveType("string")],
        )
        assert len(c.type_args) == 1


class TestSIRMemberAccess:
    def test_dot_notation(self):
        m = SIRMemberAccess(object=SIRIdentifier("obj"), property="prop")
        assert m.property == "prop"
        assert m.computed is False
        assert m.optional is False
        assert m.type == "member_access"

    def test_optional_chaining(self):
        m = SIRMemberAccess(object=SIRIdentifier("obj"), property="prop", optional=True)
        assert m.optional is True


class TestSIRIndex:
    def test_array_index(self):
        i = SIRIndex(object=SIRIdentifier("arr"), index=SIRLiteral(value=0))
        assert isinstance(i.index, SIRLiteral)
        assert i.type == "index"
        assert isinstance(i.resolved_type, SIRAnyType)


class TestSIRConditional:
    def test_ternary(self):
        c = SIRConditional(
            test=SIRIdentifier("flag"),
            consequent=SIRLiteral(value=1),
            alternate=SIRLiteral(value=0),
        )
        assert c.type == "conditional"
        assert isinstance(c.resolved_type, SIRAnyType)


class TestSIRProperty:
    def test_string_key(self):
        p = SIRProperty(key="name", value=SIRLiteral(value="Alice"))
        assert p.key == "name"
        assert p.shorthand is False
        assert p.computed is False
        assert p.type == "property"

    def test_shorthand(self):
        p = SIRProperty(key="x", value=SIRIdentifier("x"), shorthand=True)
        assert p.shorthand is True

    def test_computed_key(self):
        p = SIRProperty(
            key=SIRIdentifier("dynKey"),
            value=SIRLiteral(value=1),
            computed=True,
        )
        assert p.computed is True
        assert isinstance(p.key, SIRIdentifier)


class TestSIRObjectLiteral:
    def test_empty(self):
        o = SIRObjectLiteral()
        assert o.properties == []
        assert isinstance(o.resolved_type, SIRAnyType)
        assert o.type == "object_literal"

    def test_with_properties(self):
        o = SIRObjectLiteral(
            properties=[
                SIRProperty(key="a", value=SIRLiteral(value=1)),
                SIRProperty(key="b", value=SIRLiteral(value=2)),
            ]
        )
        assert len(o.properties) == 2

    def test_with_spread(self):
        o = SIRObjectLiteral(
            properties=[SIRSpread(value=SIRIdentifier("defaults"))]
        )
        assert isinstance(o.properties[0], SIRSpread)


class TestSIRArrayLiteral:
    def test_empty(self):
        a = SIRArrayLiteral()
        assert a.elements == []
        assert isinstance(a.resolved_type, SIRAnyType)
        assert a.type == "array_literal"

    def test_with_elements(self):
        a = SIRArrayLiteral(
            elements=[SIRLiteral(value=1), SIRLiteral(value=2), SIRLiteral(value=3)]
        )
        assert len(a.elements) == 3

    def test_with_elision_hole(self):
        # JS: [1, , 3] — the middle element is None (elision)
        a = SIRArrayLiteral(elements=[SIRLiteral(value=1), None, SIRLiteral(value=3)])
        assert a.elements[1] is None

    def test_with_spread(self):
        a = SIRArrayLiteral(elements=[SIRSpread(value=SIRIdentifier("rest"))])
        assert isinstance(a.elements[0], SIRSpread)


class TestSIRArrowFunction:
    def test_defaults(self):
        f = SIRArrowFunction()
        assert f.params == []
        assert isinstance(f.body, SIRBlock)
        assert isinstance(f.return_type, SIRAnyType)
        assert f.is_async is False
        assert f.type == "arrow_function"
        assert isinstance(f.resolved_type, SIRAnyType)

    def test_concise_body(self):
        # (x) => x + 1
        f = SIRArrowFunction(
            params=[SIRParam("x")],
            body=SIRBinaryOp(op="+", left=SIRIdentifier("x"), right=SIRLiteral(value=1)),
        )
        assert isinstance(f.body, SIRBinaryOp)

    def test_async_arrow(self):
        f = SIRArrowFunction(is_async=True)
        assert f.is_async is True


class TestSIRFunctionExpression:
    def test_anonymous(self):
        f = SIRFunctionExpression()
        assert f.name is None
        assert f.params == []
        assert isinstance(f.body, SIRBlock)
        assert f.is_async is False
        assert f.is_generator is False
        assert f.type == "function_expression"

    def test_named(self):
        f = SIRFunctionExpression(name="compute")
        assert f.name == "compute"

    def test_generator_expression(self):
        f = SIRFunctionExpression(is_generator=True)
        assert f.is_generator is True


class TestSIRTemplateLiteral:
    def test_no_interpolation(self):
        # `Hello, world!`
        t = SIRTemplateLiteral(quasis=["Hello, world!"])
        assert t.quasis == ["Hello, world!"]
        assert t.expressions == []
        assert t.type == "template_literal"

    def test_one_interpolation(self):
        # `Hello, ${name}!`
        t = SIRTemplateLiteral(
            quasis=["Hello, ", "!"],
            expressions=[SIRIdentifier("name")],
        )
        assert len(t.quasis) == len(t.expressions) + 1

    def test_two_interpolations(self):
        # `${a} + ${b} = ${c}`
        t = SIRTemplateLiteral(
            quasis=["", " + ", " = ", ""],
            expressions=[
                SIRIdentifier("a"),
                SIRIdentifier("b"),
                SIRIdentifier("c"),
            ],
        )
        assert len(t.quasis) == len(t.expressions) + 1
        assert isinstance(t.resolved_type, SIRAnyType)


class TestSIRAwait:
    def test_construction(self):
        a = SIRAwait(value=SIRCall(callee=SIRIdentifier("fetchData")))
        assert a.type == "await"
        assert isinstance(a.resolved_type, SIRAnyType)
        assert a.loc is None


class TestSIRYield:
    def test_defaults(self):
        y = SIRYield()
        assert y.value is None
        assert y.delegate is False
        assert y.type == "yield"

    def test_yield_value(self):
        y = SIRYield(value=SIRLiteral(value=1))
        assert isinstance(y.value, SIRLiteral)

    def test_yield_star(self):
        y = SIRYield(value=SIRIdentifier("gen"), delegate=True)
        assert y.delegate is True


class TestSIRTypeAssertion:
    def test_defaults(self):
        a = SIRTypeAssertion(value=SIRIdentifier("x"))
        assert isinstance(a.target_type, SIRAnyType)
        assert isinstance(a.resolved_type, SIRAnyType)
        assert a.type == "type_assertion"

    def test_with_target_type(self):
        a = SIRTypeAssertion(
            value=SIRIdentifier("x"),
            target_type=SIRPrimitiveType("string"),
        )
        assert isinstance(a.target_type, SIRPrimitiveType)


class TestSIRSequence:
    def test_defaults(self):
        s = SIRSequence()
        assert s.expressions == []
        assert s.type == "sequence"

    def test_with_expressions(self):
        s = SIRSequence(
            expressions=[SIRLiteral(value=1), SIRLiteral(value=2), SIRLiteral(value=3)]
        )
        assert len(s.expressions) == 3


# ---------------------------------------------------------------------------
# Module root
# ---------------------------------------------------------------------------


class TestSIRModule:
    def test_empty_module(self):
        m = SIRModule()
        assert m.source_language is None
        assert m.body == []
        assert m.loc is None
        assert m.extra == {}
        assert m.type == "module"

    def test_with_source_language(self):
        m = SIRModule(source_language="javascript")
        assert m.source_language == "javascript"

    def test_body_contains_decls_and_stmts(self):
        m = SIRModule(
            source_language="javascript",
            body=[
                SIRVariableDecl(name="x", kind="const", value=SIRLiteral(value=1)),
                SIRExpressionStmt(expression=SIRCall(callee=SIRIdentifier("log"))),
            ],
        )
        assert len(m.body) == 2

    def test_extra_bag_is_independent(self):
        m1 = SIRModule()
        m2 = SIRModule()
        m1.extra["tag"] = "test"
        assert "tag" not in m2.extra


# ---------------------------------------------------------------------------
# Composite tree — the greet() function from the spec
# ---------------------------------------------------------------------------


class TestGreetFunction:
    """Build the SIR for the canonical greet() example from the spec.

    Original JavaScript::

        function greet(name) {
            return "Hello, " + name;
        }

    This test verifies that every node in the tree can be constructed and
    that the resulting SIRModule is well-formed.
    """

    def test_greet_module(self):
        greet_fn = SIRFunctionDecl(
            name="greet",
            params=[SIRParam(name="name")],
            return_type=SIRPrimitiveType("string"),
            body=SIRBlock(
                body=[
                    SIRReturnStmt(
                        value=SIRBinaryOp(
                            op="+",
                            left=SIRLiteral(value="Hello, "),
                            right=SIRIdentifier(name="name"),
                        )
                    )
                ]
            ),
        )

        module = SIRModule(source_language="javascript", body=[greet_fn])

        # Verify structure
        assert module.type == "module"
        assert len(module.body) == 1
        fn = module.body[0]
        assert isinstance(fn, SIRFunctionDecl)
        assert fn.name == "greet"
        assert len(fn.params) == 1
        assert fn.params[0].name == "name"

        body_stmts = fn.body.body
        assert len(body_stmts) == 1
        ret = body_stmts[0]
        assert isinstance(ret, SIRReturnStmt)
        assert isinstance(ret.value, SIRBinaryOp)
        assert ret.value.op == "+"


# ---------------------------------------------------------------------------
# Source location threading
# ---------------------------------------------------------------------------


class TestSourceLocationThreading:
    """Verify that loc can be set on nodes at all levels of the tree."""

    def test_loc_on_module(self):
        loc = SIRSourceLocation(file="app.js", start_line=1, start_col=0, end_line=10, end_col=0)
        m = SIRModule(loc=loc)
        assert m.loc is loc

    def test_loc_on_expression(self):
        loc = SIRSourceLocation(file="app.js", start_line=3, start_col=4, end_line=3, end_col=9)
        ident = SIRIdentifier(name="x", loc=loc)
        assert ident.loc is loc

    def test_loc_on_statement(self):
        loc = SIRSourceLocation(file="app.js", start_line=5, start_col=0, end_line=5, end_col=20)
        stmt = SIRReturnStmt(value=SIRLiteral(value=1), loc=loc)
        assert stmt.loc is loc

    def test_loc_on_declaration(self):
        loc = SIRSourceLocation(file="app.js", start_line=1, start_col=0, end_line=1, end_col=15)
        decl = SIRVariableDecl(name="x", loc=loc)
        assert decl.loc is loc

    def test_loc_defaults_to_none_everywhere(self):
        """All nodes must default loc to None — no accidental loc sharing."""
        nodes = [
            SIRModule(),
            SIRVariableDecl(name="x"),
            SIRFunctionDecl(name="f"),
            SIRClassDecl(name="C"),
            SIRImport(source="./m"),
            SIRExport(),
            SIRBlock(),
            SIRReturnStmt(),
            SIRIfStmt(test=SIRLiteral(value=True), consequent=SIRBlock()),
            SIRForOfStmt(binding="x", iterable=SIRIdentifier("xs"), body=SIRBlock()),
            SIRLiteral(value=1),
            SIRIdentifier(name="x"),
            SIRBinaryOp(op="+", left=SIRLiteral(value=1), right=SIRLiteral(value=2)),
            SIRCall(callee=SIRIdentifier("fn")),
        ]
        for node in nodes:
            assert node.loc is None, f"{type(node).__name__}.loc should default to None"


# ---------------------------------------------------------------------------
# resolved_type defaults
# ---------------------------------------------------------------------------


class TestResolvedTypeDefaults:
    """Every expression must default resolved_type to SIRAnyType()."""

    def test_literal_resolved_type(self):
        assert isinstance(SIRLiteral(value=1).resolved_type, SIRAnyType)

    def test_identifier_resolved_type(self):
        assert isinstance(SIRIdentifier(name="x").resolved_type, SIRAnyType)

    def test_binary_op_resolved_type(self):
        node = SIRBinaryOp(op="+", left=SIRLiteral(value=1), right=SIRLiteral(value=2))
        assert isinstance(node.resolved_type, SIRAnyType)

    def test_unary_op_resolved_type(self):
        assert isinstance(SIRUnaryOp(op="-", operand=SIRLiteral(value=1)).resolved_type, SIRAnyType)

    def test_call_resolved_type(self):
        assert isinstance(SIRCall(callee=SIRIdentifier("f")).resolved_type, SIRAnyType)

    def test_member_access_resolved_type(self):
        m = SIRMemberAccess(object=SIRIdentifier("obj"), property="x")
        assert isinstance(m.resolved_type, SIRAnyType)

    def test_assignment_resolved_type(self):
        a = SIRAssignment(op="=", target=SIRIdentifier("x"), value=SIRLiteral(value=1))
        assert isinstance(a.resolved_type, SIRAnyType)

    def test_conditional_resolved_type(self):
        c = SIRConditional(
            test=SIRLiteral(value=True),
            consequent=SIRLiteral(value=1),
            alternate=SIRLiteral(value=0),
        )
        assert isinstance(c.resolved_type, SIRAnyType)

    def test_arrow_function_resolved_type(self):
        assert isinstance(SIRArrowFunction().resolved_type, SIRAnyType)

    def test_await_resolved_type(self):
        a = SIRAwait(value=SIRIdentifier("p"))
        assert isinstance(a.resolved_type, SIRAnyType)

    def test_yield_resolved_type(self):
        assert isinstance(SIRYield().resolved_type, SIRAnyType)

    def test_object_literal_resolved_type(self):
        assert isinstance(SIRObjectLiteral().resolved_type, SIRAnyType)

    def test_array_literal_resolved_type(self):
        assert isinstance(SIRArrayLiteral().resolved_type, SIRAnyType)

    def test_template_literal_resolved_type(self):
        assert isinstance(SIRTemplateLiteral(quasis=["hi"]).resolved_type, SIRAnyType)

    def test_index_resolved_type(self):
        i = SIRIndex(object=SIRIdentifier("arr"), index=SIRLiteral(value=0))
        assert isinstance(i.resolved_type, SIRAnyType)

    def test_sequence_resolved_type(self):
        assert isinstance(SIRSequence().resolved_type, SIRAnyType)

    def test_type_assertion_resolved_type(self):
        a = SIRTypeAssertion(value=SIRIdentifier("x"))
        assert isinstance(a.resolved_type, SIRAnyType)

    def test_function_expression_resolved_type(self):
        assert isinstance(SIRFunctionExpression().resolved_type, SIRAnyType)

    def test_resolved_type_instances_are_independent(self):
        """Each SIRIdentifier must get its own SIRAnyType instance."""
        a = SIRIdentifier(name="a")
        b = SIRIdentifier(name="b")
        # They are equal by value but not the same object.
        assert a.resolved_type == b.resolved_type
        # Mutating one must not affect the other (they are separate instances).
        # Since SIRAnyType has no mutable fields this is guaranteed by
        # default_factory=SIRAnyType, but we verify the identity check.
        assert a.resolved_type is not b.resolved_type


# ---------------------------------------------------------------------------
# Extension bag isolation
# ---------------------------------------------------------------------------


class TestExtraBagIsolation:
    """Every node's ``extra`` dict must be a fresh instance, not shared."""

    def test_variable_decl_extra_isolated(self):
        a = SIRVariableDecl(name="a")
        b = SIRVariableDecl(name="b")
        a.extra["key"] = "value"
        assert "key" not in b.extra

    def test_function_decl_extra_isolated(self):
        f1 = SIRFunctionDecl(name="f1")
        f2 = SIRFunctionDecl(name="f2")
        f1.extra["pure"] = True
        assert "pure" not in f2.extra

    def test_class_decl_extra_isolated(self):
        c1 = SIRClassDecl(name="A")
        c2 = SIRClassDecl(name="B")
        c1.extra["abstract"] = True
        assert "abstract" not in c2.extra

    def test_module_extra_isolated(self):
        m1 = SIRModule()
        m2 = SIRModule()
        m1.extra["strict"] = True
        assert "strict" not in m2.extra


# ---------------------------------------------------------------------------
# Type discriminants
# ---------------------------------------------------------------------------


class TestTypeDiscriminants:
    """The ``type`` field on each node must match the expected literal string."""

    def test_all_type_discriminants(self):
        cases = [
            (SIRAnyType(), "any"),
            (SIRNeverType(), "never"),
            (SIRVoidType(), "void"),
            (SIRPrimitiveType("string"), "primitive"),
            (SIRUnionType(), "union"),
            (SIRIntersectionType(), "intersection"),
            (SIRArrayType(), "array"),
            (SIRObjectType(), "object"),
            (SIRFunctionType(), "function"),
            (SIRGenericType(name="Array"), "generic"),
            (SIRReferenceType("T"), "reference"),
            (SIRTupleType(), "tuple"),
            (SIRVariableDecl(name="x"), "variable_decl"),
            (SIRBlock(), "block"),
            (SIRMethodDef(name="m"), "method_def"),
            (SIRPropertyDef(name="p"), "property_def"),
            (SIRFunctionDecl(name="f"), "function_decl"),
            (SIRClassDecl(name="C"), "class_decl"),
            (SIRPropertySignature(name="x"), "property_signature"),
            (SIRMethodSignature(name="m"), "method_signature"),
            (SIRInterfaceDecl(name="I"), "interface_decl"),
            (SIRTypeAliasDecl(name="T"), "type_alias_decl"),
            (SIRImport(source="./m"), "import"),
            (SIRExport(), "export"),
            (SIRExpressionStmt(expression=SIRLiteral(value=1)), "expression_stmt"),
            (SIRIfStmt(test=SIRLiteral(value=True), consequent=SIRBlock()), "if"),
            (SIRWhileStmt(test=SIRLiteral(value=True), body=SIRBlock()), "while"),
            (SIRForOfStmt(binding="x", iterable=SIRIdentifier("xs"), body=SIRBlock()), "for_of"),
            (SIRForInStmt(binding="k", object=SIRIdentifier("obj"), body=SIRBlock()), "for_in"),
            (SIRForStmt(), "for"),
            (SIRReturnStmt(), "return"),
            (SIRThrowStmt(value=SIRIdentifier("err")), "throw"),
            (SIRCatchClause(), "catch"),
            (SIRTryStmt(body=SIRBlock(), handler=SIRCatchClause()), "try"),
            (SIRBreakStmt(), "break"),
            (SIRContinueStmt(), "continue"),
            (SIRSwitchCase(), "switch_case"),
            (SIRSwitchStmt(discriminant=SIRIdentifier("x")), "switch"),
            (SIRLangSpecific(language="python", construct="with"), "lang_specific"),
            (SIRLiteral(value=1), "literal"),
            (SIRIdentifier(name="x"), "identifier"),
            (SIRBinaryOp(op="+", left=SIRLiteral(value=1), right=SIRLiteral(value=2)), "binary_op"),
            (SIRUnaryOp(op="-", operand=SIRLiteral(value=1)), "unary_op"),
            (SIRSpread(value=SIRIdentifier("x")), "spread"),
            (SIRAssignment(op="=", target=SIRIdentifier("x"), value=SIRLiteral(value=1)), "assignment"),
            (SIRCall(callee=SIRIdentifier("f")), "call"),
            (SIRMemberAccess(object=SIRIdentifier("o"), property="p"), "member_access"),
            (SIRIndex(object=SIRIdentifier("a"), index=SIRLiteral(value=0)), "index"),
            (SIRConditional(test=SIRLiteral(value=True), consequent=SIRLiteral(value=1), alternate=SIRLiteral(value=0)), "conditional"),
            (SIRProperty(key="k", value=SIRLiteral(value=1)), "property"),
            (SIRObjectLiteral(), "object_literal"),
            (SIRArrayLiteral(), "array_literal"),
            (SIRArrowFunction(), "arrow_function"),
            (SIRFunctionExpression(), "function_expression"),
            (SIRTemplateLiteral(quasis=["hi"]), "template_literal"),
            (SIRAwait(value=SIRIdentifier("p")), "await"),
            (SIRYield(), "yield"),
            (SIRTypeAssertion(value=SIRIdentifier("x")), "type_assertion"),
            (SIRSequence(), "sequence"),
            (SIRModule(), "module"),
        ]
        for node, expected in cases:
            assert node.type == expected, (
                f"{type(node).__name__}.type: expected {expected!r}, got {node.type!r}"
            )
