"""Tests for parsed-source loading and explicit initialization execution."""

from __future__ import annotations

import pytest
from logic_builtins import assertzo, dynamico
from logic_engine import (
    GoalExpr,
    RelationCall,
    atom,
    reify,
    relation,
    visible_clauses_for,
)

from prolog_loader import (
    PrologInitializationError,
    __version__,
    load_iso_prolog_source,
    load_swi_prolog_source,
    run_initialization_goals,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestPrologLoader:
    """Loading should keep parsing separate from explicit initialization."""

    def test_load_iso_source_collects_initialization_metadata(self) -> None:
        loaded = load_iso_prolog_source(
            """
            :- dynamic(parent/2).
            :- initialization(main(Result)).
            parent(homer, bart).
            main(done).
            """,
        )

        assert len(loaded.initialization_directives) == 1
        assert str(loaded.initialization_terms[0]) == "main(Result)"
        assert loaded.program.dynamic_relations == frozenset(
            {relation("parent", 2).key()},
        )
        assert loaded.predicate_registry.get("parent", 2) is not None

    def test_load_swi_source_collects_initialization_metadata(self) -> None:
        loaded = load_swi_prolog_source(
            """
            :- initialization(main).
            main.
            """,
        )

        assert len(loaded.initialization_goals) == 1
        assert str(loaded.initialization_terms[0]) == "main"

    def test_run_initialization_goals_executes_clause_backed_startup_goals(
        self,
    ) -> None:
        loaded = load_iso_prolog_source(
            """
            :- initialization(main(Result)).
            main(done).
            """,
        )

        state = run_initialization_goals(loaded)
        result_var = loaded.initialization_directives[0].variables["Result"]

        assert reify(result_var, state.substitution) == atom("done")

    def test_run_initialization_goals_accepts_goal_adapters_for_builtins(
        self,
    ) -> None:
        loaded = load_iso_prolog_source(
            """
            :- initialization(dynamico(memo, 1)).
            :- initialization(assertzo(memo(ok))).
            """,
        )

        def adapt(goal: GoalExpr) -> object:
            if isinstance(goal, RelationCall):
                if goal.relation == relation("dynamico", 2):
                    return dynamico(*goal.args)
                if goal.relation == relation("assertzo", 1):
                    return assertzo(goal.args[0])
            return goal

        state = run_initialization_goals(loaded, goal_adapter=adapt)
        memo = relation("memo", 1)
        visible = visible_clauses_for(loaded.program, memo, state)

        assert len(visible) == 1
        assert visible[0].head == memo("ok")

    def test_run_initialization_goals_raises_for_failed_startup_goals(self) -> None:
        loaded = load_iso_prolog_source(":- initialization(missing_goal).\n")

        with pytest.raises(
            PrologInitializationError,
            match=r"initialization directive 1 failed: missing_goal",
        ):
            run_initialization_goals(loaded)
