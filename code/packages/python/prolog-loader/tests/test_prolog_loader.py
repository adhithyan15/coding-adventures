"""Tests for parsed-source loading and explicit initialization execution."""

from __future__ import annotations

from pathlib import Path

import pytest
from logic_engine import (
    Atom,
    Compound,
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
    logic_list,
    num,
    program,
    reify,
    relation,
    solve_all,
    solve_from,
    string,
    term,
    visible_clauses_for,
)
from swi_prolog_parser import parse_swi_query

from prolog_loader import (
    LoadedPrologProject,
    PrologExpansionError,
    PrologInitializationError,
    SourceResolver,
    __version__,
    adapt_prolog_goal,
    link_loaded_prolog_sources,
    load_iso_prolog_source,
    load_swi_prolog_file,
    load_swi_prolog_project,
    load_swi_prolog_project_from_files,
    load_swi_prolog_source,
    rewrite_loaded_prolog_query,
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

    def test_load_swi_source_applies_term_expansion_to_clauses(self) -> None:
        loaded = load_swi_prolog_source(
            """
            :- term_expansion(edge(X, Y), link(X, Y)).
            edge(homer, bart).
            ?- link(homer, Who).
            """,
        )
        query = loaded.queries[0]

        assert solve_all(loaded.program, query.variables["Who"], query.goal) == [
            atom("bart"),
        ]

    def test_load_swi_source_term_expansion_supports_lists_and_repeatable_passes(
        self,
    ) -> None:
        loaded = load_swi_prolog_source(
            """
            :- term_expansion(bundle(X), [left(X), middle(X)]).
            :- term_expansion(middle(X), right(X)).
            bundle(ok).
            ?- left(Value).
            ?- right(Value).
            """,
        )
        first_query = loaded.queries[0]
        second_query = loaded.queries[1]

        assert solve_all(
            loaded.program,
            first_query.variables["Value"],
            first_query.goal,
        ) == [atom("ok")]
        assert solve_all(
            loaded.program,
            second_query.variables["Value"],
            second_query.goal,
        ) == [atom("ok")]

    def test_load_swi_source_applies_goal_expansion_to_queries_and_initialization(
        self,
    ) -> None:
        loaded = load_swi_prolog_source(
            """
            :- goal_expansion(run, main).
            :- initialization(run).
            main.
            ?- run.
            """,
        )
        query = loaded.queries[0]

        assert str(loaded.initialization_terms[0]) == "main"
        assert next(solve_from(loaded.program, query.goal, State()), None) is not None
        assert run_initialization_goals(loaded) == State()

    def test_load_swi_source_raises_for_invalid_term_expansion_output(self) -> None:
        with pytest.raises(
            PrologExpansionError,
            match=r"term expansion produced invalid clause term: 42",
        ):
            load_swi_prolog_source(
                """
                :- term_expansion(edge, 42).
                edge.
                """,
            )

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

    def test_rewrite_loaded_prolog_query_uses_module_import_context(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(family, [ancestor/2]).
            ancestor(homer, bart).
            ancestor(homer, lisa).
            """,
            """
            :- module(app, []).
            :- use_module(family, [ancestor/2]).
            """,
        )

        query = rewrite_loaded_prolog_query(
            project,
            parse_swi_query("?- ancestor(homer, Who)."),
            module="app",
        )

        assert solve_all(project.program, query.variables["Who"], query.goal) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_rewrite_loaded_prolog_query_rejects_unknown_query_module(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(app, []).
            """,
        )

        with pytest.raises(ValueError, match="query module missing"):
            rewrite_loaded_prolog_query(
                project,
                parse_swi_query("?- anything."),
                module="missing",
            )

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

    def test_load_swi_prolog_file_tracks_source_path_and_dependencies(
        self,
        tmp_path: Path,
    ) -> None:
        family_path = tmp_path / "family.pl"
        family_path.write_text(
            ":- module(family, [ancestor/2]).\nancestor(homer, bart).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, [run/1]).\n"
            ":- use_module(family, [ancestor/2]).\n"
            ":- consult('facts.pl').\n"
            "run(Who) :- ancestor(homer, Who).\n",
            encoding="utf-8",
        )

        loaded = load_swi_prolog_file(app_path)

        assert loaded.source_path == app_path.resolve()
        assert [dependency.kind for dependency in loaded.file_dependencies] == [
            "use_module",
            "consult",
        ]
        assert loaded.file_dependencies[0].resolved_path == family_path.resolve()
        assert loaded.file_dependencies[1].resolved_path == (
            tmp_path / "facts.pl"
        ).resolve()

    def test_load_swi_prolog_project_from_files_loads_consulted_user_sources(
        self,
        tmp_path: Path,
    ) -> None:
        facts_path = tmp_path / "facts.pl"
        facts_path.write_text(
            "parent(homer, bart).\nparent(bart, lisa).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- consult(facts).\n?- parent(homer, Who).\n",
            encoding="utf-8",
        )

        project = load_swi_prolog_project_from_files(app_path)
        query = project.queries[0]

        assert solve_all(project.program, query.variables["Who"], query.goal) == [
            atom("bart"),
        ]

    def test_load_swi_prolog_project_from_files_resolves_use_module_files(
        self,
        tmp_path: Path,
    ) -> None:
        family_path = tmp_path / "family.pl"
        family_path.write_text(
            ":- module(family, [ancestor/2]).\n"
            "ancestor(homer, bart).\n"
            "ancestor(bart, lisa).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, [run/1]).\n"
            ":- use_module(family, [ancestor/2]).\n"
            "run(Who) :- ancestor(homer, Who).\n"
            "?- run(Who).\n",
            encoding="utf-8",
        )

        project = load_swi_prolog_project_from_files(app_path)
        query = project.queries[0]

        assert solve_all(project.program, query.variables["Who"], query.goal) == [
            atom("bart"),
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

    def test_module_qualification_rewrites_call_n_meta_arguments(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(family, [ancestor/2]).
            ancestor(homer, bart).
            ancestor(homer, lisa).
            """,
            """
            :- module(app, []).
            ?- call(family:ancestor, homer, Who).
            """,
        )

        query = project.queries[0]

        assert solve_all(
            project.program,
            query.variables["Who"],
            adapt_prolog_goal(query.goal),
        ) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_module_qualification_rewrites_apply_family_closures(self) -> None:
        project = load_swi_prolog_project(
            """
            :- module(apply_helpers, [
                increment/2,
                convert/2,
                small/1,
                pair_push/4,
                push/3
            ]).
            increment(1, 2).
            increment(2, 3).
            convert(1, one).
            convert(3, three).
            small(1).
            small(2).
            pair_push(Left, Right, Acc, [pair(Left, Right)|Acc]).
            push(Item, Acc, [Item|Acc]).
            """,
            """
            :- module(app, []).
            :- use_module(apply_helpers, [
                increment/2,
                convert/2,
                small/1,
                pair_push/4,
                push/3
            ]).
            ?- maplist(increment, [1,2], Ys),
               convlist(convert, [1,2,3], Converted),
               include(small, [1,2,3], Small),
               foldl(pair_push, [a,b], [x,y], [], Folded),
               scanl(push, [a,b], [], Scanned).
            """,
        )
        query = project.queries[0]

        assert solve_all(
            project.program,
            (
                query.variables["Ys"],
                query.variables["Converted"],
                query.variables["Small"],
                query.variables["Folded"],
                query.variables["Scanned"],
            ),
            adapt_prolog_goal(query.goal),
        ) == [
            (
                logic_list([2, 3]),
                logic_list(["one", "three"]),
                logic_list([1, 2]),
                logic_list([term("pair", "b", "y"), term("pair", "a", "x")]),
                logic_list([logic_list(["a"]), logic_list(["b", "a"])]),
            ),
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

    def test_load_swi_prolog_project_from_files_resolves_relative_use_module_paths(
        self,
        tmp_path: Path,
    ) -> None:
        family_path = tmp_path / "family.pl"
        family_path.write_text(
            ":- module(family, [main/1]).\nmain(done).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, []).\n"
            ":- use_module('./family.pl', [main/1]).\n"
            ":- initialization(main(Result)).\n",
            encoding="utf-8",
        )

        project = load_swi_prolog_project_from_files(app_path)
        state = run_prolog_initialization_goals(project)
        result_var = project.initialization_directives[0].variables["Result"]

        assert family_path.resolve() in {
            source.source_path for source in project.sources if source.source_path
        }
        assert reify(result_var, state.substitution) == atom("done")

    def test_use_module_file_targets_must_declare_a_module(
        self,
        tmp_path: Path,
    ) -> None:
        helper_path = tmp_path / "helper.pl"
        helper_path.write_text("helper(ok).\n", encoding="utf-8")
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, []).\n:- use_module(helper, [helper/1]).\n",
            encoding="utf-8",
        )

        expected_message = (
            r"use_module file target .*helper\.pl "
            r"must load a module/2 declaration"
        )
        with pytest.raises(
            ValueError,
            match=expected_message,
        ):
            load_swi_prolog_project_from_files(app_path)

    def test_load_swi_prolog_project_from_files_splices_include_into_parent(
        self,
        tmp_path: Path,
    ) -> None:
        facts_path = tmp_path / "facts.pl"
        facts_path.write_text("parent(homer, bart).\n", encoding="utf-8")
        helper_path = tmp_path / "helpers.pl"
        helper_path.write_text(
            ":- consult('facts.pl').\nrun(Who) :- parent(homer, Who).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, [run/1]).\n"
            ":- include('helpers.pl').\n"
            "?- run(Who).\n",
            encoding="utf-8",
        )

        project = load_swi_prolog_project_from_files(app_path)
        query = project.queries[0]

        assert solve_all(project.program, query.variables["Who"], query.goal) == [
            atom("bart"),
        ]

    def test_include_target_must_not_declare_a_module(
        self,
        tmp_path: Path,
    ) -> None:
        helper_path = tmp_path / "helpers.pl"
        helper_path.write_text(
            ":- module(helpers, [helper/1]).\nhelper(ok).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, []).\n:- include('helpers.pl').\n",
            encoding="utf-8",
        )

        with pytest.raises(
            ValueError,
            match=r"included file .*helpers\.pl must not declare module/2",
        ):
            load_swi_prolog_file(app_path)

    def test_load_swi_prolog_file_rejects_circular_includes(
        self,
        tmp_path: Path,
    ) -> None:
        first_path = tmp_path / "first.pl"
        second_path = tmp_path / "second.pl"
        first_path.write_text(":- include('second.pl').\n", encoding="utf-8")
        second_path.write_text(":- include('first.pl').\n", encoding="utf-8")

        with pytest.raises(
            ValueError,
            match=r"circular include detected at .*first\.pl",
        ):
            load_swi_prolog_file(first_path)

    def test_load_swi_prolog_project_from_files_uses_custom_library_resolver(
        self,
        tmp_path: Path,
    ) -> None:
        library_dir = tmp_path / "lib"
        library_dir.mkdir()
        family_path = library_dir / "family.pl"
        family_path.write_text(
            ":- module(family, [ancestor/2]).\nancestor(homer, bart).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, []).\n"
            ":- use_module(library(family), [ancestor/2]).\n"
            "?- ancestor(homer, Who).\n",
            encoding="utf-8",
        )

        def resolve_library(
            term_value: object,
            source_path: Path,
        ) -> Path | None:
            del source_path
            if (
                isinstance(term_value, Compound)
                and term_value.functor.name == "library"
                and len(term_value.args) == 1
                and isinstance(term_value.args[0], Atom)
            ):
                return library_dir / f"{term_value.args[0].symbol.name}.pl"
            return None

        source_resolver: SourceResolver = resolve_library

        project = load_swi_prolog_project_from_files(
            app_path,
            source_resolver=source_resolver,
        )
        query = project.queries[0]

        assert family_path.resolve() in {
            source.source_path for source in project.sources if source.source_path
        }
        assert solve_all(project.program, query.variables["Who"], query.goal) == [
            atom("bart"),
        ]


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
            relation("integer", 1)(1),
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
            relation("->", 2)(term("memo", atom("ok")), term("memo", atom("then"))),
            relation("not", 1)(term("memo", atom("missing"))),
            relation("\\+", 1)(term("memo", atom("missing"))),
            relation("functor", 3)(term("memo", atom("ok")), atom("memo"), 1),
            relation("arg", 3)(1, term("memo", atom("ok")), atom("ok")),
            relation("=..", 2)(
                term("memo", atom("ok")),
                term(".", atom("memo"), term(".", atom("ok"), atom("[]"))),
            ),
            relation("atom_chars", 2)(atom("tea"), logic_list(["t", "e", "a"])),
            relation("atom_codes", 2)(atom("tea"), logic_list([116, 101, 97])),
            relation("atom_concat", 3)(atom("tea"), atom("cup"), atom("teacup")),
            relation("atom_length", 2)(atom("teacup"), 6),
            relation("atomic_list_concat", 2)(
                logic_list(["tea", "cup"]),
                atom("teacup"),
            ),
            relation("atomic_list_concat", 3)(
                logic_list(["tea", "cup"]),
                atom("-"),
                atom("tea-cup"),
            ),
            relation("number_chars", 2)(42, logic_list(["4", "2"])),
            relation("number_codes", 2)(42, logic_list([52, 50])),
            relation("number_string", 2)(42, string("42")),
            relation("char_code", 2)(atom("A"), 65),
            relation("string_chars", 2)(string("hi"), logic_list(["h", "i"])),
            relation("string_codes", 2)(string("hi"), logic_list([104, 105])),
            relation("string_length", 2)(string("hi"), 2),
            relation("sub_atom", 5)(
                atom("teacup"),
                3,
                3,
                0,
                atom("cup"),
            ),
            relation("sub_string", 5)(
                string("logic"),
                2,
                2,
                1,
                string("gi"),
            ),
            relation("=", 2)(LogicVar(id=14), atom("a")),
            relation("\\=", 2)(atom("a"), atom("b")),
            relation("dif", 2)(LogicVar(id=15), atom("tea")),
            relation("==", 2)(atom("a"), atom("a")),
            relation("\\==", 2)(atom("a"), atom("b")),
            relation("=@=", 2)(
                term("box", LogicVar(id=78)),
                term("box", LogicVar(id=79)),
            ),
            relation("\\=@=", 2)(
                term("pair", LogicVar(id=80), LogicVar(id=80)),
                term("pair", LogicVar(id=81), LogicVar(id=82)),
            ),
            relation("subsumes_term", 2)(
                term("box", LogicVar(id=83)),
                term("box", atom("tea")),
            ),
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
            relation("true", 0)(),
            relation("fail", 0)(),
            relation("!", 0)(),
            relation("is", 2)(LogicVar(id=10), term("+", 1, 2)),
            relation("succ", 2)(1, LogicVar(id=29)),
            relation("#=", 2)(LogicVar(id=30), term("+", LogicVar(id=31), 1)),
            relation("#\\=", 2)(LogicVar(id=32), LogicVar(id=33)),
            relation("#<", 2)(LogicVar(id=34), LogicVar(id=35)),
            relation("#=<", 2)(LogicVar(id=36), LogicVar(id=37)),
            relation("#>", 2)(LogicVar(id=38), LogicVar(id=39)),
            relation("#>=", 2)(LogicVar(id=40), LogicVar(id=41)),
            relation("in", 2)(LogicVar(id=42), logic_list([1, 2, 3])),
            relation("ins", 2)(
                logic_list([LogicVar(id=43)]),
                logic_list([1, 2, 3]),
            ),
            relation("all_different", 1)(
                logic_list([LogicVar(id=44), LogicVar(id=45)])
            ),
            relation("all_distinct", 1)(
                logic_list([LogicVar(id=46), LogicVar(id=47)])
            ),
            relation("labeling", 2)(logic_list([]), logic_list([LogicVar(id=48)])),
            relation("label", 1)(logic_list([LogicVar(id=49)])),
            relation("=:=", 2)(term("+", 1, 2), 3),
            relation("=\\=", 2)(term("+", 1, 2), 4),
            relation("<", 2)(1, 2),
            relation("=<", 2)(1, 1),
            relation(">", 2)(2, 1),
            relation(">=", 2)(2, 2),
            relation("between", 3)(1, 3, LogicVar(id=28)),
            relation("findall", 3)(
                atom("ok"),
                term("memo", atom("ok")),
                LogicVar(id=11),
            ),
            relation("bagof", 3)(atom("ok"), term("memo", atom("ok")), LogicVar(id=12)),
            relation("setof", 3)(atom("ok"), term("memo", atom("ok")), LogicVar(id=13)),
            relation("forall", 2)(term("memo", atom("ok")), term("memo", atom("ok"))),
            relation("copy_term", 2)(term("box", LogicVar(id=15)), LogicVar(id=14)),
            relation("term_variables", 2)(
                term("box", LogicVar(id=16)),
                LogicVar(id=17),
            ),
            relation("current_prolog_flag", 2)(
                atom("unknown"),
                LogicVar(id=18),
            ),
            relation("set_prolog_flag", 2)(atom("unknown"), atom("error")),
            relation("is_list", 1)(
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
            ),
            relation("last", 2)(
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                atom("cake"),
            ),
            relation("length", 2)(
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                2,
            ),
            relation("member", 2)(
                atom("tea"),
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
            ),
            relation("msort", 2)(
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=20),
            ),
            relation("permutation", 2)(
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=16),
            ),
            relation("reverse", 2)(
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=17),
            ),
            relation("sort", 2)(
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=21),
            ),
            relation("append", 3)(
                term(".", atom("tea"), atom("[]")),
                term(".", atom("cake"), atom("[]")),
                LogicVar(id=18),
            ),
            relation("nth0", 3)(
                1,
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=22),
            ),
            relation("nth1", 3)(
                2,
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=23),
            ),
            relation("nth0", 4)(
                1,
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=24),
                LogicVar(id=25),
            ),
            relation("nth1", 4)(
                2,
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=26),
                LogicVar(id=27),
            ),
            relation("select", 3)(
                atom("tea"),
                term(".", atom("tea"), term(".", atom("cake"), atom("[]"))),
                LogicVar(id=19),
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

    def test_adapt_prolog_goal_rewrites_if_then_else_control(self) -> None:
        parsed = parse_swi_query(
            "?- ((X = first ; X = second) -> Result = X ; Result = none).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            parsed.variables["Result"],
            adapted,
        ) == [atom("first")]

    def test_adapt_prolog_goal_rewrites_term_equality_predicates(self) -> None:
        parsed = parse_swi_query(
            "?- X = box(tea), X == box(tea), X \\== box(cake), "
            "X \\= box(cake), Result = ok.",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (parsed.variables["X"], parsed.variables["Result"]),
            adapted,
        ) == [(term("box", "tea"), atom("ok"))]

    def test_adapt_prolog_goal_rewrites_variant_and_subsumes_predicates(self) -> None:
        parsed = parse_swi_query(
            "?- pair(X, X) =@= pair(Y, Y), "
            "pair(X, X) \\=@= pair(Y, Z), "
            "subsumes_term(box(A), box(tea)), Result = ok.",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            parsed.variables["Result"],
            adapted,
        ) == [atom("ok")]

    def test_adapt_prolog_goal_rewrites_text_conversion_predicates(self) -> None:
        parsed = parse_swi_query(
            "?- atom_chars(tea, Chars), "
            "atom_codes(Atom, [116, 101, 97]), "
            "number_chars(Number, ['4', '2']), "
            "number_codes(Float, [51, 46, 53]), "
            "number_string(Parsed, \"7\"), "
            "atom_concat(tea, cup, Joined), "
            "atom_concat(Prefix, cup, teacup), "
            "atom_length(teacup, AtomLength), "
            "sub_atom(teacup, 3, 3, 0, SubAtom), "
            "atomic_list_concat([tea, 2, go], '-', AtomList), "
            "atomic_list_concat(Split, '-', 'tea-cup'), "
            "char_code(Char, 90), "
            "string_chars(String, [h, i]), "
            "string_length(\"hello\", StringLength), "
            "sub_string(\"logic\", 2, 2, 1, SubString), "
            'string_codes("ok", Codes).',
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (
                parsed.variables["Chars"],
                parsed.variables["Atom"],
                parsed.variables["Number"],
                parsed.variables["Float"],
                parsed.variables["Parsed"],
                parsed.variables["Joined"],
                parsed.variables["Prefix"],
                parsed.variables["AtomLength"],
                parsed.variables["SubAtom"],
                parsed.variables["AtomList"],
                parsed.variables["Split"],
                parsed.variables["Char"],
                parsed.variables["String"],
                parsed.variables["StringLength"],
                parsed.variables["SubString"],
                parsed.variables["Codes"],
            ),
            adapted,
        ) == [
            (
                logic_list(["t", "e", "a"]),
                atom("tea"),
                num(42),
                num(3.5),
                num(7),
                atom("teacup"),
                atom("tea"),
                num(6),
                atom("cup"),
                atom("tea-2-go"),
                logic_list(["tea", "cup"]),
                atom("Z"),
                string("hi"),
                num(5),
                string("gi"),
                logic_list([111, 107]),
            ),
        ]

    def test_adapt_prolog_goal_rewrites_term_equality_failures(self) -> None:
        parsed_unifiable = parse_swi_query("?- X \\= box(tea).")
        parsed_identical = parse_swi_query("?- X = box(tea), X \\== box(tea).")
        parsed_equal = parse_swi_query("?- X = box(tea), X \\= box(tea).")

        assert solve_all(
            program(),
            parsed_unifiable.variables["X"],
            adapt_prolog_goal(parsed_unifiable.goal),
        ) == []
        assert solve_all(
            program(),
            parsed_identical.variables["X"],
            adapt_prolog_goal(parsed_identical.goal),
        ) == []
        assert solve_all(
            program(),
            parsed_equal.variables["X"],
            adapt_prolog_goal(parsed_equal.goal),
        ) == []

    def test_adapt_prolog_goal_rewrites_dif_as_delayed_disequality(self) -> None:
        parsed = parse_swi_query(
            "?- dif(X, tea), X = cake, dif(Left, Right), "
            "Left = box(tea), Right = box(cake).",
        )
        parsed_failure = parse_swi_query("?- dif(X, tea), X = tea.")

        assert solve_all(
            program(),
            (
                parsed.variables["X"],
                parsed.variables["Left"],
                parsed.variables["Right"],
            ),
            adapt_prolog_goal(parsed.goal),
        ) == [(atom("cake"), term("box", "tea"), term("box", "cake"))]
        assert solve_all(
            program(),
            parsed_failure.variables["X"],
            adapt_prolog_goal(parsed_failure.goal),
        ) == []

    def test_adapt_prolog_goal_uses_else_branch_from_original_state(self) -> None:
        parsed = parse_swi_query(
            "?- (fail -> Result = then ; Result = else).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            parsed.variables["Result"],
            adapted,
        ) == [atom("else")]

    def test_adapt_prolog_goal_rewrites_between_generator(self) -> None:
        parsed = parse_swi_query(
            "?- between(1, 4, Value), succ(Value, Next), integer(Next), Next > 3.",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (parsed.variables["Value"], parsed.variables["Next"]),
            adapted,
        ) == [
            (num(3), num(4)),
            (num(4), num(5)),
        ]

    def test_adapt_prolog_goal_rewrites_call_n(self) -> None:
        parsed = parse_swi_query(
            "?- call(member, Item, [tea, cake]).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            parsed.variables["Item"],
            adapted,
        ) == [atom("tea"), atom("cake")]

    def test_adapt_prolog_goal_rewrites_higher_order_list_builtins(self) -> None:
        loaded = load_swi_prolog_source(
            """
            small(1).
            small(2).
            increment(1, 2).
            increment(2, 3).
            increment(3, 4).
            push(Item, Acc, [Item|Acc]).

            ?- maplist(increment, [1,2,3], Ys),
               include(small, [1,2,3], Small),
               exclude(small, [1,2,3], Big),
               partition(small, [1,2,3], Yes, No),
               foldl(push, [a,b,c], [], Stack).
            """,
        )
        query = loaded.queries[0]
        adapted = adapt_prolog_goal(query.goal)

        assert solve_all(
            loaded.program,
            (
                query.variables["Ys"],
                query.variables["Small"],
                query.variables["Big"],
                query.variables["Yes"],
                query.variables["No"],
                query.variables["Stack"],
            ),
            adapted,
        ) == [
            (
                logic_list([2, 3, 4]),
                logic_list([1, 2]),
                logic_list([3]),
                logic_list([1, 2]),
                logic_list([3]),
                logic_list(["c", "b", "a"]),
            ),
        ]

    def test_adapt_prolog_goal_rewrites_apply_family_builtins(self) -> None:
        loaded = load_swi_prolog_source(
            """
            join4(A, B, C, joined(A, B, C)).
            convert(1, one).
            convert(3, three).
            pair_push(Left, Right, Acc, [pair(Left, Right)|Acc]).
            push(Item, Acc, [Item|Acc]).

            ?- maplist(join4, [a,b], [x,y], [1,2], Joined),
               convlist(convert, [1,2,3], Converted),
               foldl(pair_push, [a,b], [x,y], [], Folded),
               scanl(push, [a,b], [], Scanned).
            """,
        )
        query = loaded.queries[0]
        adapted = adapt_prolog_goal(query.goal)

        assert solve_all(
            loaded.program,
            (
                query.variables["Joined"],
                query.variables["Converted"],
                query.variables["Folded"],
                query.variables["Scanned"],
            ),
            adapted,
        ) == [
            (
                logic_list([term("joined", "a", "x", 1), term("joined", "b", "y", 2)]),
                logic_list(["one", "three"]),
                logic_list([term("pair", "b", "y"), term("pair", "a", "x")]),
                logic_list([logic_list(["a"]), logic_list(["b", "a"])]),
            ),
        ]

    def test_adapt_prolog_goal_rewrites_clpfd_callable_forms(self) -> None:
        parsed = parse_swi_query(
            "?- ins([X,Y], [1,2,3]), "
            "in(Z, [1,2,3,4,5,6]), "
            "#<(X,Y), "
            "#=(Z, +(X,Y)), "
            "all_different([X,Y]), "
            "labeling([], [X,Y,Z]).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (parsed.variables["X"], parsed.variables["Y"], parsed.variables["Z"]),
            adapted,
        ) == [
            (num(1), num(2), num(3)),
            (num(1), num(3), num(4)),
            (num(2), num(3), num(5)),
        ]

    def test_adapt_prolog_goal_flattens_clpfd_sum_equality(self) -> None:
        parsed = parse_swi_query(
            "?- [X,Y] ins 1..3, "
            "Z in 4..6, "
            "X #< Y, "
            "Z #= X + Y + 1, "
            "labeling([], [X,Y,Z]).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (parsed.variables["X"], parsed.variables["Y"], parsed.variables["Z"]),
            adapted,
        ) == [
            (num(1), num(2), num(4)),
            (num(1), num(3), num(5)),
            (num(2), num(3), num(6)),
        ]

    def test_adapt_prolog_goal_honors_labeling_order_options(self) -> None:
        parsed = parse_swi_query(
            "?- X in 1..3, "
            "labeling([down], [X]).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            parsed.variables["X"],
            adapted,
        ) == [num(3), num(2), num(1)]

    def test_adapt_prolog_goal_rewrites_clpfd_sum_global(self) -> None:
        parsed = parse_swi_query(
            "?- [X,Y,Z] ins 1..4, "
            "sum([X,Y,Z], #=, 6), "
            "X #< Y, "
            "Y #< Z, "
            "labeling([], [X,Y,Z]).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (parsed.variables["X"], parsed.variables["Y"], parsed.variables["Z"]),
            adapted,
        ) == [
            (num(1), num(2), num(3)),
        ]

    def test_adapt_prolog_goal_rewrites_clpfd_scalar_product(self) -> None:
        parsed = parse_swi_query(
            "?- [X,Y] ins 0..4, "
            "scalar_product([2,3], [X,Y], #=, 12), "
            "X #< Y, "
            "labeling([], [X,Y]).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (parsed.variables["X"], parsed.variables["Y"]),
            adapted,
        ) == [
            (num(0), num(4)),
        ]

    def test_adapt_prolog_goal_rewrites_clpfd_modeling_globals(self) -> None:
        parsed = parse_swi_query(
            "?- [I,X,Y,Z] ins 1..4, "
            "I #= 2, "
            "element(I, [X,Y,Z], 4), "
            "sum([X,Y,Z], #=<, 8), "
            "scalar_product([2,1,1], [X,Y,Z], #>, 8), "
            "all_different([X,Y,Z]), "
            "labeling([], [I,X,Y,Z]).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (
                parsed.variables["I"],
                parsed.variables["X"],
                parsed.variables["Y"],
                parsed.variables["Z"],
            ),
            adapted,
        ) == [
            (num(2), num(1), num(4), num(3)),
            (num(2), num(2), num(4), num(1)),
            (num(2), num(3), num(4), num(1)),
        ]

    def test_adapt_prolog_goal_rewrites_clpfd_reification(self) -> None:
        parsed = parse_swi_query(
            "?- [X,Y] ins 1..2, "
            "(X #< Y) #<==> B, "
            "(#\\ B) #<==> NB, "
            "labeling([], [X,Y,B,NB]).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (
                parsed.variables["X"],
                parsed.variables["Y"],
                parsed.variables["B"],
                parsed.variables["NB"],
            ),
            adapted,
        ) == [
            (num(1), num(1), num(0), num(1)),
            (num(1), num(2), num(1), num(0)),
            (num(2), num(1), num(0), num(1)),
            (num(2), num(2), num(0), num(1)),
        ]

    def test_adapt_prolog_goal_rewrites_common_list_predicates(self) -> None:
        parsed = parse_swi_query(
            "?- member(Item, [tea, cake]), "
            "append([Item], [jam], Combined), "
            "reverse(Combined, Reversed), "
            "sort([Item, jam, Item], UniqueSorted), "
            "msort([Item, jam, Item], Sorted), "
            "nth0(1, Reversed, ZeroBased), "
            "nth1(2, Reversed, OneBased), "
            "nth0(1, Reversed, ZeroRestBased, ZeroRest), "
            "nth1(2, Reversed, OneRestBased, OneRest), "
            "length(Reversed, Count).",
        )

        adapted = adapt_prolog_goal(parsed.goal)

        assert solve_all(
            program(),
            (
                parsed.variables["Item"],
                parsed.variables["Combined"],
                parsed.variables["Reversed"],
                parsed.variables["UniqueSorted"],
                parsed.variables["Sorted"],
                parsed.variables["ZeroBased"],
                parsed.variables["OneBased"],
                parsed.variables["ZeroRestBased"],
                parsed.variables["ZeroRest"],
                parsed.variables["OneRestBased"],
                parsed.variables["OneRest"],
                parsed.variables["Count"],
            ),
            adapted,
        ) == [
            (
                atom("tea"),
                logic_list(["tea", "jam"]),
                logic_list(["jam", "tea"]),
                logic_list(["jam", "tea"]),
                logic_list(["jam", "tea", "tea"]),
                atom("tea"),
                atom("tea"),
                atom("tea"),
                logic_list(["jam"]),
                atom("tea"),
                logic_list(["jam"]),
                num(2),
            ),
            (
                atom("cake"),
                logic_list(["cake", "jam"]),
                logic_list(["jam", "cake"]),
                logic_list(["cake", "jam"]),
                logic_list(["cake", "cake", "jam"]),
                atom("cake"),
                atom("cake"),
                atom("cake"),
                logic_list(["jam"]),
                atom("cake"),
                logic_list(["jam"]),
                num(2),
            ),
        ]

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
