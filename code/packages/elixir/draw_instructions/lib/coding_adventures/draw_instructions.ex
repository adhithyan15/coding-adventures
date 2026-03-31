defmodule CodingAdventures.DrawInstructions do
  @moduledoc """
  Backend-neutral draw scene primitives.

  This module separates producer logic from renderer logic:

  - **Producer packages** decide *what* should be drawn (rectangles, text,
    lines, groups, clip regions).
  - **Renderer packages** decide *how* to serialize or paint that scene
    (SVG, PNG, Canvas, terminal, etc.).

  ## Scene model

  A scene is a flat list of instructions wrapped in a `DrawScene` struct
  that carries overall dimensions and a background colour.  Instructions
  are plain maps tagged with a `:kind` atom:

  | Kind    | Purpose                                   |
  |---------|-------------------------------------------|
  | `:rect` | Filled or stroked rectangle               |
  | `:text` | Positioned text label                     |
  | `:line` | Straight line segment between two points  |
  | `:group`| Semantic grouping of child instructions   |
  | `:clip` | Rectangular clip region for children      |

  ## Metadata

  Every instruction carries an optional `metadata` map.  Metadata lets
  producers attach domain meaning (e.g. a barcode character index)
  without polluting the shared scene model.

  ## Renderer behaviour

  Any module that implements `render/1` can serve as a renderer.
  The convenience function `render_with/2` also accepts a map with
  a `:render` key holding a 1-arity function, which is handy for
  one-off or test renderers.
  """

  # ---------------------------------------------------------------------------
  # Types
  # ---------------------------------------------------------------------------

  @typedoc "Metadata values attached to draw instructions."
  @type metadata_value :: String.t() | number() | boolean()

  @typedoc "A key-value map of metadata."
  @type metadata :: %{optional(atom()) => metadata_value()}

  @typedoc """
  A rectangle instruction.

  Rectangles can be filled, stroked, or both.  A filled rectangle with
  no stroke draws a solid block of colour.  A stroked rectangle with
  `fill: \"none\"` draws an outline.  Both together draw a filled box
  with a visible border.
  """
  @type draw_rect :: %{
          kind: :rect,
          x: number(),
          y: number(),
          width: number(),
          height: number(),
          fill: String.t(),
          stroke: String.t() | nil,
          stroke_width: number() | nil,
          metadata: metadata()
        }

  @typedoc """
  A text instruction.

  Font weight is optional and defaults to `\"normal\"`.  Use `\"bold\"`
  to distinguish headers from body text.
  """
  @type draw_text :: %{
          kind: :text,
          x: number(),
          y: number(),
          value: String.t(),
          fill: String.t(),
          font_family: String.t(),
          font_size: number(),
          align: String.t(),
          font_weight: String.t() | nil,
          metadata: metadata()
        }

  @typedoc "A group of child instructions."
  @type draw_group :: %{
          kind: :group,
          children: [draw_instruction()],
          metadata: metadata()
        }

  @typedoc """
  A straight line segment between two points.

  Lines are always stroked (never filled).  Use them for grid lines,
  separators, and borders.
  """
  @type draw_line :: %{
          kind: :line,
          x1: number(),
          y1: number(),
          x2: number(),
          y2: number(),
          stroke: String.t(),
          stroke_width: number(),
          metadata: metadata()
        }

  @typedoc """
  A clipping region that constrains its children.

  Any drawing by children that falls outside the clip rectangle is
  invisible.  This is how the table component prevents cell text from
  bleeding into adjacent columns.
  """
  @type draw_clip :: %{
          kind: :clip,
          x: number(),
          y: number(),
          width: number(),
          height: number(),
          children: [draw_instruction()],
          metadata: metadata()
        }

  @typedoc "Any draw instruction."
  @type draw_instruction :: draw_rect() | draw_text() | draw_group() | draw_line() | draw_clip()

  @typedoc "A complete scene ready for rendering."
  @type draw_scene :: %{
          width: number(),
          height: number(),
          background: String.t(),
          instructions: [draw_instruction()],
          metadata: metadata()
        }

  # ---------------------------------------------------------------------------
  # Renderer behaviour
  # ---------------------------------------------------------------------------

  @doc "Callback that renderers must implement."
  @callback render(draw_scene()) :: any()

  # ---------------------------------------------------------------------------
  # Constructors
  # ---------------------------------------------------------------------------

  @doc """
  Create a rectangle instruction.

  ## Parameters

    - `x`, `y` — top-left corner in scene coordinates
    - `width`, `height` — dimensions
    - `fill` — CSS colour string (default `"#000000"`)
    - `opts` — keyword list of optional fields:
      - `:stroke` — border colour string
      - `:stroke_width` — border thickness (default `1` when stroke is set)
      - `:metadata` — arbitrary key-value map

  ## Examples

      iex> CodingAdventures.DrawInstructions.draw_rect(0, 0, 10, 20)
      %{kind: :rect, x: 0, y: 0, width: 10, height: 20, fill: "#000000",
        stroke: nil, stroke_width: nil, metadata: %{}}

      iex> CodingAdventures.DrawInstructions.draw_rect(5, 5, 50, 30, "#ff0000",
      ...>   stroke: "#000000", stroke_width: 2)
      %{kind: :rect, x: 5, y: 5, width: 50, height: 30, fill: "#ff0000",
        stroke: "#000000", stroke_width: 2, metadata: %{}}
  """
  @spec draw_rect(number(), number(), number(), number(), String.t(), keyword() | map()) ::
          draw_rect()
  def draw_rect(x, y, width, height, fill \\ "#000000", opts \\ [])

  # Backward compatibility: if the 6th argument is a plain map, treat it
  # as metadata directly (the pre-0.2.0 API).
  def draw_rect(x, y, width, height, fill, opts) when is_map(opts) do
    %{
      kind: :rect,
      x: x,
      y: y,
      width: width,
      height: height,
      fill: fill,
      stroke: nil,
      stroke_width: nil,
      metadata: opts
    }
  end

  # New API: keyword list with :stroke, :stroke_width, and :metadata keys.
  def draw_rect(x, y, width, height, fill, opts) when is_list(opts) do
    %{
      kind: :rect,
      x: x,
      y: y,
      width: width,
      height: height,
      fill: fill,
      stroke: Keyword.get(opts, :stroke, nil),
      stroke_width: Keyword.get(opts, :stroke_width, nil),
      metadata: Keyword.get(opts, :metadata, %{})
    }
  end

  @doc """
  Create a text instruction.

  ## Parameters

    - `x`, `y` — anchor position in scene coordinates
    - `value` — the text string to render
    - `opts` — keyword list of optional fields:
      - `:fill` — text colour (default `"#000000"`)
      - `:font_family` — CSS font family (default `"monospace"`)
      - `:font_size` — size in scene units (default `16`)
      - `:align` — text anchor: `"start"`, `"middle"`, or `"end"` (default `"middle"`)
      - `:font_weight` — `"normal"` or `"bold"` (default `nil`)
      - `:metadata` — arbitrary key-value map

  ## Examples

      iex> CodingAdventures.DrawInstructions.draw_text(10, 20, "hello")
      %{kind: :text, x: 10, y: 20, value: "hello", fill: "#000000",
        font_family: "monospace", font_size: 16, align: "middle",
        font_weight: nil, metadata: %{}}
  """
  @spec draw_text(number(), number(), String.t(), keyword() | map()) :: draw_text()
  def draw_text(x, y, value, opts \\ [])

  # Backward compatibility: if the 4th argument is a plain map, treat it
  # as metadata directly (the pre-0.2.0 API).
  def draw_text(x, y, value, opts) when is_map(opts) do
    %{
      kind: :text,
      x: x,
      y: y,
      value: value,
      fill: "#000000",
      font_family: "monospace",
      font_size: 16,
      align: "middle",
      font_weight: nil,
      metadata: opts
    }
  end

  # New API: keyword list with all text options.
  def draw_text(x, y, value, opts) when is_list(opts) do
    %{
      kind: :text,
      x: x,
      y: y,
      value: value,
      fill: Keyword.get(opts, :fill, "#000000"),
      font_family: Keyword.get(opts, :font_family, "monospace"),
      font_size: Keyword.get(opts, :font_size, 16),
      align: Keyword.get(opts, :align, "middle"),
      font_weight: Keyword.get(opts, :font_weight, nil),
      metadata: Keyword.get(opts, :metadata, %{})
    }
  end

  @doc """
  Create a group instruction.

  Groups provide hierarchical structure without introducing transforms.
  They are useful for preserving semantic structure — one group per
  encoded symbol, one group per overlay layer, etc.

  ## Parameters

    - `children` — list of child draw instructions
    - `metadata` — optional key-value map (default `%{}`)
  """
  @spec draw_group([draw_instruction()], metadata()) :: draw_group()
  def draw_group(children, metadata \\ %{}) do
    %{kind: :group, children: children, metadata: metadata}
  end

  @doc """
  Create a line instruction.

  Lines are always stroked (never filled).  They map directly to
  SVG `<line>` elements.

  ## Parameters

    - `x1`, `y1` — start point
    - `x2`, `y2` — end point
    - `stroke` — line colour (default `"#000000"`)
    - `stroke_width` — line thickness (default `1`)
    - `metadata` — optional key-value map (default `%{}`)

  ## Examples

      iex> CodingAdventures.DrawInstructions.draw_line(0, 0, 100, 0)
      %{kind: :line, x1: 0, y1: 0, x2: 100, y2: 0, stroke: "#000000",
        stroke_width: 1, metadata: %{}}
  """
  @spec draw_line(number(), number(), number(), number(), String.t(), number(), metadata()) ::
          draw_line()
  def draw_line(x1, y1, x2, y2, stroke \\ "#000000", stroke_width \\ 1, metadata \\ %{}) do
    %{
      kind: :line,
      x1: x1,
      y1: y1,
      x2: x2,
      y2: y2,
      stroke: stroke,
      stroke_width: stroke_width,
      metadata: metadata
    }
  end

  @doc """
  Create a clip instruction.

  Any drawing by children that falls outside the clip rectangle is
  invisible.  This is how the table component prevents cell text from
  bleeding into adjacent columns — each cell's text is wrapped in a
  clip instruction bounded to the cell's dimensions.

  ## Parameters

    - `x`, `y` — top-left corner of clip rectangle
    - `width`, `height` — dimensions of clip rectangle
    - `children` — list of child draw instructions
    - `metadata` — optional key-value map (default `%{}`)
  """
  @spec draw_clip(number(), number(), number(), number(), [draw_instruction()], metadata()) ::
          draw_clip()
  def draw_clip(x, y, width, height, children, metadata \\ %{}) do
    %{
      kind: :clip,
      x: x,
      y: y,
      width: width,
      height: height,
      children: children,
      metadata: metadata
    }
  end

  @doc """
  Create a complete scene.

  A scene is the unit renderers consume.  Width and height are explicit
  because renderers should not have to infer output bounds from the
  instructions.

  ## Parameters

    - `width` — scene width in logical units
    - `height` — scene height in logical units
    - `instructions` — list of draw instructions
    - `background` — background colour (default `"#ffffff"`)
    - `metadata` — optional key-value map (default `%{}`)
  """
  @spec create_scene(number(), number(), [draw_instruction()], String.t(), metadata()) ::
          draw_scene()
  def create_scene(width, height, instructions, background \\ "#ffffff", metadata \\ %{}) do
    %{
      width: width,
      height: height,
      background: background,
      instructions: instructions,
      metadata: metadata
    }
  end

  @doc """
  Delegate rendering to a backend implementation.

  Accepts either:

    - A **module** that implements `render/1`
    - A **map** with a `:render` key holding a 1-arity function

  ## Examples

      iex> scene = CodingAdventures.DrawInstructions.create_scene(10, 20, [])
      iex> renderer = %{render: fn s -> "\#{s.width}x\#{s.height}" end}
      iex> CodingAdventures.DrawInstructions.render_with(scene, renderer)
      "10x20"
  """
  @spec render_with(draw_scene(), atom() | map()) :: any()
  def render_with(scene, renderer) do
    cond do
      is_atom(renderer) ->
        renderer.render(scene)

      is_map(renderer) ->
        render_fn = Map.get(renderer, :render)

        if is_function(render_fn, 1) do
          render_fn.(scene)
        else
          raise ArgumentError, "renderer must be a module or map with a render/1 function"
        end

      true ->
        raise ArgumentError, "renderer must be a module or map with a render/1 function"
    end
  end
end
