#!/usr/bin/env python3

"""Check checked-in HTML lexer/parser fixture JSON schemas."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


FIXTURE_DIR = Path(__file__).resolve().parent
RUST_DIR = FIXTURE_DIR.parents[2]
PARSER_FIXTURE_DIR = RUST_DIR / "html-parser" / "tests" / "fixtures"
PARSER_SOURCE_FIXTURE = "html5lib-tree-construction-smoke.dat"
BROWSER_READINESS_FORMAT = "venture-html-browser-readiness/v1"
BROWSER_CONTENT_TREE_FORMAT = "venture-html-browser-content-tree/v1"
BROWSER_RENDER_TREE_FORMAT = "venture-html-browser-render-tree/v1"
NUMERIC_REFERENCE_FIXTURE = "whatwg-numeric-references.json"
INPUT_STREAM_FIXTURE = "whatwg-input-stream.json"
CHUNK_BOUNDARY_FIXTURE = "whatwg-chunk-boundaries.json"


@dataclass(frozen=True)
class FixtureStats:
    fixture_count: int
    case_count: int


def main() -> int:
    parse_args()
    errors, stats = check_fixture_schemas()

    print("HTML fixture JSON schemas")
    print(f"fixture files: {stats.fixture_count}")
    print(f"cases: {stats.case_count}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check the JSON schemas used by checked-in HTML lexer/parser "
            "fixture corpora."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for generated-fixture stale-check manifests.",
    )
    return parser.parse_args()


def check_fixture_schemas() -> tuple[list[str], FixtureStats]:
    errors: list[str] = []
    fixture_count = 0
    case_count = 0

    for fixture_path in fixture_json_files():
        data = read_json_object(fixture_path, errors)
        if data is None or "cases" not in data:
            continue

        relative_path = relative_fixture(fixture_path)
        cases = data.get("cases")
        fixture_count += 1
        errors.extend(check_top_level_schema(relative_path, data))

        if not isinstance(cases, list):
            errors.append(f"{relative_path}: cases must be a list")
            continue

        case_count += len(cases)
        for index, case in enumerate(cases):
            if not isinstance(case, dict):
                errors.append(f"{relative_path}: cases[{index}] must be an object")
                continue
            if data.get("format") == BROWSER_READINESS_FORMAT:
                errors.extend(check_browser_readiness_case(relative_path, index, case))
            elif data.get("format") == BROWSER_CONTENT_TREE_FORMAT:
                errors.extend(check_browser_content_tree_case(relative_path, index, case))
            elif data.get("format") == BROWSER_RENDER_TREE_FORMAT:
                errors.extend(check_browser_render_tree_case(relative_path, index, case))
            elif fixture_path.parent == PARSER_FIXTURE_DIR:
                errors.extend(check_parser_audit_case(relative_path, index, case))
            else:
                errors.extend(check_lexer_case(relative_path, fixture_path.name, index, case))

    return errors, FixtureStats(fixture_count=fixture_count, case_count=case_count)


def fixture_json_files() -> list[Path]:
    fixture_dirs = (FIXTURE_DIR, PARSER_FIXTURE_DIR)
    return sorted(
        fixture_path
        for fixture_dir in fixture_dirs
        for fixture_path in fixture_dir.glob("*.json")
        if fixture_path.is_file()
    )


def read_json_object(fixture_path: Path, errors: list[str]) -> dict[str, Any] | None:
    relative_path = relative_fixture(fixture_path)
    try:
        data = json.loads(fixture_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        errors.append(f"{relative_path}: invalid JSON: {exc}")
        return None

    if not isinstance(data, dict):
        errors.append(f"{relative_path}: fixture must be a JSON object")
        return None
    return data


def check_top_level_schema(relative_path: str, data: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    require_non_empty_string(relative_path, data, "format", errors)
    require_non_empty_string(relative_path, data, "description", errors)

    if data.get("format") in (
        BROWSER_READINESS_FORMAT,
        BROWSER_CONTENT_TREE_FORMAT,
        BROWSER_RENDER_TREE_FORMAT,
    ):
        require_non_empty_string(relative_path, data, "suite", errors)
    elif relative_path.startswith("html-parser/"):
        if data.get("source_fixture") != PARSER_SOURCE_FIXTURE:
            errors.append(
                f"{relative_path}: source_fixture must be {PARSER_SOURCE_FIXTURE!r}"
            )
        if not isinstance(data.get("case_count"), int):
            errors.append(f"{relative_path}: case_count must be an integer")
        if not isinstance(data.get("counts_by_axis"), dict):
            errors.append(f"{relative_path}: counts_by_axis must be an object")
    elif data.get("format") == "venture-html-lexer-fixtures/v1":
        require_non_empty_string(relative_path, data, "suite", errors)

    return errors


def check_lexer_case(
    relative_path: str,
    fixture_name: str,
    index: int,
    case: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    case_path = f"{relative_path}: cases[{index}]"

    if fixture_name == NUMERIC_REFERENCE_FIXTURE:
        require_integer(case_path, case, "value", errors)
        require_non_empty_string(case_path, case, "characters", errors)
        require_string(case_path, case, "decimal", errors)
        require_string(case_path, case, "decimal_missing_semicolon", errors)
        require_string(case_path, case, "hex", errors)
        require_string(case_path, case, "hex_missing_semicolon", errors)
        require_int_list(case_path, case, "codepoints", errors)
        require_optional_string_list(case_path, case, "diagnostics", errors)
        return errors

    require_string(case_path, case, "input", errors)
    require_optional_non_empty_string(case_path, case, "description", errors)

    if fixture_name == INPUT_STREAM_FIXTURE:
        require_string(case_path, case, "normalized", errors)
    elif fixture_name == CHUNK_BOUNDARY_FIXTURE:
        require_int_list(case_path, case, "split_points", errors)
        check_split_points(case_path, case, errors)
    else:
        require_string_list(case_path, case, "tokens", errors)
        require_optional_string_list(case_path, case, "diagnostics", errors)

    for field in (
        "initial_state",
        "last_start_tag",
        "current_end_tag",
        "temporary_buffer",
        "return_state",
    ):
        require_optional_non_empty_string(case_path, case, field, errors)
    require_optional_string(case_path, case, "current_comment", errors)
    for field in ("start_tag", "current_doctype"):
        if field in case and not isinstance(case[field], dict):
            errors.append(f"{case_path}.{field} must be an object")

    return errors


def check_parser_audit_case(
    relative_path: str,
    index: int,
    case: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    case_path = f"{relative_path}: cases[{index}]"

    for field in ("id", "axis", "reason", "source"):
        require_non_empty_string(case_path, case, field, errors)
    for field in ("context", "scripting"):
        require_optional_non_empty_string(case_path, case, field, errors)

    return errors


def check_browser_readiness_case(
    relative_path: str,
    index: int,
    case: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    case_path = f"{relative_path}: cases[{index}]"

    require_non_empty_string(case_path, case, "id", errors)
    require_optional_non_empty_string(case_path, case, "description", errors)
    require_string(case_path, case, "input", errors)

    expected = case.get("expected")
    if not isinstance(expected, dict):
        errors.append(f"{case_path}.expected must be an object")
        return errors

    require_optional_nullable_string(case_path, expected, "title", errors)
    require_optional_nullable_string(case_path, expected, "base_href", errors)
    require_optional_nullable_string(case_path, expected, "base_target", errors)
    for field in ("document_lang", "document_dir", "body_id", "body_lang", "body_dir"):
        require_optional_nullable_string(case_path, expected, field, errors)
    require_optional_string_list(case_path, expected, "body_classes", errors)
    require_string(f"{case_path}.expected", expected, "body_text", errors)
    require_object_list(f"{case_path}.expected", expected, "metas", errors)
    require_object_list(f"{case_path}.expected", expected, "resources", errors)
    require_object_list(f"{case_path}.expected", expected, "anchors", errors)
    require_object_list(f"{case_path}.expected", expected, "headings", errors)
    require_object_list(f"{case_path}.expected", expected, "links", errors)
    require_object_list(f"{case_path}.expected", expected, "images", errors)
    require_object_list(f"{case_path}.expected", expected, "forms", errors)
    require_object_list(f"{case_path}.expected", expected, "tables", errors)
    check_browser_expected_lists(case_path, expected, errors)

    return errors


def check_browser_expected_lists(
    case_path: str,
    expected: dict[str, Any],
    errors: list[str],
) -> None:
    for index, meta in enumerate(object_list_items(expected, "metas")):
        meta_path = f"{case_path}.expected.metas[{index}]"
        for field in ("name", "http_equiv", "property", "charset", "content"):
            require_optional_nullable_string(meta_path, meta, field, errors)

    for index, resource in enumerate(object_list_items(expected, "resources")):
        resource_path = f"{case_path}.expected.resources[{index}]"
        require_string(resource_path, resource, "kind", errors)
        require_string(resource_path, resource, "url", errors)
        for field in ("resolved_url", "rel", "type_hint", "media", "title", "width", "height"):
            require_optional_nullable_string(resource_path, resource, field, errors)
        require_boolean(resource_path, resource, "async_script", errors)
        require_boolean(resource_path, resource, "defer_script", errors)

    for index, anchor in enumerate(object_list_items(expected, "anchors")):
        anchor_path = f"{case_path}.expected.anchors[{index}]"
        for field in ("id", "name"):
            require_optional_nullable_string(anchor_path, anchor, field, errors)
        require_string(anchor_path, anchor, "text", errors)

    for index, heading in enumerate(object_list_items(expected, "headings")):
        heading_path = f"{case_path}.expected.headings[{index}]"
        require_integer(heading_path, heading, "level", errors)
        require_string(heading_path, heading, "text", errors)

    for index, link in enumerate(object_list_items(expected, "links")):
        link_path = f"{case_path}.expected.links[{index}]"
        for field in ("href", "resolved_href", "name", "target", "rel", "title"):
            require_optional_nullable_string(link_path, link, field, errors)
        require_string(link_path, link, "text", errors)

    for index, image in enumerate(object_list_items(expected, "images")):
        image_path = f"{case_path}.expected.images[{index}]"
        for field in ("src", "resolved_src", "alt", "width", "height"):
            require_optional_nullable_string(image_path, image, field, errors)

    for index, form in enumerate(object_list_items(expected, "forms")):
        form_path = f"{case_path}.expected.forms[{index}]"
        require_optional_nullable_string(form_path, form, "action", errors)
        require_optional_nullable_string(form_path, form, "resolved_action", errors)
        require_string(form_path, form, "method", errors)
        require_optional_nullable_string(form_path, form, "enctype", errors)
        require_optional_nullable_string(form_path, form, "target", errors)
        require_object_list(form_path, form, "controls", errors)
        for control_index, control in enumerate(object_list_items(form, "controls")):
            control_path = f"{form_path}.controls[{control_index}]"
            require_string(control_path, control, "control_type", errors)
            require_optional_nullable_string(control_path, control, "name", errors)
            require_optional_nullable_string(control_path, control, "value", errors)
            require_boolean(control_path, control, "disabled", errors)
            require_boolean(control_path, control, "checked", errors)
            require_string(control_path, control, "text", errors)
            require_string_list(control_path, control, "options", errors)

    for index, table in enumerate(object_list_items(expected, "tables")):
        table_path = f"{case_path}.expected.tables[{index}]"
        require_optional_nullable_string(table_path, table, "caption", errors)
        for field in (
            "row_count",
            "column_count",
            "column_hint_count",
            "cell_count",
            "header_cell_count",
        ):
            require_integer(table_path, table, field, errors)


def check_browser_content_tree_case(
    relative_path: str,
    index: int,
    case: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    case_path = f"{relative_path}: cases[{index}]"

    require_non_empty_string(case_path, case, "id", errors)
    require_optional_non_empty_string(case_path, case, "description", errors)
    require_string(case_path, case, "input", errors)

    expected = case.get("expected")
    if not isinstance(expected, dict):
        errors.append(f"{case_path}.expected must be an object")
        return errors
    require_object_list(f"{case_path}.expected", expected, "children", errors)
    for node_index, node in enumerate(object_list_items(expected, "children")):
        check_browser_content_node(
            f"{case_path}.expected.children[{node_index}]",
            node,
            errors,
        )

    return errors


def check_browser_content_node(
    node_path: str,
    node: dict[str, Any],
    errors: list[str],
) -> None:
    require_string(node_path, node, "role", errors)
    for field in (
        "name",
        "id",
        "title",
        "lang",
        "dir",
        "text",
        "href",
        "resolved_href",
        "src",
        "resolved_src",
        "alt",
        "resource_kind",
        "width",
        "height",
        "type_hint",
        "media",
        "control_type",
        "value",
        "table_section_kind",
        "colspan",
        "rowspan",
        "span",
        "scope",
        "abbr",
        "text_flow",
        "list_kind",
        "list_start",
        "list_marker_type",
        "list_item_value",
        "quote_cite",
        "resolved_quote_cite",
        "break_kind",
        "section_kind",
        "landmark_kind",
    ):
        require_optional_nullable_string(node_path, node, field, errors)
    require_optional_integer(node_path, node, "heading_level", errors)
    require_optional_string_list(node_path, node, "classes", errors)
    require_optional_string_list(node_path, node, "headers", errors)
    for field in ("disabled", "checked", "selected", "list_reversed"):
        require_optional_boolean(node_path, node, field, errors)
    require_optional_string_list(node_path, node, "options", errors)
    require_object_list(node_path, node, "children", errors)
    for child_index, child in enumerate(object_list_items(node, "children")):
        check_browser_content_node(
            f"{node_path}.children[{child_index}]",
            child,
            errors,
        )


def check_browser_render_tree_case(
    relative_path: str,
    index: int,
    case: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    case_path = f"{relative_path}: cases[{index}]"

    require_non_empty_string(case_path, case, "id", errors)
    require_optional_non_empty_string(case_path, case, "description", errors)
    require_string(case_path, case, "input", errors)

    expected = case.get("expected")
    if not isinstance(expected, dict):
        errors.append(f"{case_path}.expected must be an object")
        return errors
    require_object_list(f"{case_path}.expected", expected, "children", errors)
    for node_index, node in enumerate(object_list_items(expected, "children")):
        check_browser_render_node(
            f"{case_path}.expected.children[{node_index}]",
            node,
            errors,
        )

    return errors


def check_browser_render_node(
    node_path: str,
    node: dict[str, Any],
    errors: list[str],
) -> None:
    require_string(node_path, node, "display", errors)
    require_string(node_path, node, "role", errors)
    for field in (
        "name",
        "id",
        "title",
        "lang",
        "dir",
        "text",
        "href",
        "resolved_href",
        "src",
        "resolved_src",
        "alt",
        "resource_kind",
        "width",
        "height",
        "type_hint",
        "media",
        "control_type",
        "value",
        "table_section_kind",
        "colspan",
        "rowspan",
        "span",
        "scope",
        "abbr",
        "text_flow",
        "list_kind",
        "list_start",
        "list_marker_type",
        "list_item_value",
        "quote_cite",
        "resolved_quote_cite",
        "break_kind",
        "section_kind",
        "landmark_kind",
    ):
        require_optional_nullable_string(node_path, node, field, errors)
    require_optional_integer(node_path, node, "heading_level", errors)
    require_optional_string_list(node_path, node, "classes", errors)
    require_optional_string_list(node_path, node, "headers", errors)
    for field in ("disabled", "checked", "selected", "list_reversed"):
        require_optional_boolean(node_path, node, field, errors)
    require_optional_string_list(node_path, node, "options", errors)
    require_object_list(node_path, node, "children", errors)
    for child_index, child in enumerate(object_list_items(node, "children")):
        check_browser_render_node(
            f"{node_path}.children[{child_index}]",
            child,
            errors,
        )


def check_split_points(
    case_path: str,
    case: dict[str, Any],
    errors: list[str],
) -> None:
    input_value = case.get("input")
    split_points = case.get("split_points")
    if not isinstance(input_value, str) or not isinstance(split_points, list):
        return

    input_length = len(input_value)
    invalid_points = [
        point
        for point in split_points
        if not isinstance(point, int) or point < 0 or point > input_length
    ]
    if invalid_points:
        errors.append(
            f"{case_path}.split_points contains positions outside input length "
            f"{input_length}: {invalid_points!r}"
        )


def require_non_empty_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    value = data.get(field)
    if not isinstance(value, str) or not value:
        errors.append(f"{path}.{field} must be a non-empty string")


def require_optional_non_empty_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_non_empty_string(path, data, field, errors)


def require_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if not isinstance(data.get(field), str):
        errors.append(f"{path}.{field} must be a string")


def require_optional_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_string(path, data, field, errors)


def require_optional_nullable_string(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data and data[field] is not None and not isinstance(data[field], str):
        errors.append(f"{path}.{field} must be a string or null")


def require_integer(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if not isinstance(data.get(field), int):
        errors.append(f"{path}.{field} must be an integer")


def require_optional_integer(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_integer(path, data, field, errors)


def require_boolean(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if not isinstance(data.get(field), bool):
        errors.append(f"{path}.{field} must be a boolean")


def require_optional_boolean(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_boolean(path, data, field, errors)


def require_string_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    value = data.get(field)
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        errors.append(f"{path}.{field} must be a list of strings")


def require_optional_string_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_string_list(path, data, field, errors)


def require_object_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    value = data.get(field)
    if not isinstance(value, list) or not all(isinstance(item, dict) for item in value):
        errors.append(f"{path}.{field} must be a list of objects")


def object_list_items(data: dict[str, Any], field: str) -> list[dict[str, Any]]:
    value = data.get(field)
    if not isinstance(value, list) or not all(isinstance(item, dict) for item in value):
        return []
    return value


def require_optional_string_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_string_list(path, data, field, errors)


def require_int_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    value = data.get(field)
    if not isinstance(value, list) or not all(isinstance(item, int) for item in value):
        errors.append(f"{path}.{field} must be a list of integers")


def relative_fixture(path: Path) -> str:
    try:
        return str(path.relative_to(RUST_DIR))
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
