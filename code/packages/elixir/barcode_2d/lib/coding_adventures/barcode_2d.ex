defmodule CodingAdventures.Barcode2D do
  @moduledoc """
  Shared 2D barcode abstraction layer.

  This module provides the two building blocks every 2D barcode format needs:

    1. `ModuleGrid` — the universal intermediate representation produced by
       every 2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode).
       It is just a 2D boolean grid: `true` = dark module, `false` = light module.

    2. `layout/2` — the single function that converts abstract module
       coordinates into pixel-level `PaintScene` instructions ready for the
       PaintVM to render.

  ## Where this fits in the pipeline

  ```
  Input data
    → format encoder (qr-code, data-matrix, aztec…)
    → ModuleGrid          ← produced by the encoder
    → layout/2            ← THIS MODULE converts to pixels
    → PaintScene          ← consumed by paint-vm (P2D01)
    → backend (SVG, Metal, Canvas, terminal…)
  ```

  All coordinates before `layout/2` are measured in "module units" — abstract
  grid steps. Only `layout/2` multiplies by `module_size_px` to produce real
  pixel coordinates. This means encoders never need to know anything about
  screen resolution or output format.

  ## Supported module shapes

  - **square** (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
    Each module becomes a `paint_rect`.

  - **hex** (flat-top hexagons): used by MaxiCode. Each module becomes a
    `paint_path` tracing six vertices.

  ## Immutability

  `ModuleGrid` is a struct with a list-of-lists for modules. `set_module/4`
  returns a new grid using `List.replace_at/3` without mutating the original.
  This makes encoders easy to test, compose, and backtrack.

  ## Elixir reserved word note

  Elixir reserves words like `after`, `rescue`, `catch`, `else`, `end`.
  These cannot be used as variable names. In this module we avoid them
  entirely — for example "rest" instead of "after".
  """

  alias CodingAdventures.PaintInstructions

  @version "0.1.0"
  def version, do: @version

  # ============================================================================
  # ModuleGrid struct
  # ============================================================================

  defmodule ModuleGrid do
    @moduledoc """
    The universal intermediate representation produced by every 2D barcode
    encoder.

    Fields:

    - `rows` — number of rows (height of the grid).
    - `cols` — number of columns (width of the grid).
    - `modules` — list of `rows` lists, each containing `cols` booleans.
      `modules[row][col]` is `true` for a dark module, `false` for light.
      Row 0 is the top row; column 0 is the leftmost column.
    - `module_shape` — `:square` or `:hex`.

    ## Example

    A 3×3 grid with all modules light:

    ```elixir
    %ModuleGrid{
      rows: 3,
      cols: 3,
      modules: [[false, false, false],
                [false, false, false],
                [false, false, false]],
      module_shape: :square
    }
    ```
    """

    defstruct [:cols, :rows, :modules, :module_shape]

    @type module_shape :: :square | :hex
    @type t :: %__MODULE__{
            cols: non_neg_integer(),
            rows: non_neg_integer(),
            modules: [[boolean()]],
            module_shape: module_shape()
          }
  end

  # ============================================================================
  # Barcode2DLayoutConfig struct
  # ============================================================================

  defmodule Barcode2DLayoutConfig do
    @moduledoc """
    Configuration for `CodingAdventures.Barcode2D.layout/2`.

    All fields have defaults so you can start with `%Barcode2DLayoutConfig{}`
    and override only what you need.

    | Field                | Default     | Notes                                      |
    |----------------------|-------------|---------------------------------------------|
    | `module_size_px`     | `10.0`      | Must be > 0. Width of one module in pixels. |
    | `quiet_zone_modules` | `4`         | Quiet-zone width in module units (≥ 0).     |
    | `foreground`         | `"#000000"` | Color for dark modules.                     |
    | `background`         | `"#ffffff"` | Color for light modules and quiet zone.     |
    | `show_annotations`   | `false`     | Reserved for future annotated rendering.    |
    | `module_shape`       | `:square`   | Must match `ModuleGrid.module_shape`.        |

    ## moduleSizePx / module_size_px

    The size of one module in pixels. For square modules this is both width
    and height. For hex modules it is the flat-to-flat hexagon width (also
    equal to the side length for a regular hexagon).

    ## quietZoneModules / quiet_zone_modules

    Number of module-width units added as a quiet zone on **each** side of the
    grid. QR Code requires a minimum of 4. Data Matrix requires 1.
    """

    defstruct module_size_px: 10.0,
              quiet_zone_modules: 4,
              foreground: "#000000",
              background: "#ffffff",
              show_annotations: false,
              module_shape: :square

    @type module_shape :: :square | :hex
    @type t :: %__MODULE__{
            module_size_px: float(),
            quiet_zone_modules: non_neg_integer(),
            foreground: String.t(),
            background: String.t(),
            show_annotations: boolean(),
            module_shape: module_shape()
          }
  end

  # ============================================================================
  # make_module_grid/3 — create an all-light grid
  # ============================================================================

  @doc """
  Create a new `ModuleGrid` with every module set to `false` (light).

  This is the starting point for every 2D barcode encoder. The encoder calls
  `make_module_grid(rows, cols)` and then uses `set_module/4` to paint dark
  modules one by one as it places finder patterns, timing strips, data bits,
  and error correction bits.

  ## Parameters

  - `rows` — number of rows (height). Must be a positive integer.
  - `cols` — number of columns (width). Must be a positive integer.
  - `module_shape` — `:square` (default) or `:hex`.

  ## Example

  Start a 21×21 QR Code v1 grid:

  ```elixir
  grid = CodingAdventures.Barcode2D.make_module_grid(21, 21)
  # grid.rows == 21
  # grid.cols == 21
  # grid.modules |> List.first() |> List.first() == false  (all light)
  ```
  """
  @spec make_module_grid(pos_integer(), pos_integer(), ModuleGrid.module_shape()) ::
          ModuleGrid.t()
  def make_module_grid(rows, cols, module_shape \\ :square) do
    # Build rows*cols grid of false values.
    # Each row is an independent list so set_module/4 can replace individual
    # rows using List.replace_at/3 without copying the entire grid.
    row_template = List.duplicate(false, cols)
    modules = List.duplicate(row_template, rows)

    %ModuleGrid{
      rows: rows,
      cols: cols,
      modules: modules,
      module_shape: module_shape
    }
  end

  # ============================================================================
  # set_module/4 — immutable single-module update
  # ============================================================================

  @doc """
  Return a new `ModuleGrid` identical to `grid` except that module at
  `(row, col)` is set to `dark`.

  This function is **pure and immutable** — it never modifies the input grid.
  Only the affected row is re-allocated via `List.replace_at/3`; all other
  rows are shared between old and new grids.

  Returns `{:ok, new_grid}` on success, or `{:error, reason}` if the
  coordinates are out of bounds.

  ## Why immutability matters

  Barcode encoders often need to backtrack — e.g. trying all eight QR mask
  patterns and keeping the best score. Immutable grids make this trivial:
  save the grid before trying a mask, evaluate it, discard if the score is
  worse. No undo stack needed.

  ## Example

  ```elixir
  grid = CodingAdventures.Barcode2D.make_module_grid(3, 3)
  {:ok, grid2} = CodingAdventures.Barcode2D.set_module(grid, 1, 1, true)
  # grid.modules |> Enum.at(1) |> Enum.at(1) == false   (original unchanged)
  # grid2.modules |> Enum.at(1) |> Enum.at(1) == true
  ```
  """
  @spec set_module(ModuleGrid.t(), non_neg_integer(), non_neg_integer(), boolean()) ::
          {:ok, ModuleGrid.t()} | {:error, String.t()}
  def set_module(%ModuleGrid{} = grid, row, col, dark) do
    cond do
      row < 0 or row >= grid.rows ->
        {:error,
         "set_module: row #{row} out of range [0, #{grid.rows - 1}]"}

      col < 0 or col >= grid.cols ->
        {:error,
         "set_module: col #{col} out of range [0, #{grid.cols - 1}]"}

      true ->
        # Replace only the affected row; Enum.at and List.replace_at work on
        # Elixir lists in O(n) which is fine for typical barcode grid sizes.
        old_row = Enum.at(grid.modules, row)
        new_row = List.replace_at(old_row, col, dark)
        new_modules = List.replace_at(grid.modules, row, new_row)
        {:ok, %ModuleGrid{grid | modules: new_modules}}
    end
  end

  # ============================================================================
  # layout/2 — ModuleGrid → PaintScene
  # ============================================================================

  @doc """
  Convert a `ModuleGrid` into a `PaintScene` ready for the PaintVM.

  This is the **only** function in the entire 2D barcode stack that knows
  about pixels. Everything above this step works in abstract module units.
  Everything below this step is handled by the paint backend.

  ## Square modules (the common case)

  Each dark module at `(row, col)` becomes one `paint_rect`:

  ```
  quiet_zone_px = quiet_zone_modules * module_size_px
  x = quiet_zone_px + col * module_size_px
  y = quiet_zone_px + row * module_size_px
  ```

  Total symbol size (including quiet zone on all four sides):

  ```
  total_width  = (cols + 2 * quiet_zone_modules) * module_size_px
  total_height = (rows + 2 * quiet_zone_modules) * module_size_px
  ```

  The scene always starts with one background `paint_rect` covering the full
  symbol, ensuring the quiet zone and light modules are filled with the
  background colour even if the backend has a transparent default.

  ## Hex modules (MaxiCode)

  Each dark module at `(row, col)` becomes one `paint_path` tracing a
  flat-top regular hexagon. Odd-numbered rows are offset right by half a
  hexagon width to produce the standard hexagonal tiling:

  ```
  Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡
  Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡
  Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
  ```

  Geometry for a flat-top hexagon with circumradius R (center to vertex):

  ```
  hex_width  = module_size_px
  hex_height = module_size_px * (sqrt(3) / 2)
  circum_r   = module_size_px / sqrt(3)

  Vertex i:  ( cx + circum_r * cos(i * 60°),
               cy + circum_r * sin(i * 60°) )
  ```

  Center coordinates including quiet zone offset:

  ```
  cx = quiet_zone_px + col * hex_width + (row rem 2) * (hex_width / 2)
  cy = quiet_zone_px + row * hex_height
  ```

  ## Validation

  Returns `{:error, reason}` if:
  - `module_size_px <= 0`
  - `quiet_zone_modules < 0`
  - `config.module_shape != grid.module_shape`

  ## Example

  ```elixir
  grid = CodingAdventures.Barcode2D.make_module_grid(5, 5)
  {:ok, grid} = CodingAdventures.Barcode2D.set_module(grid, 2, 2, true)
  {:ok, scene} = CodingAdventures.Barcode2D.layout(grid)
  scene.width  # == 130.0  (5 + 2*4) * 10
  scene.height # == 130.0
  ```
  """
  @spec layout(ModuleGrid.t(), Barcode2DLayoutConfig.t()) ::
          {:ok, PaintInstructions.paint_scene()} | {:error, String.t()}
  def layout(%ModuleGrid{} = grid, %Barcode2DLayoutConfig{} = config \\ %Barcode2DLayoutConfig{}) do
    with :ok <- validate_config(config, grid) do
      if config.module_shape == :square do
        {:ok, layout_square(grid, config)}
      else
        {:ok, layout_hex(grid, config)}
      end
    end
  end

  # ============================================================================
  # validate_config/2 — internal config validation
  # ============================================================================

  # Validates the layout config against the grid. Returns :ok or {:error, msg}.
  #
  # Three things can go wrong:
  #   1. module_size_px <= 0  — nonsensical pixel size
  #   2. quiet_zone_modules < 0  — negative quiet zone
  #   3. config.module_shape != grid.module_shape  — shape mismatch
  defp validate_config(%Barcode2DLayoutConfig{} = config, %ModuleGrid{} = grid) do
    cond do
      config.module_size_px <= 0 ->
        {:error, "module_size_px must be > 0, got #{config.module_size_px}"}

      config.quiet_zone_modules < 0 ->
        {:error, "quiet_zone_modules must be >= 0, got #{config.quiet_zone_modules}"}

      config.module_shape != grid.module_shape ->
        {:error,
         "config.module_shape :#{config.module_shape} does not match " <>
           "grid.module_shape :#{grid.module_shape}"}

      true ->
        :ok
    end
  end

  # ============================================================================
  # layout_square/2 — internal helper for square-module grids
  # ============================================================================

  # Render a square-module ModuleGrid into a PaintScene.
  #
  # Algorithm:
  #   1. Compute total pixel dimensions including quiet zone.
  #   2. Emit one background paint_rect covering the entire symbol.
  #   3. For each dark module, emit one filled paint_rect.
  #
  # Light modules are covered by the background rect — no explicit light rects
  # are emitted. This keeps the instruction count proportional to dark modules.
  defp layout_square(%ModuleGrid{} = grid, %Barcode2DLayoutConfig{} = config) do
    %{
      module_size_px: module_size_px,
      quiet_zone_modules: quiet_zone_modules,
      foreground: foreground,
      background: background
    } = config

    # Quiet zone in pixels on each side.
    quiet_zone_px = quiet_zone_modules * module_size_px

    # Total canvas dimensions including quiet zone on all four sides.
    total_width = (grid.cols + 2 * quiet_zone_modules) * module_size_px
    total_height = (grid.rows + 2 * quiet_zone_modules) * module_size_px

    # Start with the background rect covering the whole symbol (quiet zone
    # plus all modules). This ensures the background is filled even when the
    # rendering backend defaults to transparent.
    bg_rect = PaintInstructions.paint_rect(0, 0, total_width, total_height, background)

    # Collect one paint_rect per dark module using Enum.with_index for indexed
    # iteration (we need both the row index and the column index).
    dark_rects =
      grid.modules
      |> Enum.with_index()
      |> Enum.flat_map(fn {row_data, row_idx} ->
        row_data
        |> Enum.with_index()
        |> Enum.filter(fn {dark, _col_idx} -> dark end)
        |> Enum.map(fn {_dark, col_idx} ->
          # Pixel origin of this module (top-left corner of its square).
          x = quiet_zone_px + col_idx * module_size_px
          y = quiet_zone_px + row_idx * module_size_px
          PaintInstructions.paint_rect(x, y, module_size_px, module_size_px, foreground)
        end)
      end)

    instructions = [bg_rect | dark_rects]

    PaintInstructions.paint_scene(
      total_width,
      total_height,
      instructions,
      background
    )
  end

  # ============================================================================
  # layout_hex/2 — internal helper for hex-module grids (MaxiCode)
  # ============================================================================

  # Render a hex-module ModuleGrid into a PaintScene.
  #
  # Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
  # offset-row grid. Odd rows are shifted right by half a hexagon width.
  #
  # Flat-top hexagon geometry reminder:
  #
  #   A "flat-top" hexagon has two flat edges at the top and bottom:
  #
  #      ___
  #     /   \      ← two vertices at top
  #    |     |
  #     \___/      ← two vertices at bottom
  #
  # For a flat-top hexagon centered at (cx, cy) with circumradius R:
  #
  #   Vertices at angles 0°, 60°, 120°, 180°, 240°, 300°:
  #
  #     angle  cos    sin    role
  #       0°    1      0     right midpoint
  #      60°   0.5   √3/2   bottom-right
  #     120°  -0.5   √3/2   bottom-left
  #     180°  -1      0     left midpoint
  #     240°  -0.5  -√3/2   top-left
  #     300°   0.5  -√3/2   top-right
  #
  # Tiling parameters:
  #   hex_width  = module_size_px        (flat-to-flat = side length)
  #   hex_height = module_size_px * √3/2  (vertical row step)
  #   circum_r   = module_size_px / √3   (center-to-vertex distance)
  defp layout_hex(%ModuleGrid{} = grid, %Barcode2DLayoutConfig{} = config) do
    %{
      module_size_px: module_size_px,
      quiet_zone_modules: quiet_zone_modules,
      foreground: foreground,
      background: background
    } = config

    # Hex geometry constants.
    hex_width = module_size_px
    hex_height = module_size_px * (:math.sqrt(3) / 2.0)
    circum_r = module_size_px / :math.sqrt(3)

    quiet_zone_px = quiet_zone_modules * module_size_px

    # Total canvas size. The +hex_width/2 accounts for the odd-row offset so
    # the rightmost modules on odd rows don't clip outside the canvas.
    total_width = (grid.cols + 2 * quiet_zone_modules) * hex_width + hex_width / 2
    total_height = (grid.rows + 2 * quiet_zone_modules) * hex_height

    # Background rect.
    bg_rect = PaintInstructions.paint_rect(0, 0, total_width, total_height, background)

    # One paint_path per dark module.
    dark_paths =
      grid.modules
      |> Enum.with_index()
      |> Enum.flat_map(fn {row_data, row_idx} ->
        row_data
        |> Enum.with_index()
        |> Enum.filter(fn {dark, _col_idx} -> dark end)
        |> Enum.map(fn {_dark, col_idx} ->
          # Center of this hexagon in pixel space.
          # Odd rows shift right by hex_width/2 to interlock with even rows.
          cx = quiet_zone_px + col_idx * hex_width + rem(row_idx, 2) * (hex_width / 2)
          cy = quiet_zone_px + row_idx * hex_height

          commands = build_flat_top_hex_path(cx, cy, circum_r)
          paint_path(commands, foreground)
        end)
      end)

    instructions = [bg_rect | dark_paths]

    PaintInstructions.paint_scene(
      total_width,
      total_height,
      instructions,
      background
    )
  end

  # ============================================================================
  # build_flat_top_hex_path/3 — geometry helper
  # ============================================================================

  # Build the six path commands for a flat-top regular hexagon.
  #
  # The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300°
  # from center (cx, cy) at circumradius circum_r:
  #
  #   vertex_i = ( cx + circum_r * cos(i * 60°),
  #                cy + circum_r * sin(i * 60°) )
  #
  # The path is:
  #   move_to  vertex 0
  #   line_to  vertex 1
  #   line_to  vertex 2
  #   line_to  vertex 3
  #   line_to  vertex 4
  #   line_to  vertex 5
  #   close
  defp build_flat_top_hex_path(cx, cy, circum_r) do
    deg_to_rad = :math.pi() / 180.0

    # Compute all 6 vertices.
    vertices =
      Enum.map(0..5, fn i ->
        angle = i * 60.0 * deg_to_rad
        %{
          x: cx + circum_r * :math.cos(angle),
          y: cy + circum_r * :math.sin(angle)
        }
      end)

    # Build path commands: move_to vertex 0, then line_to vertices 1-5, close.
    [first_vertex | rest_vertices] = vertices

    move_cmd = %{kind: :move_to, x: first_vertex.x, y: first_vertex.y}

    line_cmds =
      Enum.map(rest_vertices, fn v ->
        %{kind: :line_to, x: v.x, y: v.y}
      end)

    close_cmd = %{kind: :close}

    [move_cmd | line_cmds] ++ [close_cmd]
  end

  # ============================================================================
  # paint_path/2 — local path instruction builder
  # ============================================================================

  # Build a paint_path instruction map.
  #
  # The Elixir paint_instructions package currently only exports paint_rect
  # and paint_scene. Hex rendering needs a path instruction that carries a
  # list of PathCommand maps. We represent it as a plain map with kind: :path,
  # matching the convention used by the TypeScript paint-instructions package.
  defp paint_path(commands, fill) do
    %{
      kind: :path,
      commands: commands,
      fill: fill,
      metadata: %{}
    }
  end
end
