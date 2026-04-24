"""Tests for parsed-source loading and explicit initialization execution."""

from __future__ import annotations

import pytest
from logic_engine import (
    ConjExpr,
    DisjExpr,
    FreshExpr,
    LogicVar,
    RelationCall,
    State,
    atom,
    conj,
    disj,
    eq,
    fresh,
    reify,
    relation,
    solve_all,
    term,
    visible_clauses_for,
)

from prolog_loader import (
    LoadedPrologProject,
    PrologInitializationError,
    __version__,
    adapt_prolog_goal,
    link_loaded_prolog_sources,
    load_iso_prolog_source,
    load_swi_prolog_project,
    load_swi_prolog_source,
    run_initialization_goals,
    run_prolog_initialization_goals,
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

    def test_load_swi_source_collects_module_metadata(self) -> None:
        loaded = load_swi_prolog_source(
            """
            :- module(family, [parent/2, ancestor/2, op(500, yfx, ++)]).
            :- use_module(graph, [edge/2]).
            parent(homer, bart).
            """,
        )

        assert loaded.module_spec is not None
        assert loaded.module_spec.name.name == "family"
        assert [str(export) for export in loaded.module_spec.exports] == [
            "parent/2",
            "ancestor/2",
        ]
        assert str(loaded.module_spec.exported_operators[0].symbol) == "++"
        assert loaded.module_imports[0].module_name.name == "graph"
        assert [str(imported) for imported in loaded.module_imports[0].imports] == [
            "edge/2",
        ]

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

    def test_run_prolog_initialization_goals_executes_builtin_runtime_goals(
        self,
    ) -> None:
        loaded = load_iso_prolog_source(
            """
            :- initialization(dynamic(memo/1)).
            :- initialization(assertz(memo(ok))).
            :- initialization(call(memo(ok))).
            :- initialization(once(memo(ok))).
            :- initialization(not(memo(missing))).
            :- initialization(current_predicate(memo/1)).
            :- initialization(predicate_property(memo/1, dynamic)).
            """,
        )

        state = run_prolog_initialization_goals(loaded)
        memo = relation("memo", 1)
        visible = visible_clauses_for(loaded.program, memo, state)

        assert len(visible) == 1
        assert visible[0].head == memo("ok")

    def test_run_initialization_goals_accepts_shared_prolog_goal_adapter(
        self,
    ) -> None:
        loaded = load_iso_prolog_source(
            """
            :- initialization(dynamic(memo/1)).
            :- initialization(assertz(memo(ok))).
            """,
        )

        state = run_initialization_goals(loaded, goal_adapter=adapt_prolog_goal)
        memo = relation("memo", 1)
        visible = visible_clauses_for(loaded.program, memo, state)

        assert len(visible) == 1
        assert visible[0].head == memo("ok")

    def test_run_initialization_goals_still_accepts_custom_goal_adapters(
        self,
    ) -> None:
        loaded = load_iso_prolog_source(":- initialization(custom_startup).\n")

        def adapt(goal: object) -> object:
            if isinstance(goal, RelationCall) and goal.relation == relation(
                "custom_startup",
                0,
            ):
                return eq(atom("ok"), atom("ok"))
            return goal

        state = run_initialization_goals(loaded, goal_adapter=adapt)

        assert state == State()

    def test_run_initialization_goals_raises_for_failed_startup_goals(self) -> None:
        loaded = load_iso_prolog_source(":- initialization(missing_goal).\n")

        with pytest.raises(
            PrologInitializationError,
            match=r"initialization directive 1 failed: missing_goal",
        ):
            run_initialization_goals(loaded)

    def test_run_prolog_initialization_goals_supports_phrase_with_dcg_rules(
        self,
    ) -> None:
        loaded = load_iso_prolog_source(
            """
            digits --> [a], [b].
            :- initialization(phrase(digits, [a, b], Rest)).
            """,
        )

        state = run_prolog_initialization_goals(loaded)
        rest_var = loaded.initialization_directives[0].variables["Rest"]

        assert reify(rest_var, state.substitution) == atom("[]")

    def test_link_loaded_prolog_sources_resolves_module_imports(self) -> None:
        family = load_swi_prolog_source(
            """
            :- module(family, [parent/2, ancestor/2]).
            parent(homer, bart).
            parent(bart, lisa).
            ancestor(X, Y) :- parent(X, Y).
            ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
            """,
        )
        app = load_swi_prolog_source(
            """
            :- module(app, [run/1]).
            :- use_module(family, [ancestor/2]).
            run(Who) :- ancestor(homer, Who).
            ?- run(Who).
            """,
        )

        project = link_loaded_prolog_sources(family, app)
        query = project.queries[0]

        assert isinstance(project, LoadedPrologProject)
        assert [module.name.name for module in project.modules] == ["family", "app"]
        assert solve_all(project.program, query.variables["Who"], query.goal) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_load_swi_prolog_project_keeps_local_definitions_over_imports(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(family, [message/1]).
            message(imported).
            """,
            """
            :- module(app, [message/1]).
            :- use_module(family, [message/1]).
            message(local).
            ?- message(Value).
            """,
        )

        query = project.queries[0]

        assert solve_all(project.program, query.variables["Value"], query.goal) == [
            atom("local"),
        ]

    def test_run_project_initialization_goals_resolves_imported_module_calls(
        self,
    ) -> None:
        project = load_swi_prolog_project(
            """
            :- module(family, [main/1]).
            main(done).
            """,
            """
            :- module(app, []).
            :- use_module(family, [main/1]).
            :- initialization(main(Result)).
            """,
        )

        state = run_prolog_initialization_goals(project)
        result_var = project.initialization_directives[0].variables["Result"]

        assert reify(result_var, state.substitution) == atom("done")

    def test_linked_queries_support_explicit_module_qualification(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(family, [parent/2, ancestor/2]).
            parent(homer, bart).
            parent(bart, lisa).
            ancestor(X, Y) :- parent(X, Y).
            ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
            """,
            """
            :- module(app, []).
            ?- family:ancestor(homer, Who).
            """,
        )

        query = project.queries[0]

        assert solve_all(project.program, query.variables["Who"], query.goal) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_module_qualification_uses_target_module_imports(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(edges, [edge/2]).
            edge(homer, bart).
            edge(bart, lisa).
            """,
            """
            :- module(family, [ancestor/2]).
            :- use_module(edges, [edge/2]).
            ancestor(X, Y) :- edge(X, Y).
            ancestor(X, Y) :- edge(X, Z), ancestor(Z, Y).
            """,
            """
            :- module(app, []).
            ?- family:ancestor(homer, Who).
            """,
        )

        query = project.queries[0]

        assert solve_all(project.program, query.variables["Who"], query.goal) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_module_qualification_rewrites_meta_call_arguments(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(family, [ancestor/2]).
            ancestor(homer, bart).
            """,
            """
            :- module(app, []).
            ?- call(family:ancestor(homer, Who)).
            """,
        )

        query = project.queries[0]

        assert solve_all(
            project.program,
            query.variables["Who"],
            adapt_prolog_goal(query.goal),
        ) == [
            atom("bart"),
        ]

    def test_module_qualified_initialization_goals_execute(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(family, [main/1]).
            main(done).
            """,
            """
            :- module(app, []).
            :- initialization(family:main(Result)).
            """,
        )

        state = run_prolog_initialization_goals(project)
        result_var = project.initialization_directives[0].variables["Result"]

        assert reify(result_var, state.substitution) == atom("done")

    def test_unknown_module_qualification_raises_during_linking(self) -> None:
        family = load_swi_prolog_source(
            """
            :- module(app, []).
            ?- missing:main(Result).
            """,
        )

        with pytest.raises(
            ValueError,
            match=r"module qualification references unknown module missing",
        ):
            link_loaded_prolog_sources(family)


class TestPrologGoalAdapter:
    """The shared adapter should translate common Prolog builtin shapes."""

    @pytest.mark.parametrize(
        "goal",
        [
            relation("var", 1)(atom("X")),
            relation("nonvar", 1)(atom("x")),
            relation("ground", 1)(atom("x")),
            relation("atom", 1)(atom("x")),
            relation("atomic", 1)(atom("x")),
            relation("number", 1)(1),
            relation("string", 1)("hello"),
            relation("compound", 1)(term("pair", atom("a"), atom("b"))),
            relation("callable", 1)(term("memo", atom("ok"))),
            relation("call", 1)(term("memo", atom("ok"))),
            relation(
                "phrase",
                2,
            )(term("digits"), term(".", atom("a"), term(".", atom("b"), atom("[]")))),
            relation("phrase", 3)(
                term("digits"),
                term(".", atom("a"), term(".", atom("b"), atom("[]"))),
                atom("[]"),
            ),
            relation("once", 1)(term("memo", atom("ok"))),
            relation("not", 1)(term("memo", atom("missing"))),
            relation("\\+", 1)(term("memo", atom("missing"))),
            relation("functor", 3)(term("memo", atom("ok")), atom("memo"), 1),
            relation("arg", 3)(1, term("memo", atom("ok")), atom("ok")),
            relation("=..", 2)(
                term("memo", atom("ok")),
                term(".", atom("memo"), term(".", atom("ok"), atom("[]"))),
            ),
            relation("==", 2)(atom("a"), atom("a")),
            relation("compare", 3)(atom("<"), atom("a"), atom("b")),
            relation("@<", 2)(atom("a"), atom("b")),
            relation("@=<", 2)(atom("a"), atom("b")),
            relation("@>", 2)(atom("b"), atom("a")),
            relation("@>=", 2)(atom("b"), atom("a")),
            relation("asserta", 1)(term("memo", atom("ok"))),
            relation("assertz", 1)(term("memo", atom("ok"))),
            relation("retract", 1)(term("memo", atom("ok"))),
            relation("retractall", 1)(term("memo", atom("ok"))),
            relation("clause", 2)(term("memo", atom("ok")), atom("true")),
            relation("dynamic", 1)(term("/", atom("memo"), 1)),
            relation("abolish", 1)(term("/", atom("memo"), 1)),
            relation("current_predicate", 1)(term("/", atom("memo"), 1)),
            relation("predicate_property", 2)(
                term("/", atom("memo"), 1),
                atom("dynamic"),
            ),
            relation("predicate_property", 2)(atom("memo"), atom("defined")),
            relation("predicate_property", 2)(
                term("memo", atom("ok")),
                atom("defined"),
            ),
        ],
    )
    def test_adapt_prolog_goal_rewrites_supported_relation_calls(
        self,
        goal: RelationCall,
    ) -> None:
        adapted = adapt_prolog_goal(goal)

        assert adapted is not goal

    def test_adapt_prolog_goal_rewrites_indicator_lists(self) -> None:
        goal = relation("dynamic", 1)(
            term(
                ".",
                term("/", atom("memo"), 1),
                term(".", term("/", atom("cache"), 2), atom("[]")),
            ),
        )

        adapted = adapt_prolog_goal(goal)

        assert isinstance(adapted, ConjExpr)
        assert len(adapted.goals) == 2

    def test_adapt_prolog_goal_recurses_through_composite_expressions(self) -> None:
        composite = conj(
            relation("call", 1)(term("memo", atom("ok"))),
            disj(
                relation("dynamic", 1)(term("/", atom("memo"), 1)),
                relation("unknown", 1)(atom("value")),
            ),
            fresh(
                1,
                lambda pred: relation("predicate_property", 2)(
                    pred,
                    atom("defined"),
                ),
            ),
        )

        adapted = adapt_prolog_goal(composite)

        assert isinstance(adapted, ConjExpr)
        assert isinstance(adapted.goals[1], DisjExpr)
        assert isinstance(adapted.goals[2], FreshExpr)

    def test_adapt_prolog_goal_preserves_unsupported_shapes(self) -> None:
        variable_indicator = LogicVar(id=1)
        bad_dynamic = relation("dynamic", 1)(variable_indicator)
        bad_abolish = relation("abolish", 1)(atom("memo"))
        bad_current = relation("current_predicate", 1)(atom("memo"))
        unknown = relation("unknown_builtin", 1)(atom("memo"))

        assert adapt_prolog_goal(bad_dynamic) is bad_dynamic
        assert adapt_prolog_goal(bad_abolish) is bad_abolish
        assert adapt_prolog_goal(bad_current) is bad_current
        assert adapt_prolog_goal(unknown) is unknown

    def test_adapt_prolog_goal_exposes_variable_indicator_forms(self) -> None:
        predicate_indicator = LogicVar(id=2)
        current_goal = relation("current_predicate", 1)(predicate_indicator)
        property_goal = relation("predicate_property", 2)(
            predicate_indicator,
            atom("defined"),
        )

        assert isinstance(adapt_prolog_goal(current_goal), FreshExpr)
        assert isinstance(adapt_prolog_goal(property_goal), FreshExpr)
