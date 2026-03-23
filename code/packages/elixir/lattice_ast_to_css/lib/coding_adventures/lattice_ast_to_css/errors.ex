defmodule CodingAdventures.LatticeAstToCss.Errors do
  @moduledoc """
  Structured error types for the Lattice AST-to-CSS compiler.

  Every error in the Lattice compiler is represented as a tagged tuple:

      {:error, error_struct}

  Where `error_struct` is one of the structs defined here. Each struct
  carries a human-readable `message` and the `line`/`column` where the
  error occurred (from the triggering token).

  ## Error Hierarchy by Compiler Pass

  **Pass 1 — Module Resolution:**
  - `ModuleNotFoundError` — `@use` references a file that doesn't exist

  **Pass 2 — Symbol Collection:**
  - `ReturnOutsideFunctionError` — `@return` appears outside a `@function`

  **Pass 3 — Expansion:**
  - `UndefinedVariableError`  — `$var` referenced but never declared
  - `UndefinedMixinError`     — `@include` references an unknown mixin
  - `UndefinedFunctionError`  — function call references an unknown function
  - `WrongArityError`         — mixin/function called with wrong arg count
  - `CircularReferenceError`  — mixin or function calls itself (directly or
                                indirectly)
  - `TypeErrorInExpression`   — arithmetic on incompatible types
  - `UnitMismatchError`       — arithmetic on incompatible units
  - `MissingReturnError`      — function body has no `@return` statement

  ## Pattern for Error Handling

  In Elixir, we use `raise` for truly exceptional situations and `{:error, _}`
  tuples for expected failure modes. Lattice compiler errors are expected
  (user mistakes in source), so they use a throw-based approach internally
  and return `{:error, message}` at the public API boundary.

  ## Example

      case CodingAdventures.LatticeAstToCss.Transformer.transform(ast) do
        {:ok, css_ast} -> emit(css_ast)
        {:error, msg}  -> IO.puts("Compile error: " <> msg)
      end
  """

  # ---------------------------------------------------------------------------
  # Pass 1: Module Resolution Errors
  # ---------------------------------------------------------------------------

  defmodule ModuleNotFoundError do
    @moduledoc """
    Raised when `@use` references a module that cannot be found.

    Example: `@use "nonexistent";`
    """
    defstruct [:message, :module_name, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            module_name: String.t(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(module_name, line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "Module '#{module_name}' not found",
        module_name: module_name,
        line: line,
        column: column
      }
    end
  end

  # ---------------------------------------------------------------------------
  # Pass 2: Symbol Collection Errors
  # ---------------------------------------------------------------------------

  defmodule ReturnOutsideFunctionError do
    @moduledoc """
    Raised when `@return` appears outside a `@function` body.

    Example: `@return 42;` at the top level or inside a mixin.
    """
    defstruct [:message, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "@return outside @function",
        line: line,
        column: column
      }
    end
  end

  # ---------------------------------------------------------------------------
  # Pass 3: Expansion Errors
  # ---------------------------------------------------------------------------

  defmodule UndefinedVariableError do
    @moduledoc """
    Raised when a `$variable` is referenced but never declared.

    Example: `color: $nonexistent;`
    """
    defstruct [:message, :name, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            name: String.t(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(name, line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "Undefined variable '#{name}'",
        name: name,
        line: line,
        column: column
      }
    end
  end

  defmodule UndefinedMixinError do
    @moduledoc """
    Raised when `@include` references a mixin that was never defined.

    Example: `@include nonexistent;`
    """
    defstruct [:message, :name, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            name: String.t(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(name, line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "Undefined mixin '#{name}'",
        name: name,
        line: line,
        column: column
      }
    end
  end

  defmodule UndefinedFunctionError do
    @moduledoc """
    Raised when a function call references a function that was never defined.

    Note: this only applies to Lattice functions, not CSS functions like
    `rgb()`, `calc()`, `var()`, etc. CSS functions are passed through unchanged.

    Example: `padding: spacing(2);` (if `spacing` was never defined)
    """
    defstruct [:message, :name, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            name: String.t(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(name, line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "Undefined function '#{name}'",
        name: name,
        line: line,
        column: column
      }
    end
  end

  defmodule WrongArityError do
    @moduledoc """
    Raised when a mixin or function is called with the wrong number of args.

    The `expected` count accounts for parameters that have defaults —
    only parameters without defaults are required.

    Example: `@mixin button($bg, $fg)` called as `@include button(red, blue, green);`
    """
    defstruct [:message, :kind, :name, :expected, :got, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            kind: String.t(),
            name: String.t(),
            expected: non_neg_integer(),
            got: non_neg_integer(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(kind, name, expected, got, line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "#{kind} '#{name}' expects #{expected} args, got #{got}",
        kind: kind,
        name: name,
        expected: expected,
        got: got,
        line: line,
        column: column
      }
    end
  end

  defmodule CircularReferenceError do
    @moduledoc """
    Raised when a mixin or function calls itself, forming a cycle.

    The `chain` shows the full call path: `["a", "b", "a"]`.

    Example:
        @mixin a { @include b; }
        @mixin b { @include a; }    <- Circular mixin: a -> b -> a
    """
    defstruct [:message, :kind, :chain, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            kind: String.t(),
            chain: [String.t()],
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(kind, chain, line \\ 0, column \\ 0) do
      chain_str = Enum.join(chain, " -> ")
      %__MODULE__{
        message: "Circular #{kind}: #{chain_str}",
        kind: kind,
        chain: chain,
        line: line,
        column: column
      }
    end
  end

  defmodule TypeErrorInExpression do
    @moduledoc """
    Raised when arithmetic is attempted on incompatible types.

    Example: `10px + red` (can't add a dimension and a color/ident)
    """
    defstruct [:message, :op, :left_type, :right_type, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            op: String.t(),
            left_type: String.t(),
            right_type: String.t(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(op, left, right, line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "Cannot #{op} '#{left}' and '#{right}'",
        op: op,
        left_type: left,
        right_type: right,
        line: line,
        column: column
      }
    end
  end

  defmodule UnitMismatchError do
    @moduledoc """
    Raised when arithmetic combines dimensions with incompatible units.

    Example: `10px + 5s` (length + time — these can never be added)
    """
    defstruct [:message, :left_unit, :right_unit, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            left_unit: String.t(),
            right_unit: String.t(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(left_unit, right_unit, line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "Cannot add '#{left_unit}' and '#{right_unit}' units",
        left_unit: left_unit,
        right_unit: right_unit,
        line: line,
        column: column
      }
    end
  end

  defmodule MissingReturnError do
    @moduledoc """
    Raised when a function body has no `@return` statement.

    Every `@function` must return a value via `@return`. A function body
    that contains only variable declarations or control flow with no
    `@return` in any reachable branch is an error.

    Example: `@function noop($x) { $y: $x; }`
    """
    defstruct [:message, :name, line: 0, column: 0]

    @type t :: %__MODULE__{
            message: String.t(),
            name: String.t(),
            line: non_neg_integer(),
            column: non_neg_integer()
          }

    def new(name, line \\ 0, column \\ 0) do
      %__MODULE__{
        message: "Function '#{name}' has no @return",
        name: name,
        line: line,
        column: column
      }
    end
  end
end
