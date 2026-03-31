defmodule CodingAdventures.DrawInstructionsSvg do
  @moduledoc """
  SVG renderer for backend-neutral draw instructions.

  This module is intentionally boring in the best possible way.  It knows
  how to serialize a generic draw scene to SVG, and nothing more.  It does
  not contain barcode rules, table rules, or any other producer domain
  logic.  That separation is the whole reason this package exists.

  ## Supported instructions

  | Kind    | SVG element                              |
  |---------|------------------------------------------|
  | `:rect` | `<rect>` with optional stroke attributes |
  | `:text` | `<text>` with optional font-weight       |
  | `:line` | `<line>` with stroke attributes           |
  | `:group`| `<g>` wrapping children recursively      |
  | `:clip` | `<clipPath>` + `<g clip-path="...">`     |

  ## Metadata

  Metadata keys are serialized as `data-*` attributes on the SVG element.
  This is a nice compromise: SVG stays valid, semantic information survives
  into the output, and browser tooling can inspect the metadata later.

  ## Clip IDs

  SVG clipping requires unique `id` attributes.  This renderer uses the
  process dictionary to maintain a counter that resets at the start of
  each `render/1` call, producing deterministic output (`clip-1`, `clip-2`,
  etc.) for the same scene.
  """

  @behaviour CodingAdventures.DrawInstructions

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Render a draw scene to an SVG string.

  The output is a complete `<svg>` document with:
  - An `xmlns` declaration for standalone use
  - A `viewBox` matching the scene dimensions
  - An accessibility `role="img"` and `aria-label`
  - A background rectangle covering the full scene
  - All instructions serialized in document order
  """
  @impl true
  @spec render(CodingAdventures.DrawInstructions.draw_scene()) :: String.t()
  def render(scene) do
    # Reset clip counter for deterministic output across renders.
    Process.put(:draw_svg_clip_counter, 0)

    label =
      scene.metadata
      |> Map.get(:label, "draw instructions scene")
      |> to_string()
      |> xml_escape()

    instructions = Enum.map(scene.instructions, &render_instruction/1)

    [
      ~s(<svg xmlns="http://www.w3.org/2000/svg" width="#{scene.width}" height="#{scene.height}" viewBox="0 0 #{scene.width} #{scene.height}" role="img" aria-label="#{label}">),
      ~s(  <rect x="0" y="0" width="#{scene.width}" height="#{scene.height}" fill="#{xml_escape(scene.background)}" />),
      instructions,
      "</svg>"
    ]
    |> List.flatten()
    |> Enum.join("\n")
  end

  # ---------------------------------------------------------------------------
  # Instruction renderers
  # ---------------------------------------------------------------------------

  # Rectangle — the workhorse primitive.  Supports optional stroke for
  # borders and focus rings.
  defp render_instruction(%{kind: :rect} = inst) do
    stroke_attrs = render_stroke_attrs(inst)

    ~s(  <rect x="#{inst.x}" y="#{inst.y}" width="#{inst.width}" height="#{inst.height}" fill="#{xml_escape(inst.fill)}"#{stroke_attrs}#{metadata_to_attributes(inst.metadata)} />)
  end

  # Text — positioned label with optional bold weight.
  #
  # The font-weight attribute is only emitted when it differs from the
  # default ("normal"), keeping the SVG output compact.
  defp render_instruction(%{kind: :text} = inst) do
    weight_attr =
      if inst[:font_weight] != nil and inst[:font_weight] != "normal" do
        ~s( font-weight="#{inst.font_weight}")
      else
        ""
      end

    ~s(  <text x="#{inst.x}" y="#{inst.y}" text-anchor="#{inst.align}" font-family="#{xml_escape(inst.font_family)}" font-size="#{inst.font_size}" fill="#{xml_escape(inst.fill)}"#{weight_attr}#{metadata_to_attributes(inst.metadata)}>#{xml_escape(inst.value)}</text>)
  end

  # Line — straight segment between two points.  Lines are always stroked.
  # The SVG `<line>` element uses x1/y1/x2/y2 attributes — a direct 1:1
  # mapping from our draw_line fields.
  defp render_instruction(%{kind: :line} = inst) do
    ~s(  <line x1="#{inst.x1}" y1="#{inst.y1}" x2="#{inst.x2}" y2="#{inst.y2}" stroke="#{xml_escape(inst.stroke)}" stroke-width="#{inst.stroke_width}"#{metadata_to_attributes(inst.metadata)} />)
  end

  # Group — recursive container.  Emits an SVG `<g>` wrapping all children.
  defp render_instruction(%{kind: :group} = inst) do
    children = Enum.map_join(inst.children, "\n", &render_instruction/1)
    "  <g#{metadata_to_attributes(inst.metadata)}>\n#{children}\n  </g>"
  end

  # Clip — rectangular clipping region.
  #
  # SVG clipping uses a `<clipPath>` element containing a `<rect>` that
  # defines the clip region, referenced by `clip-path="url(#id)"` on a
  # `<g>` that wraps the clipped children.
  #
  # We generate unique IDs using a process-dictionary counter to avoid
  # collisions while keeping output deterministic.
  defp render_instruction(%{kind: :clip} = inst) do
    counter = Process.get(:draw_svg_clip_counter, 0) + 1
    Process.put(:draw_svg_clip_counter, counter)
    id = "clip-#{counter}"

    children = Enum.map_join(inst.children, "\n", &render_instruction/1)

    [
      "  <defs>",
      ~s(    <clipPath id="#{id}">),
      ~s(      <rect x="#{inst.x}" y="#{inst.y}" width="#{inst.width}" height="#{inst.height}" />),
      "    </clipPath>",
      "  </defs>",
      ~s[  <g clip-path="url(##{id})"#{metadata_to_attributes(inst.metadata)}>],
      children,
      "  </g>"
    ]
    |> Enum.join("\n")
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Build stroke-related SVG attributes for a rect instruction.
  # Only emits attributes when a stroke colour is present.
  defp render_stroke_attrs(%{stroke: nil}), do: ""

  defp render_stroke_attrs(%{stroke: stroke} = inst) do
    width = inst[:stroke_width] || 1
    ~s( stroke="#{xml_escape(stroke)}" stroke-width="#{width}")
  end

  # Fallback for maps without a :stroke key (backward compatibility).
  defp render_stroke_attrs(_), do: ""

  # Serialize metadata as `data-*` attributes.
  #
  # Keys are converted to strings and prefixed with `data-`.  Values are
  # XML-escaped.  This keeps the SVG valid while preserving semantic
  # information for downstream tooling.
  defp metadata_to_attributes(nil), do: ""
  defp metadata_to_attributes(metadata) when map_size(metadata) == 0, do: ""

  defp metadata_to_attributes(metadata) do
    Enum.map_join(metadata, "", fn {key, value} ->
      ~s( data-#{key}="#{xml_escape(to_string(value))}")
    end)
  end

  # Escape user-provided text before embedding it into XML.
  #
  # The five characters that must be escaped in XML attribute values and
  # text content are: & < > " '
  defp xml_escape(value) do
    value
    |> String.replace("&", "&amp;")
    |> String.replace("<", "&lt;")
    |> String.replace(">", "&gt;")
    |> String.replace("\"", "&quot;")
    |> String.replace("'", "&apos;")
  end
end
