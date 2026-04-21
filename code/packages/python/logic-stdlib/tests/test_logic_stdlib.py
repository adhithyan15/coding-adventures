"""Tests for the first relational standard-library helpers."""

from logic_engine import (
    __version__ as logic_engine_version,
)
from logic_engine import (
    atom,
    conj,
    eq,
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
    lasto,
    listo,
    membero,
    permuteo,
    reverseo,
    selecto,
    subsequenceo,
    tailo,
)


class TestVersion:
    """Verify the package is importable and wired to the engine layer."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.4.0"
        engine_major, engine_minor, _engine_patch = logic_engine_version.split(".")
        assert (int(engine_major), int(engine_minor)) >= (0, 4)


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

    def test_listo_recognizes_the_empty_list(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), listo(logic_list([]))),
        ) == [atom("ok")]

    def test_listo_succeeds_for_a_concrete_proper_list(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), listo(logic_list(["tea", "cake", "jam"]))),
        ) == [atom("ok")]

    def test_listo_rejects_an_improper_dotted_pair(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), listo(logic_list(["tea"], tail="cake"))),
        ) == []

    def test_lasto_extracts_the_final_element(self) -> None:
        last = var("Last")

        assert solve_all(
            program(),
            last,
            lasto(logic_list(["tea", "cake", "jam"]), last),
        ) == [atom("jam")]

    def test_lasto_handles_single_element_lists(self) -> None:
        last = var("Last")

        assert solve_all(
            program(),
            last,
            lasto(logic_list(["tea"]), last),
        ) == [atom("tea")]

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

    def test_selecto_can_remove_a_known_element(self) -> None:
        remainder = var("Remainder")

        assert solve_all(
            program(),
            remainder,
            selecto("cake", logic_list(["tea", "cake", "jam"]), remainder),
        ) == [logic_list(["tea", "jam"])]

    def test_selecto_enumerates_all_element_and_remainder_pairs(self) -> None:
        item = var("Item")
        remainder = var("Remainder")

        assert solve_all(
            program(),
            (item, remainder),
            selecto(item, logic_list(["tea", "cake", "jam"]), remainder),
        ) == [
            (atom("tea"), logic_list(["cake", "jam"])),
            (atom("cake"), logic_list(["tea", "jam"])),
            (atom("jam"), logic_list(["tea", "cake"])),
        ]

    def test_permuteo_enumerates_every_permutation_of_a_small_list(self) -> None:
        order = var("Order")

        assert solve_all(
            program(),
            order,
            permuteo(logic_list(["tea", "cake", "jam"]), order),
        ) == [
            logic_list(["tea", "cake", "jam"]),
            logic_list(["tea", "jam", "cake"]),
            logic_list(["cake", "tea", "jam"]),
            logic_list(["cake", "jam", "tea"]),
            logic_list(["jam", "tea", "cake"]),
            logic_list(["jam", "cake", "tea"]),
        ]

    def test_permuteo_can_validate_a_specific_ordering(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(
                eq(marker, "ok"),
                permuteo(
                    logic_list(["tea", "jam"]),
                    logic_list(["jam", "tea"]),
                ),
            ),
        ) == [atom("ok")]

    def test_reverseo_reverses_a_concrete_list(self) -> None:
        reversed_items = var("ReversedItems")

        assert solve_all(
            program(),
            reversed_items,
            reverseo(logic_list(["tea", "cake", "jam"]), reversed_items),
        ) == [logic_list(["jam", "cake", "tea"])]

    def test_reverseo_can_validate_a_specific_reverse_ordering(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(
                eq(marker, "ok"),
                reverseo(
                    logic_list(["tea", "cake", "jam"]),
                    logic_list(["jam", "cake", "tea"]),
                ),
            ),
        ) == [atom("ok")]

    def test_subsequenceo_enumerates_all_subsequences_of_a_small_list(self) -> None:
        sequence = var("Sequence")

        assert solve_all(
            program(),
            sequence,
            subsequenceo(logic_list(["tea", "cake"]), sequence),
        ) == [
            logic_list(["tea", "cake"]),
            logic_list(["tea"]),
            logic_list(["cake"]),
            logic_list([]),
        ]

    def test_subsequenceo_can_validate_a_known_ordered_subsequence(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(
                eq(marker, "ok"),
                subsequenceo(
                    logic_list(["tea", "cake", "jam"]),
                    logic_list(["tea", "jam"]),
                ),
            ),
        ) == [atom("ok")]

    def test_subsequenceo_rejects_out_of_order_candidates(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(
                eq(marker, "ok"),
                subsequenceo(
                    logic_list(["tea", "cake", "jam"]),
                    logic_list(["jam", "tea"]),
                ),
            ),
        ) == []
