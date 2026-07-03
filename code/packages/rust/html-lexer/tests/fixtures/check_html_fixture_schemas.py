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
    check_browser_document_metadata(case_path, expected, errors)
    for field in ("document_lang", "document_dir", "body_id", "body_lang", "body_dir"):
        require_optional_nullable_string(case_path, expected, field, errors)
    require_optional_string_list(case_path, expected, "body_classes", errors)
    require_optional_string_list(case_path, expected, "document_event_handlers", errors)
    require_optional_string_list(case_path, expected, "body_event_handlers", errors)
    require_string(f"{case_path}.expected", expected, "body_text", errors)
    require_object_list(f"{case_path}.expected", expected, "metas", errors)
    require_object_list(f"{case_path}.expected", expected, "resources", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "scripts", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "stylesheets", errors)
    require_object_list(f"{case_path}.expected", expected, "anchors", errors)
    require_object_list(f"{case_path}.expected", expected, "headings", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "text_semantics", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "navigation_groups", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "section_landmarks", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "command_elements", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "popovers", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "aria_collections", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "aria_ranges", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "aria_live_regions", errors)
    require_object_list(f"{case_path}.expected", expected, "links", errors)
    require_object_list(f"{case_path}.expected", expected, "images", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "image_maps", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "media", errors)
    require_optional_object_list(
        f"{case_path}.expected", expected, "embedded_contexts", errors
    )
    require_optional_object_list(
        f"{case_path}.expected", expected, "interactive_elements", errors
    )
    require_optional_object_list(
        f"{case_path}.expected", expected, "component_hydration_targets", errors
    )
    require_optional_object_list(
        f"{case_path}.expected", expected, "data_attribute_descriptors", errors
    )
    require_optional_object_list(
        f"{case_path}.expected", expected, "global_state_descriptors", errors
    )
    require_optional_object_list(
        f"{case_path}.expected", expected, "structured_items", errors
    )
    require_optional_object_list(f"{case_path}.expected", expected, "templates", errors)
    require_object_list(f"{case_path}.expected", expected, "forms", errors)
    require_object_list(f"{case_path}.expected", expected, "tables", errors)
    require_optional_object_list(f"{case_path}.expected", expected, "table_cells", errors)
    check_browser_expected_lists(case_path, expected, errors)

    return errors


def check_browser_document_metadata(
    case_path: str,
    expected: dict[str, Any],
    errors: list[str],
) -> None:
    metadata = expected.get("metadata")
    if metadata is None:
        return
    metadata_path = f"{case_path}.expected.metadata"
    if not isinstance(metadata, dict):
        errors.append(f"{metadata_path} must be an object")
        return

    for field in (
        "charset",
        "viewport",
        "description",
        "application_name",
        "referrer_policy",
        "robots",
        "color_scheme",
        "canonical_url",
        "resolved_canonical_url",
        "manifest_url",
        "resolved_manifest_url",
    ):
        require_optional_nullable_string(metadata_path, metadata, field, errors)

    require_optional_object_list(metadata_path, metadata, "theme_colors", errors)
    require_optional_object_list(metadata_path, metadata, "viewport_directives", errors)
    for index, directive in enumerate(object_list_items(metadata, "viewport_directives")):
        directive_path = f"{metadata_path}.viewport_directives[{index}]"
        require_string(directive_path, directive, "name", errors)
        require_optional_nullable_string(directive_path, directive, "value", errors)
    require_optional_string_list(metadata_path, metadata, "robots_directives", errors)
    require_optional_object_list(metadata_path, metadata, "http_equiv_hints", errors)
    for index, hint in enumerate(object_list_items(metadata, "http_equiv_hints")):
        hint_path = f"{metadata_path}.http_equiv_hints[{index}]"
        require_string(hint_path, hint, "name", errors)
        require_string(hint_path, hint, "content", errors)
    require_optional_object_list(metadata_path, metadata, "resource_hints", errors)
    for index, hint in enumerate(object_list_items(metadata, "resource_hints")):
        hint_path = f"{metadata_path}.resource_hints[{index}]"
        require_string(hint_path, hint, "kind", errors)
        require_string(hint_path, hint, "url", errors)
        for field in (
            "resolved_url",
            "rel",
            "as_hint",
            "type_hint",
            "media",
            "integrity",
            "crossorigin",
            "nonce",
            "referrerpolicy",
            "fetchpriority",
            "blocking",
            "imagesrcset",
            "resolved_imagesrcset",
            "imagesizes",
        ):
            require_optional_nullable_string(hint_path, hint, field, errors)
        require_optional_string_list(hint_path, hint, "rel_tokens", errors)
        require_optional_string_list(hint_path, hint, "blocking_tokens", errors)

    for index, theme_color in enumerate(object_list_items(metadata, "theme_colors")):
        theme_color_path = f"{metadata_path}.theme_colors[{index}]"
        require_string(theme_color_path, theme_color, "color", errors)
        require_optional_nullable_string(theme_color_path, theme_color, "media", errors)

    refresh = metadata.get("refresh")
    if refresh is not None:
        refresh_path = f"{metadata_path}.refresh"
        if not isinstance(refresh, dict):
            errors.append(f"{refresh_path} must be an object")
        else:
            for field in ("delay", "url", "resolved_url"):
                require_optional_nullable_string(refresh_path, refresh, field, errors)


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
        for field in (
            "resolved_url",
            "rel",
            "as_hint",
            "type_hint",
            "media",
            "title",
            "sizes",
            "hreflang",
            "color",
            "width",
            "height",
            "integrity",
            "crossorigin",
            "nonce",
            "referrerpolicy",
            "fetchpriority",
            "csp",
            "blocking",
            "browsing_context_name",
            "loading",
            "allow",
            "srcdoc",
            "imagesrcset",
            "resolved_imagesrcset",
            "imagesizes",
            "track_kind",
            "srclang",
            "track_label",
        ):
            require_optional_nullable_string(resource_path, resource, field, errors)
        require_optional_string_list(resource_path, resource, "rel_tokens", errors)
        require_optional_string_list(resource_path, resource, "blocking_tokens", errors)
        require_optional_string_list(resource_path, resource, "sandbox", errors)
        require_optional_boolean(resource_path, resource, "allowfullscreen", errors)
        require_optional_boolean(resource_path, resource, "credentialless", errors)
        require_optional_boolean(resource_path, resource, "default_track", errors)
        require_boolean(resource_path, resource, "async_script", errors)
        require_boolean(resource_path, resource, "defer_script", errors)

    for index, script in enumerate(object_list_items(expected, "scripts")):
        script_path = f"{case_path}.expected.scripts[{index}]"
        require_string(script_path, script, "script_kind", errors)
        for field in (
            "src",
            "resolved_src",
            "type_hint",
            "integrity",
            "crossorigin",
            "nonce",
            "referrerpolicy",
            "fetchpriority",
            "blocking",
            "text",
        ):
            require_optional_nullable_string(script_path, script, field, errors)
        require_optional_string_list(script_path, script, "blocking_tokens", errors)
        for field in ("async_script", "defer_script", "nomodule"):
            require_optional_boolean(script_path, script, field, errors)

    for index, stylesheet in enumerate(object_list_items(expected, "stylesheets")):
        stylesheet_path = f"{case_path}.expected.stylesheets[{index}]"
        require_string(stylesheet_path, stylesheet, "source", errors)
        for field in (
            "href",
            "resolved_href",
            "rel",
            "type_hint",
            "media",
            "title",
            "integrity",
            "crossorigin",
            "nonce",
            "referrerpolicy",
            "fetchpriority",
            "blocking",
            "text",
        ):
            require_optional_nullable_string(stylesheet_path, stylesheet, field, errors)
        require_optional_string_list(stylesheet_path, stylesheet, "rel_tokens", errors)
        require_optional_string_list(stylesheet_path, stylesheet, "blocking_tokens", errors)
        for field in ("disabled", "alternate"):
            require_optional_boolean(stylesheet_path, stylesheet, field, errors)

    for index, anchor in enumerate(object_list_items(expected, "anchors")):
        anchor_path = f"{case_path}.expected.anchors[{index}]"
        for field in ("id", "name"):
            require_optional_nullable_string(anchor_path, anchor, field, errors)
        require_string(anchor_path, anchor, "text", errors)

    for index, heading in enumerate(object_list_items(expected, "headings")):
        heading_path = f"{case_path}.expected.headings[{index}]"
        require_integer(heading_path, heading, "level", errors)
        require_string(heading_path, heading, "text", errors)

    for index, semantic in enumerate(object_list_items(expected, "text_semantics")):
        semantic_path = f"{case_path}.expected.text_semantics[{index}]"
        for field in (
            "element",
            "role",
            "text",
        ):
            require_string(semantic_path, semantic, field, errors)
        for field in (
            "id",
            "title",
            "lang",
            "dir",
            "quote_cite",
            "resolved_quote_cite",
            "data_value",
            "datetime",
            "edit_cite",
            "resolved_edit_cite",
            "edit_datetime",
            "ruby_kind",
            "bidi_kind",
            "phrase_kind",
        ):
            require_optional_nullable_string(semantic_path, semantic, field, errors)

    for index, group in enumerate(object_list_items(expected, "navigation_groups")):
        group_path = f"{case_path}.expected.navigation_groups[{index}]"
        for field in (
            "element",
            "role",
            "text",
        ):
            require_string(group_path, group, field, errors)
        for field in (
            "id",
            "accessible_name",
            "aria_label",
            "landmark_kind",
            "list_kind",
            "list_start",
            "list_marker_type",
        ):
            require_optional_nullable_string(group_path, group, field, errors)
        require_optional_string_list(group_path, group, "aria_labelledby", errors)
        require_integer(group_path, group, "item_count", errors)
        require_optional_boolean(group_path, group, "list_reversed", errors)

    for index, section in enumerate(object_list_items(expected, "section_landmarks")):
        section_path = f"{case_path}.expected.section_landmarks[{index}]"
        for field in (
            "element",
            "role",
            "text",
        ):
            require_string(section_path, section, field, errors)
        for field in (
            "id",
            "authored_role",
            "accessible_name",
            "aria_label",
            "section_kind",
            "landmark_kind",
            "heading_text",
        ):
            require_optional_nullable_string(section_path, section, field, errors)
        require_optional_string_list(section_path, section, "aria_labelledby", errors)
        require_optional_integer(section_path, section, "heading_level", errors)

    for index, command in enumerate(object_list_items(expected, "command_elements")):
        command_path = f"{case_path}.expected.command_elements[{index}]"
        for field in (
            "element",
            "role",
            "command_kind",
            "text",
        ):
            require_string(command_path, command, field, errors)
        for field in (
            "id",
            "authored_role",
            "accessible_name",
            "accessible_description",
            "href",
            "resolved_href",
            "target",
            "effective_target",
            "control_type",
            "form_owner",
            "form_action",
            "resolved_form_action",
            "form_method",
            "form_target",
            "command",
            "command_for",
            "popover_target",
            "popover_target_action",
            "aria_expanded",
            "aria_haspopup",
            "aria_pressed",
            "aria_current",
            "aria_disabled",
            "tabindex",
        ):
            require_optional_nullable_string(command_path, command, field, errors)
        for field in (
            "aria_controls",
            "accesskey",
            "event_handlers",
        ):
            require_optional_string_list(command_path, command, field, errors)
        for field in (
            "form_novalidate",
            "focusable",
            "disabled",
        ):
            require_optional_boolean(command_path, command, field, errors)

    for index, popover in enumerate(object_list_items(expected, "popovers")):
        popover_path = f"{case_path}.expected.popovers[{index}]"
        for field in ("element", "role", "text", "popover"):
            require_string(popover_path, popover, field, errors)
        for field in (
            "id",
            "accessible_name",
            "accessible_description",
            "aria_label",
        ):
            require_optional_nullable_string(popover_path, popover, field, errors)
        for field in ("aria_labelledby", "aria_describedby"):
            require_optional_string_list(popover_path, popover, field, errors)
        require_optional_object_list(popover_path, popover, "invokers", errors)
        for invoker_index, invoker in enumerate(object_list_items(popover, "invokers")):
            invoker_path = f"{popover_path}.invokers[{invoker_index}]"
            for field in ("element", "text", "command_kind"):
                require_string(invoker_path, invoker, field, errors)
            for field in (
                "id",
                "accessible_name",
                "command",
                "command_for",
                "popover_target",
                "popover_target_action",
                "aria_expanded",
            ):
                require_optional_nullable_string(invoker_path, invoker, field, errors)
            require_optional_string_list(invoker_path, invoker, "aria_controls", errors)
            require_optional_boolean(invoker_path, invoker, "focusable", errors)

    for index, collection in enumerate(object_list_items(expected, "aria_collections")):
        collection_path = f"{case_path}.expected.aria_collections[{index}]"
        for field in (
            "element",
            "role",
            "text",
        ):
            require_string(collection_path, collection, field, errors)
        for field in (
            "id",
            "accessible_name",
            "accessible_description",
            "aria_label",
            "aria_orientation",
            "aria_multiselectable",
            "aria_activedescendant",
        ):
            require_optional_nullable_string(collection_path, collection, field, errors)
        for field in (
            "aria_labelledby",
            "aria_describedby",
            "aria_owns",
        ):
            require_optional_string_list(collection_path, collection, field, errors)
        for field in (
            "item_count",
            "selected_item_count",
            "checked_item_count",
            "current_item_count",
            "disabled_item_count",
        ):
            require_optional_integer(collection_path, collection, field, errors)
        require_optional_object_list(collection_path, collection, "items", errors)
        for item_index, item in enumerate(object_list_items(collection, "items")):
            item_path = f"{collection_path}.items[{item_index}]"
            for field in (
                "element",
                "role",
                "text",
            ):
                require_string(item_path, item, field, errors)
            for field in (
                "id",
                "accessible_name",
                "aria_selected",
                "aria_checked",
                "aria_current",
                "aria_disabled",
                "aria_expanded",
                "aria_level",
                "aria_posinset",
                "aria_setsize",
                "aria_rowindex",
                "aria_colindex",
            ):
                require_optional_nullable_string(item_path, item, field, errors)
            require_optional_string_list(item_path, item, "aria_controls", errors)

    for index, aria_range in enumerate(object_list_items(expected, "aria_ranges")):
        range_path = f"{case_path}.expected.aria_ranges[{index}]"
        for field in (
            "element",
            "role",
            "text",
        ):
            require_string(range_path, aria_range, field, errors)
        for field in (
            "id",
            "accessible_name",
            "accessible_description",
            "aria_label",
            "aria_valuenow",
            "aria_valuemin",
            "aria_valuemax",
            "aria_valuetext",
            "aria_orientation",
            "aria_disabled",
            "aria_readonly",
            "aria_required",
            "tabindex",
            "text_value",
        ):
            require_optional_nullable_string(range_path, aria_range, field, errors)
        for field in (
            "aria_labelledby",
            "aria_describedby",
        ):
            require_optional_string_list(range_path, aria_range, field, errors)

    for index, link in enumerate(object_list_items(expected, "links")):
        link_path = f"{case_path}.expected.links[{index}]"
        for field in (
            "element",
            "id",
            "href",
            "resolved_href",
            "name",
            "target",
            "effective_target",
            "rel",
            "title",
            "download",
            "hreflang",
            "type_hint",
            "referrerpolicy",
        ):
            require_optional_nullable_string(link_path, link, field, errors)
        require_optional_string_list(link_path, link, "rel_tokens", errors)
        for field in ("rel_external", "rel_nofollow", "rel_noopener", "rel_noreferrer"):
            require_optional_boolean(link_path, link, field, errors)
        require_optional_string_list(link_path, link, "ping", errors)
        require_optional_string_list(link_path, link, "resolved_ping", errors)
        require_optional_string_list(link_path, link, "attributionsrc", errors)
        require_optional_string_list(link_path, link, "resolved_attributionsrc", errors)
        require_string(link_path, link, "text", errors)

    for index, image in enumerate(object_list_items(expected, "images")):
        image_path = f"{case_path}.expected.images[{index}]"
        for field in (
            "src",
            "resolved_src",
            "alt",
            "width",
            "height",
            "srcset",
            "resolved_srcset",
            "sizes",
            "loading",
            "decoding",
            "fetchpriority",
            "crossorigin",
            "referrerpolicy",
            "usemap",
        ):
            require_optional_nullable_string(image_path, image, field, errors)
        require_optional_boolean(image_path, image, "ismap", errors)
        require_optional_object_list(image_path, image, "sources", errors)
        for source_index, source in enumerate(object_list_items(image, "sources")):
            source_path = f"{image_path}.sources[{source_index}]"
            for field in ("srcset", "resolved_srcset", "sizes", "media", "type_hint"):
                require_optional_nullable_string(source_path, source, field, errors)

    for index, image_map in enumerate(object_list_items(expected, "image_maps")):
        image_map_path = f"{case_path}.expected.image_maps[{index}]"
        for field in ("id", "name"):
            require_optional_nullable_string(image_map_path, image_map, field, errors)
        require_optional_object_list(image_map_path, image_map, "areas", errors)
        for area_index, area in enumerate(object_list_items(image_map, "areas")):
            area_path = f"{image_map_path}.areas[{area_index}]"
            for field in (
                "id",
                "shape",
                "coords",
                "href",
                "resolved_href",
                "alt",
                "target",
                "effective_target",
                "rel",
                "download",
                "hreflang",
                "referrerpolicy",
            ):
                require_optional_nullable_string(area_path, area, field, errors)
            require_optional_string_list(area_path, area, "rel_tokens", errors)
            require_optional_string_list(area_path, area, "ping", errors)
            require_optional_string_list(area_path, area, "resolved_ping", errors)
            require_optional_string_list(area_path, area, "attributionsrc", errors)
            require_optional_string_list(area_path, area, "resolved_attributionsrc", errors)
            for field in (
                "rel_external",
                "rel_nofollow",
                "rel_noopener",
                "rel_noreferrer",
            ):
                require_optional_boolean(area_path, area, field, errors)

    for index, media in enumerate(object_list_items(expected, "media")):
        media_path = f"{case_path}.expected.media[{index}]"
        require_string(media_path, media, "kind", errors)
        for field in (
            "src",
            "resolved_src",
            "poster",
            "resolved_poster",
            "width",
            "height",
            "preload",
            "crossorigin",
            "controlslist",
        ):
            require_optional_nullable_string(media_path, media, field, errors)
        require_optional_string_list(media_path, media, "controlslist_tokens", errors)
        for field in (
            "controls",
            "autoplay",
            "loop_media",
            "muted",
            "playsinline",
            "disableremoteplayback",
            "disablepictureinpicture",
        ):
            require_optional_boolean(media_path, media, field, errors)

    for index, context in enumerate(object_list_items(expected, "embedded_contexts")):
        context_path = f"{case_path}.expected.embedded_contexts[{index}]"
        require_string(context_path, context, "element", errors)
        for field in (
            "url",
            "resolved_url",
            "browsing_context_name",
            "title",
            "type_hint",
            "width",
            "height",
            "loading",
            "fetchpriority",
            "csp",
            "allow",
            "referrerpolicy",
            "srcdoc",
        ):
            require_optional_nullable_string(context_path, context, field, errors)
        require_optional_string_list(context_path, context, "sandbox", errors)
        for field in ("allowfullscreen", "credentialless"):
            require_optional_boolean(context_path, context, field, errors)
        require_optional_string(context_path, context, "fallback_text", errors)

    for index, element in enumerate(object_list_items(expected, "interactive_elements")):
        element_path = f"{case_path}.expected.interactive_elements[{index}]"
        require_string(element_path, element, "element", errors)
        for field in (
            "id",
            "role",
            "authored_role",
            "accessible_name",
            "accessible_description",
            "aria_label",
            "aria_activedescendant",
            "aria_current",
            "aria_expanded",
            "aria_haspopup",
            "aria_modal",
            "aria_pressed",
            "aria_selected",
            "aria_invalid",
            "aria_live",
            "aria_busy",
            "aria_disabled",
            "aria_required",
            "tabindex",
            "contenteditable",
            "editing_mode",
            "draggable",
            "draggable_state",
            "spellcheck",
            "translate",
            "popover",
            "popover_target",
            "popover_target_action",
            "command",
            "command_for",
        ):
            require_optional_nullable_string(element_path, element, field, errors)
        require_optional_string(element_path, element, "text", errors)
        for field in (
            "aria_labelledby",
            "aria_describedby",
            "aria_controls",
            "aria_owns",
            "accesskey",
            "event_handlers",
        ):
            require_optional_string_list(element_path, element, field, errors)
        for field in (
            "aria_hidden",
            "hidden",
            "inert",
            "open",
            "focusable",
            "disabled",
        ):
            require_optional_boolean(element_path, element, field, errors)

    for index, target in enumerate(
        object_list_items(expected, "component_hydration_targets")
    ):
        target_path = f"{case_path}.expected.component_hydration_targets[{index}]"
        require_string(target_path, target, "element", errors)
        for field in (
            "id",
            "custom_element_name",
            "custom_element_is",
            "slot",
            "slot_name",
            "exportparts",
            "canvas_fallback_text",
        ):
            require_optional_nullable_string(target_path, target, field, errors)
        require_optional_string(target_path, target, "text", errors)
        for field in ("classes", "part"):
            require_optional_string_list(target_path, target, field, errors)
        require_optional_boolean(target_path, target, "custom_element", errors)
        require_optional_object_list(target_path, target, "data_attributes", errors)
        for data_index, data_attribute in enumerate(
            object_list_items(target, "data_attributes")
        ):
            data_path = f"{target_path}.data_attributes[{data_index}]"
            require_string(data_path, data_attribute, "name", errors)
            require_optional_nullable_string(data_path, data_attribute, "value", errors)

    for index, descriptor in enumerate(
        object_list_items(expected, "data_attribute_descriptors")
    ):
        descriptor_path = f"{case_path}.expected.data_attribute_descriptors[{index}]"
        require_string(descriptor_path, descriptor, "element", errors)
        for field in ("id", "custom_element_name", "custom_element_is", "slot", "slot_name"):
            require_optional_nullable_string(descriptor_path, descriptor, field, errors)
        for field in ("classes", "part"):
            require_optional_string_list(descriptor_path, descriptor, field, errors)
        require_optional_boolean(descriptor_path, descriptor, "custom_element", errors)
        require_optional_string(descriptor_path, descriptor, "text", errors)
        require_optional_object_list(descriptor_path, descriptor, "data_attributes", errors)
        for data_index, data_attribute in enumerate(
            object_list_items(descriptor, "data_attributes")
        ):
            data_path = f"{descriptor_path}.data_attributes[{data_index}]"
            require_string(data_path, data_attribute, "name", errors)
            require_optional_nullable_string(data_path, data_attribute, "value", errors)

    for index, state in enumerate(object_list_items(expected, "global_state_descriptors")):
        state_path = f"{case_path}.expected.global_state_descriptors[{index}]"
        require_string(state_path, state, "element", errors)
        for field in (
            "id",
            "title",
            "lang",
            "dir",
            "tabindex",
            "contenteditable",
            "editing_mode",
            "draggable",
            "draggable_state",
            "spellcheck",
            "translate",
        ):
            require_optional_nullable_string(state_path, state, field, errors)
        for field in ("classes", "accesskey"):
            require_optional_string_list(state_path, state, field, errors)
        for field in ("hidden", "inert", "autofocus"):
            require_optional_boolean(state_path, state, field, errors)
        require_optional_string(state_path, state, "text", errors)

    for index, item in enumerate(object_list_items(expected, "structured_items")):
        item_path = f"{case_path}.expected.structured_items[{index}]"
        for field in ("id", "item_id", "resolved_item_id"):
            require_optional_nullable_string(item_path, item, field, errors)
        require_optional_string_list(item_path, item, "item_type", errors)
        require_optional_string_list(item_path, item, "item_ref", errors)
        require_optional_object_list(item_path, item, "properties", errors)
        for property_index, property_value in enumerate(
            object_list_items(item, "properties")
        ):
            property_path = f"{item_path}.properties[{property_index}]"
            require_string(property_path, property_value, "name", errors)
            for field in ("value", "value_url", "resolved_value_url"):
                require_optional_nullable_string(property_path, property_value, field, errors)

    for index, template in enumerate(object_list_items(expected, "templates")):
        template_path = f"{case_path}.expected.templates[{index}]"
        for field in ("id", "shadowrootmode"):
            require_optional_nullable_string(template_path, template, field, errors)
        for field in (
            "shadowrootdelegatesfocus",
            "shadowrootclonable",
            "shadowrootserializable",
        ):
            require_optional_boolean(template_path, template, field, errors)
        require_string(template_path, template, "content_text", errors)

    for index, form in enumerate(object_list_items(expected, "forms")):
        form_path = f"{case_path}.expected.forms[{index}]"
        require_optional_nullable_string(form_path, form, "id", errors)
        require_optional_nullable_string(form_path, form, "action", errors)
        require_optional_nullable_string(form_path, form, "resolved_action", errors)
        require_optional_nullable_string(form_path, form, "name", errors)
        require_string(form_path, form, "method", errors)
        require_optional_nullable_string(form_path, form, "enctype", errors)
        require_optional_nullable_string(form_path, form, "target", errors)
        require_optional_nullable_string(form_path, form, "effective_target", errors)
        require_optional_nullable_string(form_path, form, "accept_charset", errors)
        require_optional_string_list(form_path, form, "accept_charset_tokens", errors)
        require_optional_nullable_string(form_path, form, "autocomplete", errors)
        require_optional_string_list(form_path, form, "autocomplete_tokens", errors)
        require_optional_nullable_string(form_path, form, "rel", errors)
        require_optional_string_list(form_path, form, "rel_tokens", errors)
        for field in (
            "rel_external",
            "rel_nofollow",
            "rel_noopener",
            "rel_noreferrer",
        ):
            require_optional_boolean(form_path, form, field, errors)
        require_optional_boolean(form_path, form, "novalidate", errors)
        require_optional_object_list(form_path, form, "fieldsets", errors)
        for fieldset_index, fieldset in enumerate(object_list_items(form, "fieldsets")):
            fieldset_path = f"{form_path}.fieldsets[{fieldset_index}]"
            for field in ("id", "form_owner", "legend"):
                require_optional_nullable_string(fieldset_path, fieldset, field, errors)
            require_optional_boolean(fieldset_path, fieldset, "disabled", errors)
            require_optional_string_list(fieldset_path, fieldset, "control_ids", errors)
            require_optional_string_list(fieldset_path, fieldset, "control_names", errors)
        require_optional_object_list(form_path, form, "labels", errors)
        for label_index, label in enumerate(object_list_items(form, "labels")):
            label_path = f"{form_path}.labels[{label_index}]"
            require_optional_nullable_string(label_path, label, "id", errors)
            require_optional_nullable_string(label_path, label, "for_control", errors)
            require_string(label_path, label, "text", errors)
            for field in ("control_id", "control_name", "control_type"):
                require_optional_nullable_string(label_path, label, field, errors)
            require_string(label_path, label, "association", errors)
        require_optional_object_list(form_path, form, "datalists", errors)
        for datalist_index, datalist in enumerate(object_list_items(form, "datalists")):
            datalist_path = f"{form_path}.datalists[{datalist_index}]"
            require_optional_nullable_string(datalist_path, datalist, "id", errors)
            require_optional_string_list(datalist_path, datalist, "control_ids", errors)
            require_optional_string_list(datalist_path, datalist, "control_names", errors)
            require_optional_object_list(datalist_path, datalist, "options", errors)
            for option_index, option in enumerate(
                object_list_items(datalist, "options")
            ):
                option_path = f"{datalist_path}.options[{option_index}]"
                require_string(option_path, option, "value", errors)
                require_optional_nullable_string(option_path, option, "label", errors)
                require_string(option_path, option, "text", errors)
                require_optional_boolean(option_path, option, "disabled", errors)
        require_optional_object_list(form_path, form, "selects", errors)
        for select_index, select in enumerate(object_list_items(form, "selects")):
            select_path = f"{form_path}.selects[{select_index}]"
            for field in ("id", "name", "form_owner", "accessible_name", "size", "value"):
                require_optional_nullable_string(select_path, select, field, errors)
            require_optional_string_list(select_path, select, "labels", errors)
            for field in ("disabled", "required", "multiple"):
                require_optional_boolean(select_path, select, field, errors)
            require_optional_string_list(select_path, select, "selected_options", errors)
            require_optional_object_list(select_path, select, "options", errors)
            for option_index, option in enumerate(object_list_items(select, "options")):
                option_path = f"{select_path}.options[{option_index}]"
                require_string(option_path, option, "value", errors)
                require_optional_nullable_string(option_path, option, "label", errors)
                require_string(option_path, option, "text", errors)
                require_optional_boolean(option_path, option, "selected", errors)
                require_optional_boolean(option_path, option, "disabled", errors)
                require_optional_nullable_string(option_path, option, "group_label", errors)
            require_string(select_path, select, "text", errors)
        require_optional_object_list(form_path, form, "outputs", errors)
        for output_index, output in enumerate(object_list_items(form, "outputs")):
            output_path = f"{form_path}.outputs[{output_index}]"
            for field in (
                "id",
                "name",
                "form_owner",
                "accessible_name",
                "accessible_description",
                "value",
                "validation_barred_reason",
            ):
                require_optional_nullable_string(output_path, output, field, errors)
            require_optional_string_list(output_path, output, "labels", errors)
            require_optional_string_list(output_path, output, "for_tokens", errors)
            require_optional_string_list(output_path, output, "for_control_ids", errors)
            require_optional_string_list(output_path, output, "for_control_names", errors)
            require_optional_string_list(output_path, output, "for_control_types", errors)
            for field in ("disabled", "will_validate"):
                require_optional_boolean(output_path, output, field, errors)
            require_string(output_path, output, "text", errors)
        require_optional_object_list(form_path, form, "measurements", errors)
        for measurement_index, measurement in enumerate(
            object_list_items(form, "measurements")
        ):
            measurement_path = f"{form_path}.measurements[{measurement_index}]"
            require_optional_nullable_string(measurement_path, measurement, "id", errors)
            require_string(measurement_path, measurement, "measurement_type", errors)
            for field in (
                "accessible_name",
                "accessible_description",
                "value",
                "min",
                "max",
                "low",
                "high",
                "optimum",
            ):
                require_optional_nullable_string(measurement_path, measurement, field, errors)
            require_optional_string_list(measurement_path, measurement, "labels", errors)
            require_optional_boolean(measurement_path, measurement, "indeterminate", errors)
            require_string(measurement_path, measurement, "text", errors)
        require_optional_object_list(form_path, form, "successful_controls", errors)
        for successful_index, control in enumerate(
            object_list_items(form, "successful_controls")
        ):
            successful_path = f"{form_path}.successful_controls[{successful_index}]"
            require_optional_nullable_string(successful_path, control, "id", errors)
            require_string(successful_path, control, "control_type", errors)
            require_string(successful_path, control, "name", errors)
            require_optional_nullable_string(successful_path, control, "form_owner", errors)
            require_optional_string_list(
                successful_path, control, "submission_values", errors
            )
        require_optional_object_list(form_path, form, "validation_controls", errors)
        for validation_index, control in enumerate(
            object_list_items(form, "validation_controls")
        ):
            validation_path = f"{form_path}.validation_controls[{validation_index}]"
            require_optional_nullable_string(validation_path, control, "id", errors)
            require_string(validation_path, control, "control_type", errors)
            require_optional_nullable_string(validation_path, control, "name", errors)
            require_optional_nullable_string(validation_path, control, "form_owner", errors)
            require_optional_boolean(validation_path, control, "will_validate", errors)
            require_optional_boolean(validation_path, control, "required", errors)
            require_optional_string_list(
                validation_path, control, "validation_attributes", errors
            )
            require_optional_nullable_string(
                validation_path, control, "validation_barred_reason", errors
            )
        require_optional_object_list(form_path, form, "buttons", errors)
        for button_index, button in enumerate(object_list_items(form, "buttons")):
            button_path = f"{form_path}.buttons[{button_index}]"
            require_optional_nullable_string(button_path, button, "id", errors)
            require_string(button_path, button, "control_type", errors)
            for field in (
                "name",
                "form_owner",
                "accessible_name",
                "action",
                "resolved_action",
                "enctype",
                "target",
                "effective_target",
                "value",
                "src",
                "resolved_src",
                "alt",
                "width",
                "height",
            ):
                require_optional_nullable_string(button_path, button, field, errors)
            for field in ("disabled", "autofocus", "submitter", "novalidate"):
                require_optional_boolean(button_path, button, field, errors)
            require_string(button_path, button, "method", errors)
            require_string(button_path, button, "text", errors)
        require_optional_object_list(form_path, form, "text_entries", errors)
        for entry_index, entry in enumerate(object_list_items(form, "text_entries")):
            entry_path = f"{form_path}.text_entries[{entry_index}]"
            require_optional_nullable_string(entry_path, entry, "id", errors)
            require_string(entry_path, entry, "control_type", errors)
            for field in (
                "name",
                "form_owner",
                "accessible_name",
                "accessible_description",
                "placeholder",
                "value",
                "autocomplete",
                "autocapitalize",
                "enterkeyhint",
                "dirname",
                "spellcheck",
                "autocorrect",
                "inputmode",
                "pattern",
                "min",
                "max",
                "step",
                "minlength",
                "maxlength",
                "size",
                "rows",
                "cols",
                "wrap",
                "list",
                "validation_barred_reason",
            ):
                require_optional_nullable_string(entry_path, entry, field, errors)
            require_optional_string_list(entry_path, entry, "labels", errors)
            require_optional_string_list(entry_path, entry, "autocomplete_tokens", errors)
            require_optional_string_list(entry_path, entry, "datalist_options", errors)
            for field in ("disabled", "required", "readonly", "will_validate"):
                require_optional_boolean(entry_path, entry, field, errors)
            require_optional_string_list(
                entry_path, entry, "validation_attributes", errors
            )
            require_string(entry_path, entry, "text", errors)
        require_optional_object_list(form_path, form, "image_controls", errors)
        for image_index, image in enumerate(object_list_items(form, "image_controls")):
            image_path = f"{form_path}.image_controls[{image_index}]"
            for field in (
                "id",
                "name",
                "form_owner",
                "accessible_name",
                "src",
                "resolved_src",
                "alt",
                "width",
                "height",
                "action",
                "resolved_action",
                "enctype",
                "target",
                "effective_target",
                "value",
                "validation_barred_reason",
            ):
                require_optional_nullable_string(image_path, image, field, errors)
            require_optional_string_list(image_path, image, "labels", errors)
            require_optional_string_list(image_path, image, "coordinate_names", errors)
            for field in (
                "disabled",
                "autofocus",
                "submitter",
                "novalidate",
                "will_validate",
            ):
                require_optional_boolean(image_path, image, field, errors)
            require_string(image_path, image, "method", errors)
        require_object_list(form_path, form, "controls", errors)
        require_optional_object_list(form_path, form, "submitters", errors)
        for control_index, control in enumerate(object_list_items(form, "controls")):
            control_path = f"{form_path}.controls[{control_index}]"
            require_optional_nullable_string(control_path, control, "id", errors)
            require_string(control_path, control, "control_type", errors)
            require_optional_nullable_string(control_path, control, "name", errors)
            for field in (
                "form_owner",
                "accessible_name",
                "accessible_description",
                "placeholder",
                "autocomplete",
                "autocapitalize",
                "enterkeyhint",
                "dirname",
                "spellcheck",
                "autocorrect",
                "accept",
                "capture",
                "src",
                "resolved_src",
                "alt",
                "width",
                "height",
                "inputmode",
                "pattern",
                "min",
                "max",
                "step",
                "minlength",
                "maxlength",
                "size",
                "rows",
                "cols",
                "wrap",
                "list",
                "form_action",
                "resolved_form_action",
                "form_enctype",
                "form_method",
                "form_target",
            ):
                require_optional_nullable_string(control_path, control, field, errors)
            require_optional_string_list(control_path, control, "labels", errors)
            require_optional_string_list(control_path, control, "autocomplete_tokens", errors)
            require_optional_string_list(control_path, control, "accept_tokens", errors)
            require_optional_string_list(
                control_path, control, "validation_attributes", errors
            )
            require_optional_string_list(control_path, control, "datalist_options", errors)
            require_optional_string_list(control_path, control, "output_for", errors)
            require_optional_nullable_string(
                control_path, control, "validation_barred_reason", errors
            )
            require_optional_object_list(control_path, control, "option_items", errors)
            for option_index, option in enumerate(
                object_list_items(control, "option_items")
            ):
                option_path = f"{control_path}.option_items[{option_index}]"
                require_string(option_path, option, "value", errors)
                require_optional_nullable_string(option_path, option, "label", errors)
                require_string(option_path, option, "text", errors)
                require_optional_boolean(option_path, option, "selected", errors)
                require_optional_boolean(option_path, option, "disabled", errors)
                require_optional_nullable_string(option_path, option, "group_label", errors)
            require_optional_nullable_string(control_path, control, "value", errors)
            require_optional_boolean(control_path, control, "successful", errors)
            require_optional_string_list(control_path, control, "submission_values", errors)
            require_boolean(control_path, control, "disabled", errors)
            require_optional_boolean(control_path, control, "required", errors)
            require_optional_boolean(control_path, control, "readonly", errors)
            require_optional_boolean(control_path, control, "will_validate", errors)
            require_boolean(control_path, control, "checked", errors)
            require_optional_boolean(control_path, control, "multiple", errors)
            require_optional_boolean(control_path, control, "autofocus", errors)
            require_optional_boolean(control_path, control, "form_novalidate", errors)
            require_optional_string_list(control_path, control, "selected_options", errors)
            require_string(control_path, control, "text", errors)
            require_string_list(control_path, control, "options", errors)
        for submitter_index, submitter in enumerate(object_list_items(form, "submitters")):
            submitter_path = f"{form_path}.submitters[{submitter_index}]"
            require_optional_nullable_string(submitter_path, submitter, "id", errors)
            require_string(submitter_path, submitter, "control_type", errors)
            require_string(submitter_path, submitter, "method", errors)
            for field in (
                "name",
                "accessible_name",
                "action",
                "resolved_action",
                "enctype",
                "target",
                "effective_target",
                "value",
            ):
                require_optional_nullable_string(submitter_path, submitter, field, errors)
            require_optional_boolean(submitter_path, submitter, "novalidate", errors)

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

    for index, cell in enumerate(object_list_items(expected, "table_cells")):
        cell_path = f"{case_path}.expected.table_cells[{index}]"
        for field in ("table_index", "row_index", "column_index"):
            require_integer(cell_path, cell, field, errors)
        require_string(cell_path, cell, "element", errors)
        require_string(cell_path, cell, "text", errors)
        for field in (
            "table_id",
            "table_caption",
            "section_kind",
            "id",
            "accessible_name",
            "scope",
            "abbr",
            "rowspan",
            "colspan",
        ):
            require_optional_nullable_string(cell_path, cell, field, errors)
        require_optional_boolean(cell_path, cell, "header", errors)
        require_optional_string_list(cell_path, cell, "headers", errors)


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
        "authored_role",
        "name",
        "id",
        "title",
        "lang",
        "dir",
        "text",
        "href",
        "resolved_href",
        "target",
        "rel",
        "download",
        "hreflang",
        "src",
        "resolved_src",
        "alt",
        "resource_kind",
        "slot",
        "slot_name",
        "custom_element_name",
        "custom_element_is",
        "canvas_fallback_text",
        "width",
        "height",
        "type_hint",
        "image_map_name",
        "image_map_shape",
        "image_map_coords",
        "srcset",
        "resolved_srcset",
        "sizes",
        "track_kind",
        "srclang",
        "track_label",
        "media",
        "poster",
        "resolved_poster",
        "preload",
        "browsing_context_name",
        "loading",
        "allow",
        "referrerpolicy",
        "srcdoc",
        "control_type",
        "form_owner",
        "label_for",
        "accessible_name",
        "accessible_description",
        "aria_label",
        "aria_activedescendant",
        "aria_current",
        "aria_expanded",
        "aria_haspopup",
        "aria_modal",
        "aria_pressed",
        "aria_selected",
        "aria_invalid",
        "aria_live",
        "aria_busy",
        "aria_disabled",
        "aria_required",
        "tabindex",
        "contenteditable",
        "editing_mode",
        "draggable",
        "draggable_state",
        "spellcheck",
        "translate",
        "popover",
        "popover_target",
        "popover_target_action",
        "command",
        "command_for",
        "placeholder",
        "autocomplete",
        "autocapitalize",
        "enterkeyhint",
        "dirname",
        "accept",
        "capture",
        "inputmode",
        "pattern",
        "min",
        "max",
        "low",
        "high",
        "optimum",
        "step",
        "minlength",
        "maxlength",
        "size",
        "list",
        "form_action",
        "resolved_form_action",
        "form_enctype",
        "form_method",
        "form_target",
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
        "description_list_kind",
        "term_kind",
        "quote_cite",
        "resolved_quote_cite",
        "data_value",
        "datetime",
        "edit_cite",
        "resolved_edit_cite",
        "edit_datetime",
        "item_id",
        "resolved_item_id",
        "item_value",
        "item_value_url",
        "resolved_item_value_url",
        "ruby_kind",
        "bidi_kind",
        "break_kind",
        "grouping_kind",
        "disclosure_kind",
        "section_kind",
        "landmark_kind",
    ):
        require_optional_nullable_string(node_path, node, field, errors)
    require_optional_integer(node_path, node, "heading_level", errors)
    require_optional_string_list(node_path, node, "classes", errors)
    require_optional_string_list(node_path, node, "rel_tokens", errors)
    require_optional_string_list(node_path, node, "ping", errors)
    require_optional_string_list(node_path, node, "resolved_ping", errors)
    require_optional_string_list(node_path, node, "attributionsrc", errors)
    require_optional_string_list(node_path, node, "resolved_attributionsrc", errors)
    require_optional_string_list(node_path, node, "headers", errors)
    require_optional_string_list(node_path, node, "labels", errors)
    require_optional_string_list(node_path, node, "sandbox", errors)
    require_optional_string_list(node_path, node, "accesskey", errors)
    require_optional_string_list(node_path, node, "event_handlers", errors)
    require_optional_string_list(node_path, node, "aria_labelledby", errors)
    require_optional_string_list(node_path, node, "aria_describedby", errors)
    require_optional_string_list(node_path, node, "aria_controls", errors)
    require_optional_string_list(node_path, node, "aria_owns", errors)
    require_optional_string_list(node_path, node, "item_type", errors)
    require_optional_string_list(node_path, node, "item_ref", errors)
    require_optional_string_list(node_path, node, "itemprop", errors)
    for field in (
        "disabled",
        "required",
        "readonly",
        "checked",
        "selected",
        "multiple",
        "autofocus",
        "controls",
        "autoplay",
        "loop_media",
        "muted",
        "playsinline",
        "allowfullscreen",
        "credentialless",
        "default_track",
        "form_novalidate",
        "list_reversed",
        "aria_hidden",
        "hidden",
        "inert",
        "open",
        "focusable",
        "item_scope",
        "custom_element",
    ):
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
        "authored_role",
        "name",
        "id",
        "title",
        "lang",
        "dir",
        "text",
        "href",
        "resolved_href",
        "target",
        "rel",
        "download",
        "hreflang",
        "src",
        "resolved_src",
        "alt",
        "resource_kind",
        "slot",
        "slot_name",
        "custom_element_name",
        "custom_element_is",
        "canvas_fallback_text",
        "width",
        "height",
        "type_hint",
        "image_map_name",
        "image_map_shape",
        "image_map_coords",
        "srcset",
        "resolved_srcset",
        "sizes",
        "track_kind",
        "srclang",
        "track_label",
        "media",
        "poster",
        "resolved_poster",
        "preload",
        "browsing_context_name",
        "loading",
        "allow",
        "referrerpolicy",
        "srcdoc",
        "control_type",
        "form_owner",
        "label_for",
        "accessible_name",
        "accessible_description",
        "aria_label",
        "aria_activedescendant",
        "aria_current",
        "aria_expanded",
        "aria_haspopup",
        "aria_modal",
        "aria_pressed",
        "aria_selected",
        "aria_invalid",
        "aria_live",
        "aria_busy",
        "aria_disabled",
        "aria_required",
        "tabindex",
        "contenteditable",
        "editing_mode",
        "draggable",
        "draggable_state",
        "spellcheck",
        "translate",
        "popover",
        "popover_target",
        "popover_target_action",
        "command",
        "command_for",
        "placeholder",
        "autocomplete",
        "autocapitalize",
        "enterkeyhint",
        "dirname",
        "accept",
        "capture",
        "inputmode",
        "pattern",
        "min",
        "max",
        "low",
        "high",
        "optimum",
        "step",
        "minlength",
        "maxlength",
        "size",
        "list",
        "form_action",
        "resolved_form_action",
        "form_enctype",
        "form_method",
        "form_target",
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
        "description_list_kind",
        "term_kind",
        "quote_cite",
        "resolved_quote_cite",
        "data_value",
        "datetime",
        "edit_cite",
        "resolved_edit_cite",
        "edit_datetime",
        "item_id",
        "resolved_item_id",
        "item_value",
        "item_value_url",
        "resolved_item_value_url",
        "ruby_kind",
        "bidi_kind",
        "break_kind",
        "grouping_kind",
        "disclosure_kind",
        "section_kind",
        "landmark_kind",
    ):
        require_optional_nullable_string(node_path, node, field, errors)
    require_optional_integer(node_path, node, "heading_level", errors)
    require_optional_string_list(node_path, node, "classes", errors)
    require_optional_string_list(node_path, node, "rel_tokens", errors)
    require_optional_string_list(node_path, node, "ping", errors)
    require_optional_string_list(node_path, node, "resolved_ping", errors)
    require_optional_string_list(node_path, node, "attributionsrc", errors)
    require_optional_string_list(node_path, node, "resolved_attributionsrc", errors)
    require_optional_string_list(node_path, node, "headers", errors)
    require_optional_string_list(node_path, node, "labels", errors)
    require_optional_string_list(node_path, node, "sandbox", errors)
    require_optional_string_list(node_path, node, "accesskey", errors)
    require_optional_string_list(node_path, node, "event_handlers", errors)
    require_optional_string_list(node_path, node, "aria_labelledby", errors)
    require_optional_string_list(node_path, node, "aria_describedby", errors)
    require_optional_string_list(node_path, node, "aria_controls", errors)
    require_optional_string_list(node_path, node, "aria_owns", errors)
    require_optional_string_list(node_path, node, "item_type", errors)
    require_optional_string_list(node_path, node, "item_ref", errors)
    require_optional_string_list(node_path, node, "itemprop", errors)
    for field in (
        "disabled",
        "required",
        "readonly",
        "checked",
        "selected",
        "multiple",
        "autofocus",
        "controls",
        "autoplay",
        "loop_media",
        "muted",
        "playsinline",
        "allowfullscreen",
        "credentialless",
        "default_track",
        "form_novalidate",
        "list_reversed",
        "aria_hidden",
        "hidden",
        "inert",
        "open",
        "focusable",
        "item_scope",
        "custom_element",
    ):
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


def require_optional_object_list(
    path: str,
    data: dict[str, Any],
    field: str,
    errors: list[str],
) -> None:
    if field in data:
        require_object_list(path, data, field, errors)


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
