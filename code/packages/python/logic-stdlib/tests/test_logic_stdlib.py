"""Tests for the first relational standard-library helpers."""

from logic_engine import (
    __version__ as logic_engine_version,
)
from logic_engine import (
    atom,
    logic_list,
    program,
    solve_all,
    solve_n,
    var,
)

from logic_stdlib import (
    __version__,
    appendo,
    conso,
    emptyo,
    heado,
    membero,
    tailo,
)


class TestVersion:
    """Verify the package is importable and wired to the engine layer."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"
        assert logic_engine_version == "0.3.0"


class TestListRelations:
    """The first standard-library helpers should cover common list relations."""

    def test_emptyo_recognizes_the_empty_list(self) -> None:
        items = var("Items")

        assert solve_all(program(), items, emptyo(items)) == [logic_list([])]

    def test_conso_can_deconstruct_a_non_empty_list(self) -> None:
        head = var("Head")
        tail = var("Tail")

        assert solve_all(
            program(),
            (head, tail),
            conso(head, tail, logic_list(["tea", "cake"])),
        ) == [(atom("tea"), logic_list(["cake"]))]

    def test_heado_extracts_the_first_element(self) -> None:
        head = var("Head")

        assert solve_all(
            program(),
            head,
            heado(logic_list(["tea", "cake", "jam"]), head),
        ) == [atom("tea")]

    def test_tailo_extracts_the_remaining_list(self) -> None:
        tail = var("Tail")

        assert solve_all(
            program(),
            tail,
            tailo(logic_list(["tea", "cake", "jam"]), tail),
        ) == [logic_list(["cake", "jam"])]

    def test_membero_enumerates_members_of_a_concrete_list(self) -> None:
        item = var("Item")

        assert solve_all(
            program(),
            item,
            membero(item, logic_list(["tea", "cake", "jam"])),
        ) == [atom("tea"), atom("cake"), atom("jam")]

    def test_appendo_concatenates_two_concrete_lists(self) -> None:
        combined = var("Combined")

        assert solve_all(
            program(),
            combined,
            appendo(logic_list(["tea"]), logic_list(["cake", "jam"]), combined),
        ) == [logic_list(["tea", "cake", "jam"])]

    def test_appendo_can_split_a_concrete_list(self) -> None:
        prefix = var("Prefix")
        suffix = var("Suffix")

        assert solve_n(
            program(),
            4,
            (prefix, suffix),
            appendo(prefix, suffix, logic_list(["tea", "cake"])),
        ) == [
            (logic_list([]), logic_list(["tea", "cake"])),
            (logic_list(["tea"]), logic_list(["cake"])),
            (logic_list(["tea", "cake"]), logic_list([])),
        ]
