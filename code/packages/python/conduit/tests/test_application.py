"""Tests for Conduit application DSL — route registration, filters, settings."""

from coding_adventures.conduit.application import Conduit, _flask_to_rust_pattern

# ── Pattern conversion ───────────────────────────────────────────────────────


class TestFlaskToRustPattern:
    def test_no_params_unchanged(self) -> None:
        assert _flask_to_rust_pattern("/") == "/"
        assert _flask_to_rust_pattern("/static") == "/static"

    def test_single_param(self) -> None:
        assert _flask_to_rust_pattern("/hello/<name>") == "/hello/:name"

    def test_multiple_params(self) -> None:
        assert (
            _flask_to_rust_pattern("/users/<id>/posts/<pid>") == "/users/:id/posts/:pid"
        )

    def test_param_at_root(self) -> None:
        assert _flask_to_rust_pattern("/<resource>") == "/:resource"

    def test_no_angle_brackets_unchanged(self) -> None:
        assert _flask_to_rust_pattern("/api/v1/data") == "/api/v1/data"


# ── Route registration ───────────────────────────────────────────────────────


class TestConduitRouteRegistration:
    def test_get_registers_route(self) -> None:
        app = Conduit()

        @app.get("/")
        def index(ctx):
            pass

        assert len(app.routes) == 1
        assert app.routes[0].method == "GET"
        assert app.routes[0].pattern == "/"

    def test_post_registers_route(self) -> None:
        app = Conduit()

        @app.post("/submit")
        def submit(ctx):
            pass

        assert app.routes[0].method == "POST"

    def test_put_registers_route(self) -> None:
        app = Conduit()

        @app.put("/item")
        def update(ctx):
            pass

        assert app.routes[0].method == "PUT"

    def test_patch_registers_route(self) -> None:
        app = Conduit()

        @app.patch("/item")
        def partial(ctx):
            pass

        assert app.routes[0].method == "PATCH"

    def test_delete_registers_route(self) -> None:
        app = Conduit()

        @app.delete("/item")
        def remove(ctx):
            pass

        assert app.routes[0].method == "DELETE"

    def test_head_registers_route(self) -> None:
        app = Conduit()

        @app.head("/check")
        def check(ctx):
            pass

        assert app.routes[0].method == "HEAD"

    def test_options_registers_route(self) -> None:
        app = Conduit()

        @app.options("/resource")
        def opts(ctx):
            pass

        assert app.routes[0].method == "OPTIONS"

    def test_decorator_returns_original_function(self) -> None:
        app = Conduit()

        @app.get("/")
        def index(ctx):
            return "hello"

        # Decorated function must still be callable normally.
        assert index(None) == "hello"

    def test_pattern_converted_from_flask_to_rust(self) -> None:
        app = Conduit()

        @app.get("/user/<id>")
        def user(ctx):
            pass

        assert app.routes[0].pattern == "/user/:id"

    def test_multiple_routes_in_order(self) -> None:
        app = Conduit()

        @app.get("/a")
        def a(ctx):
            pass

        @app.post("/b")
        def b(ctx):
            pass

        assert len(app.routes) == 2
        assert app.routes[0].pattern == "/a"
        assert app.routes[1].pattern == "/b"


# ── Filter registration ───────────────────────────────────────────────────────


class TestConduitFilterRegistration:
    def test_before_request_appended(self) -> None:
        app = Conduit()

        @app.before_request
        def auth(ctx):
            pass

        assert len(app.before_filters) == 1
        assert app.before_filters[0] is auth

    def test_after_request_appended(self) -> None:
        app = Conduit()

        @app.after_request
        def log(ctx):
            pass

        assert len(app.after_filters) == 1
        assert app.after_filters[0] is log

    def test_not_found_stored(self) -> None:
        app = Conduit()

        @app.not_found
        def missing(ctx):
            pass

        assert app.not_found_handler is missing

    def test_error_handler_stored(self) -> None:
        app = Conduit()

        @app.error_handler
        def on_error(ctx, err):
            pass

        assert app.error_handler_fn is on_error

    def test_before_request_returns_function(self) -> None:
        app = Conduit()

        @app.before_request
        def fn(ctx):
            return 42

        assert fn(None) == 42

    def test_multiple_before_filters_in_order(self) -> None:
        app = Conduit()

        @app.before_request
        def first(ctx):
            pass

        @app.before_request
        def second(ctx):
            pass

        assert app.before_filters[0] is first
        assert app.before_filters[1] is second


# ── Settings ─────────────────────────────────────────────────────────────────


class TestConduitSettings:
    def test_settings_is_dict(self) -> None:
        app = Conduit()
        assert isinstance(app.settings, dict)

    def test_set_and_get(self) -> None:
        app = Conduit()
        app.settings["app_name"] = "My App"
        assert app.settings["app_name"] == "My App"

    def test_independent_per_app(self) -> None:
        app1 = Conduit()
        app2 = Conduit()
        app1.settings["x"] = 1
        assert "x" not in app2.settings


# ── Initial state ─────────────────────────────────────────────────────────────


class TestConduitInitialState:
    def test_no_routes(self) -> None:
        assert Conduit().routes == []

    def test_no_before_filters(self) -> None:
        assert Conduit().before_filters == []

    def test_no_after_filters(self) -> None:
        assert Conduit().after_filters == []

    def test_no_not_found_handler(self) -> None:
        assert Conduit().not_found_handler is None

    def test_no_error_handler(self) -> None:
        assert Conduit().error_handler_fn is None
