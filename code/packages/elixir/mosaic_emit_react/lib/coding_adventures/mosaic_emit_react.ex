defmodule CodingAdventures.MosaicEmitReact do
  @moduledoc """
  React backend for the Mosaic compiler — emits TypeScript React functional components (.tsx).

  This module implements the `CodingAdventures.MosaicVM.Renderer` behaviour and is
  driven by `CodingAdventures.MosaicVM`. Every time the VM traverses a Mosaic IR tree,
  it calls callbacks on this renderer; the renderer accumulates JSX strings and
  finalizes them when `end_component/1` is called.

  ## Architecture: String Stack

  The renderer maintains a **stack of string buffers**, one per open node.
  When `begin_node` is called, a new frame is pushed. When `end_node` is called,
  the frame is popped, wrapped in a JSX element string, and appended to the
  parent frame's children. This handles arbitrary nesting without lookahead.

      begin_component("Card")       → stack: [component-frame]
      begin_node("Column")          → stack: [component-frame, column-frame]
      begin_node("Text")            → stack: [component-frame, column-frame, text-frame]
      end_node()                    → pop text-frame → "<span>…</span>"
                                       append to column-frame
      end_node()                    → pop column-frame → "<div>…</div>"
                                       append to component-frame
      end_component()               → no-op; component-frame holds root JSX

  ## Output File Structure

  The generated .tsx file:

  1. File header comment (auto-generated warning)
  2. `import React from "react";`
  3. Optional: `import "./mosaic-type-scale.css";` if any Text uses `style:`
  4. Optional: `import type { TProps } from "./T.js";` for component-type slots
  5. `interface ComponentNameProps { … }` — TypeScript props interface
  6. `export function ComponentName({ … }: ComponentNameProps): JSX.Element { … }`

  ## Primitive Node → JSX Element Mapping

  | Mosaic  | JSX element | Base styles                                |
  |---------|-------------|--------------------------------------------|
  | Box     | div         | position: "relative"                       |
  | Column  | div         | display: "flex", flexDirection: "column"   |
  | Row     | div         | display: "flex", flexDirection: "row"      |
  | Text    | span        | (may become h2 if a11y-role: heading)      |
  | Image   | img         | self-closing                               |
  | Spacer  | div         | flex: 1                                    |
  | Scroll  | div         | overflow: "auto"                           |
  | Divider | hr          | self-closing, border styles                |

  ## Usage

      {:ok, result} = CodingAdventures.MosaicVM.run(ir, __MODULE__, initial_state())
      # result.files[0].filename == "MyComponent.tsx"
  """

  @behaviour CodingAdventures.MosaicVM.Renderer

  # ---------------------------------------------------------------------------
  # Initial state
  # ---------------------------------------------------------------------------

  @doc """
  Returns the blank initial state for the renderer.

  Pass this as the third argument to `MosaicVM.run/3`.
  """
  def initial_state do
    %{
      component_name: "",
      slots: [],
      stack: [],
      # Component-type names that appear in slot types — need `import type { TProps }`
      slot_component_imports: MapSet.new(),
      # Non-primitive node tags encountered in the tree — need `import { T }`
      node_component_imports: MapSet.new(),
      # Whether any Text used the `style:` property (needs type-scale CSS import)
      needs_type_scale_css: false
    }
  end

  # ---------------------------------------------------------------------------
  # MosaicVM.Renderer callbacks
  # ---------------------------------------------------------------------------

  @impl true
  def begin_component(state, name, slots) do
    # Pre-scan slots for component-type imports so we know the full import list
    # before traversal begins.
    slot_component_imports =
      Enum.reduce(slots, MapSet.new(), fn slot, acc ->
        case slot.type do
          %{kind: :component, name: comp_name} -> MapSet.put(acc, comp_name)
          %{kind: :list, element_type: %{kind: :component, name: comp_name}} -> MapSet.put(acc, comp_name)
          _ -> acc
        end
      end)

    %{state |
      component_name: name,
      slots: slots,
      stack: [%{kind: :component, lines: []}],
      slot_component_imports: slot_component_imports,
      node_component_imports: MapSet.new(),
      needs_type_scale_css: false
    }
  end

  @impl true
  def end_component(state) do
    content = build_file(state)
    {:ok, %{files: [%{filename: "#{state.component_name}.tsx", content: content}]}}
  end

  @impl true
  def begin_node(state, node_type, is_primitive, props) do
    {frame, updated_state} = build_node_frame(node_type, is_primitive, props, state)
    %{updated_state | stack: updated_state.stack ++ [frame]}
  end

  @impl true
  def end_node(state) do
    {stack_init, [frame]} = Enum.split(state.stack, length(state.stack) - 1)
    jsx = build_jsx_element(frame)
    state1 = %{state | stack: stack_init}
    append_to_parent(state1, jsx)
  end

  @impl true
  def render_slot_child(state, slot_name, _slot_type) do
    # Slot refs used as children render as the destructured prop variable.
    append_to_parent(state, "{#{slot_name}}")
  end

  @impl true
  def begin_when(state, slot_name) do
    frame = %{kind: :when, slot_name: slot_name, lines: []}
    %{state | stack: state.stack ++ [frame]}
  end

  @impl true
  def end_when(state) do
    {stack_init, [frame]} = Enum.split(state.stack, length(state.stack) - 1)
    children = frame.lines

    # Wrap in a React conditional expression: {flag && (<child />)}
    body =
      case children do
        [single] -> single
        many ->
          inner = Enum.map_join(many, "\n", fn l -> "  " <> l end)
          "<>\n#{inner}\n</>"
      end

    indented_body = body |> String.split("\n") |> Enum.map_join("\n", fn l -> "  " <> l end)
    jsx = "{#{frame.slot_name} && (\n#{indented_body}\n)}"

    state1 = %{state | stack: stack_init}
    append_to_parent(state1, jsx)
  end

  @impl true
  def begin_each(state, slot_name, item_name) do
    frame = %{kind: :each, slot_name: slot_name, item_name: item_name, lines: []}
    %{state | stack: state.stack ++ [frame]}
  end

  @impl true
  def end_each(state) do
    {stack_init, [frame]} = Enum.split(state.stack, length(state.stack) - 1)
    body_lines = frame.lines

    # Wrap body in React.Fragment map expression with key={_index}.
    indented_body = Enum.map_join(body_lines, "\n", fn l -> "    " <> l end)

    jsx =
      "{#{frame.slot_name}.map((#{frame.item_name}, _index) => (\n" <>
      "  <React.Fragment key={_index}>\n" <>
      "#{indented_body}\n" <>
      "  </React.Fragment>\n" <>
      "))}"

    state1 = %{state | stack: stack_init}
    append_to_parent(state1, jsx)
  end

  # ---------------------------------------------------------------------------
  # Node Frame Building (private)
  # ---------------------------------------------------------------------------

  defp build_node_frame(tag, is_primitive, props, state) do
    # Step 1: determine JSX tag and base styles from the primitive element type.
    {jsx_tag, base_styles, self_closing} = primitive_defaults(tag, is_primitive)

    state_with_import =
      if is_primitive do
        state
      else
        %{state | node_component_imports: MapSet.put(state.node_component_imports, tag)}
      end

    # Start a frame accumulator.
    frame = %{
      kind: :node,
      tag: tag,
      jsx_tag: jsx_tag,
      styles: base_styles,
      attrs: [],
      class_names: [],
      text_content: nil,
      self_closing: self_closing,
      lines: []
    }

    # Step 2: Apply each property to the frame.
    {frame2, state2} =
      Enum.reduce(props, {frame, state_with_import}, fn prop, {fr, st} ->
        apply_property(prop, tag, fr, st)
      end)

    # Step 3: Post-process — a11y-role: heading on Text → <h2>.
    frame3 =
      if tag == "Text" && "role=\"heading\"" in frame2.attrs do
        %{frame2 |
          jsx_tag: "h2",
          attrs: List.delete(frame2.attrs, "role=\"heading\"")
        }
      else
        frame2
      end

    # Store the (possibly updated) state back in the frame so end_node can
    # retrieve imports/needs_type_scale_css. We embed state updates in the
    # outer state; the frame just holds visual data.
    # Actually in Elixir we handle state separately. Let's thread the state
    # through the outer map and just return the frame + the updated state.
    # We store the updated state on the call side. Hmm — we need to propagate
    # state.needs_type_scale_css and node_component_imports. Let's return a
    # {frame, updated_outer_state} tuple.
    {frame3, state2}
  end

  # Returns {jsx_tag, base_styles_map, self_closing}
  defp primitive_defaults(tag, true) do
    case tag do
      "Box"     -> {"div", %{"position" => ~s("relative")}, false}
      "Column"  -> {"div", %{"display" => ~s("flex"), "flexDirection" => ~s("column")}, false}
      "Row"     -> {"div", %{"display" => ~s("flex"), "flexDirection" => ~s("row")}, false}
      "Text"    -> {"span", %{}, false}
      "Image"   -> {"img", %{}, true}
      "Spacer"  -> {"div", %{"flex" => "1"}, false}
      "Scroll"  -> {"div", %{"overflow" => ~s("auto")}, false}
      "Divider" -> {"hr", %{"border" => ~s("none"), "borderTop" => ~s("1px solid currentColor")}, true}
      _         -> {"div", %{}, false}
    end
  end

  defp primitive_defaults(tag, false), do: {tag, %{}, false}

  # ---------------------------------------------------------------------------
  # Property Application (private)
  # ---------------------------------------------------------------------------

  # Returns {updated_frame, updated_state}
  defp apply_property(%{name: name, value: value}, tag, frame, state) do
    case name do
      # -----------------------------------------------------------------------
      # Layout: Spacing
      # -----------------------------------------------------------------------

      "padding" ->
        {put_style_dim(frame, "padding", value), state}

      "padding-left" ->
        {put_style_dim(frame, "paddingLeft", value), state}

      "padding-right" ->
        {put_style_dim(frame, "paddingRight", value), state}

      "padding-top" ->
        {put_style_dim(frame, "paddingTop", value), state}

      "padding-bottom" ->
        {put_style_dim(frame, "paddingBottom", value), state}

      "gap" ->
        {put_style_dim(frame, "gap", value), state}

      # -----------------------------------------------------------------------
      # Layout: Size
      # -----------------------------------------------------------------------

      "width" ->
        {put_style(frame, "width", ~s("#{size_value(value)}")), state}

      "height" ->
        {put_style(frame, "height", ~s("#{size_value(value)}")), state}

      "min-width" ->
        {put_style_dim(frame, "minWidth", value), state}

      "max-width" ->
        {put_style_dim(frame, "maxWidth", value), state}

      "min-height" ->
        {put_style_dim(frame, "minHeight", value), state}

      "max-height" ->
        {put_style_dim(frame, "maxHeight", value), state}

      # -----------------------------------------------------------------------
      # Layout: Overflow
      # -----------------------------------------------------------------------

      "overflow" ->
        case value do
          %{kind: :string, value: v} ->
            overflow_map = %{"visible" => "visible", "hidden" => "hidden", "scroll" => "auto"}
            case Map.get(overflow_map, v) do
              nil -> {frame, state}
              css -> {put_style(frame, "overflow", ~s("#{css}")), state}
            end
          _ -> {frame, state}
        end

      # -----------------------------------------------------------------------
      # Layout: Alignment
      # -----------------------------------------------------------------------

      "align" ->
        case value do
          %{kind: :string, value: align_v} ->
            {apply_align(frame, align_v, tag), state}
          _ -> {frame, state}
        end

      # -----------------------------------------------------------------------
      # Visual: Background and Border
      # -----------------------------------------------------------------------

      "background" ->
        case value do
          %{kind: :color, r: r, g: g, b: b, a: a} ->
            {put_style(frame, "backgroundColor", ~s("#{rgba(r, g, b, a)}")), state}
          _ -> {frame, state}
        end

      "corner-radius" ->
        {put_style_dim(frame, "borderRadius", value), state}

      "border-width" ->
        case dim(value) do
          nil -> {frame, state}
          d ->
            frame2 = frame |> put_style("borderWidth", ~s("#{d}")) |> put_style("borderStyle", ~s("solid"))
            {frame2, state}
        end

      "border-color" ->
        case value do
          %{kind: :color, r: r, g: g, b: b, a: a} ->
            {put_style(frame, "borderColor", ~s("#{rgba(r, g, b, a)}")), state}
          _ -> {frame, state}
        end

      "opacity" ->
        case value do
          %{kind: :number, value: v} ->
            {put_style(frame, "opacity", "#{v}"), state}
          _ -> {frame, state}
        end

      # -----------------------------------------------------------------------
      # Visual: Shadow
      # -----------------------------------------------------------------------

      "shadow" ->
        case value do
          %{kind: :enum, namespace: "elevation", member: member} ->
            shadow_map = %{
              "none"   => "none",
              "low"    => "0 1px 3px rgba(0,0,0,0.12)",
              "medium" => "0 4px 12px rgba(0,0,0,0.15)",
              "high"   => "0 8px 24px rgba(0,0,0,0.20)"
            }
            case Map.get(shadow_map, member) do
              nil -> {frame, state}
              s -> {put_style(frame, "boxShadow", ~s("#{s}")), state}
            end
          _ -> {frame, state}
        end

      # -----------------------------------------------------------------------
      # Visual: Visibility
      # -----------------------------------------------------------------------

      "visible" ->
        case value do
          %{kind: :bool, value: false} ->
            {put_style(frame, "display", ~s("none")), state}
          _ -> {frame, state}
        end

      # -----------------------------------------------------------------------
      # Text-specific
      # -----------------------------------------------------------------------

      "content" when tag in ["Text", "span", "h2"] ->
        {%{frame | text_content: value_to_jsx(value)}, state}

      "color" ->
        case value do
          %{kind: :color, r: r, g: g, b: b, a: a} ->
            {put_style(frame, "color", ~s("#{rgba(r, g, b, a)}")), state}
          _ -> {frame, state}
        end

      "text-align" ->
        case value do
          %{kind: :string, value: v} ->
            text_align_map = %{"start" => "left", "center" => "center", "end" => "right"}
            case Map.get(text_align_map, v) do
              nil -> {frame, state}
              css -> {put_style(frame, "textAlign", ~s("#{css}")), state}
            end
          _ -> {frame, state}
        end

      "font-weight" ->
        case value do
          %{kind: :string, value: v} ->
            {put_style(frame, "fontWeight", ~s("#{v}")), state}
          _ -> {frame, state}
        end

      "max-lines" ->
        case value do
          %{kind: :number, value: v} ->
            frame2 =
              frame
              |> put_style("WebkitLineClamp", "#{v}")
              |> put_style("overflow", ~s("hidden"))
              |> put_style("display", ~s("-webkit-box"))
              |> put_style("WebkitBoxOrient", ~s("vertical"))
            {frame2, state}
          _ -> {frame, state}
        end

      "style" ->
        case value do
          %{kind: :enum, namespace: ns, member: m} ->
            class_name = "mosaic-#{ns}-#{m}"
            frame2 = %{frame | class_names: frame.class_names ++ [class_name]}
            state2 = %{state | needs_type_scale_css: true}
            {frame2, state2}
          %{kind: :string, value: v} ->
            frame2 = %{frame | class_names: frame.class_names ++ ["mosaic-#{v}"]}
            state2 = %{state | needs_type_scale_css: true}
            {frame2, state2}
          _ -> {frame, state}
        end

      # -----------------------------------------------------------------------
      # Image-specific
      # -----------------------------------------------------------------------

      "source" when tag == "Image" ->
        {%{frame | attrs: frame.attrs ++ ["src=#{attr_value(value)}"]}, state}

      "size" when tag == "Image" ->
        case dim(value) do
          nil -> {frame, state}
          d ->
            frame2 = frame |> put_style("width", ~s("#{d}")) |> put_style("height", ~s("#{d}"))
            {frame2, state}
        end

      "shape" when tag == "Image" ->
        case value do
          %{kind: :string, value: v} ->
            shape_map = %{"circle" => "50%", "rounded" => "8px"}
            case Map.get(shape_map, v) do
              nil -> {frame, state}
              r -> {put_style(frame, "borderRadius", ~s("#{r}")), state}
            end
          _ -> {frame, state}
        end

      "fit" when tag == "Image" ->
        case value do
          %{kind: :string, value: v} ->
            {put_style(frame, "objectFit", ~s("#{v}")), state}
          _ -> {frame, state}
        end

      # -----------------------------------------------------------------------
      # Accessibility
      # -----------------------------------------------------------------------

      "a11y-label" ->
        {%{frame | attrs: frame.attrs ++ ["aria-label=#{attr_value(value)}"]}, state}

      "a11y-role" ->
        case value do
          %{kind: :string, value: "none"} ->
            {%{frame | attrs: frame.attrs ++ ["aria-hidden=\"true\""]}, state}
          %{kind: :string, value: "heading"} ->
            {%{frame | attrs: frame.attrs ++ ["role=\"heading\""]}, state}
          %{kind: :string, value: "image"} ->
            {%{frame | attrs: frame.attrs ++ ["role=\"img\""]}, state}
          %{kind: :string, value: v} ->
            {%{frame | attrs: frame.attrs ++ ["role=\"#{v}\""]}, state}
          _ -> {frame, state}
        end

      "a11y-hidden" ->
        case value do
          %{kind: :bool, value: true} ->
            {%{frame | attrs: frame.attrs ++ ["aria-hidden=\"true\""]}, state}
          _ -> {frame, state}
        end

      _ ->
        {frame, state}
    end
  end

  # ---------------------------------------------------------------------------
  # Alignment helpers (private)
  # ---------------------------------------------------------------------------
  defp apply_align(frame, align_v, tag) do
    frame2 =
      if tag == "Box" do
        put_style(frame, "display", ~s("flex"))
      else
        frame
      end

    case {tag, align_v} do
      {"Column", "start"}             -> put_style(frame2, "alignItems", ~s("flex-start"))
      {"Column", "center"}            -> put_style(frame2, "alignItems", ~s("center"))
      {"Column", "end"}               -> put_style(frame2, "alignItems", ~s("flex-end"))
      {"Column", "stretch"}           -> put_style(frame2, "alignItems", ~s("stretch"))
      {"Column", "center-horizontal"} -> put_style(frame2, "alignItems", ~s("center"))
      {"Column", "center-vertical"}   -> put_style(frame2, "justifyContent", ~s("center"))
      {"Row", "start"}                -> put_style(frame2, "alignItems", ~s("flex-start"))
      {"Row", "center"}               -> frame2 |> put_style("alignItems", ~s("center")) |> put_style("justifyContent", ~s("center"))
      {"Row", "end"}                  -> frame2 |> put_style("alignItems", ~s("flex-end")) |> put_style("justifyContent", ~s("flex-end"))
      {"Row", "stretch"}              -> put_style(frame2, "alignItems", ~s("stretch"))
      {"Row", "center-horizontal"}    -> put_style(frame2, "justifyContent", ~s("center"))
      {"Row", "center-vertical"}      -> put_style(frame2, "alignItems", ~s("center"))
      {"Box", "start"}                -> put_style(frame2, "alignItems", ~s("flex-start"))
      {"Box", "center"}               -> put_style(frame2, "alignItems", ~s("center"))
      {"Box", "end"}                  -> put_style(frame2, "alignItems", ~s("flex-end"))
      {"Box", "stretch"}              -> put_style(frame2, "alignItems", ~s("stretch"))
      {"Box", "center-horizontal"}    -> put_style(frame2, "alignItems", ~s("center"))
      {"Box", "center-vertical"}      -> put_style(frame2, "justifyContent", ~s("center"))
      _                               -> frame2
    end
  end

  # ---------------------------------------------------------------------------
  # JSX Building (private)
  # ---------------------------------------------------------------------------

  # Append a JSX string to the top of the stack's lines list.
  defp append_to_parent(state, content) do
    stack = state.stack
    top = List.last(stack)
    updated_top = %{top | lines: top.lines ++ [content]}
    %{state | stack: List.replace_at(stack, length(stack) - 1, updated_top)}
  end

  # Convert a completed NodeFrame into a JSX element string.
  defp build_jsx_element(frame) do
    %{jsx_tag: jsx_tag, styles: styles, attrs: attrs, class_names: class_names,
      text_content: text_content, self_closing: self_closing, lines: lines} = frame

    # Build attribute string parts.
    parts = []

    parts =
      if map_size(styles) > 0 do
        style_entries =
          styles
          |> Enum.map(fn {k, v} -> "#{k}: #{v}" end)
          |> Enum.join(", ")
        parts ++ ["style={{ #{style_entries} }}"]
      else
        parts
      end

    parts =
      if length(class_names) > 0 do
        parts ++ ["className=\"#{Enum.join(class_names, " ")}\""]
      else
        parts
      end

    parts = parts ++ attrs

    attr_str = if length(parts) > 0, do: " " <> Enum.join(parts, " "), else: ""

    if self_closing do
      "<#{jsx_tag}#{attr_str} />"
    else
      children =
        cond do
          text_content != nil -> text_content
          true -> Enum.join(lines, "\n")
        end

      cond do
        children == "" ->
          "<#{jsx_tag}#{attr_str} />"

        text_content != nil ->
          "<#{jsx_tag}#{attr_str}>#{children}</#{jsx_tag}>"

        true ->
          # Block children: indent each line by 2 spaces.
          indented = children |> String.split("\n") |> Enum.map_join("\n", fn l -> "  " <> l end)
          "<#{jsx_tag}#{attr_str}>\n#{indented}\n</#{jsx_tag}>"
      end
    end
  end

  # ---------------------------------------------------------------------------
  # File Assembly (private)
  # ---------------------------------------------------------------------------

  defp build_file(state) do
    name = state.component_name

    # Collect prop lines and param lines for the function signature.
    {prop_lines, param_lines} =
      Enum.reduce(state.slots, {[], []}, fn slot, {props, params} ->
        ts_type = slot_type_to_ts(slot.type)
        optional = if slot.default_value != nil, do: "?", else: ""
        comment =
          if slot.default_value != nil,
            do: " // default: #{default_value_literal(slot.default_value)}",
            else: ""
        prop_line = "  #{slot.name}#{optional}: #{ts_type};#{comment}"

        param_line =
          if slot.default_value != nil,
            do: "  #{slot.name} = #{default_value_literal(slot.default_value)},",
            else: "  #{slot.name},"

        {props ++ [prop_line], params ++ [param_line]}
      end)

    # Import statements.
    import_lines =
      (
        state.slot_component_imports
        |> Enum.sort()
        |> Enum.map(fn comp -> "import type { #{comp}Props } from \"./#{comp}.js\";" end)
      ) ++
      (
        state.node_component_imports
        |> Enum.sort()
        |> Enum.map(fn comp -> "import { #{comp} } from \"./#{comp}.js\";" end)
      )

    # Root JSX content from the component frame.
    component_frame = List.first(state.stack)
    root_jsx = Enum.join(component_frame.lines, "\n")
    # Indent 4 spaces (2 for return body, 2 for JSX root)
    indented_root =
      root_jsx
      |> String.split("\n")
      |> Enum.map_join("\n", fn l -> "    " <> l end)

    # Build the file.
    lines = [
      "// AUTO-GENERATED from #{name}.mosaic — do not edit",
      "// Generated by mosaic-emit-react v1.0",
      "// Source: #{name}.mosaic",
      "//",
      "// To modify this component, edit #{name}.mosaic and re-run the compiler.",
      "",
      "import React from \"react\";"
    ]

    lines =
      if state.needs_type_scale_css do
        lines ++ ["import \"./mosaic-type-scale.css\";"]
      else
        lines
      end

    lines =
      if length(import_lines) > 0 do
        lines ++ [""] ++ import_lines
      else
        lines
      end

    lines = lines ++ [
      "",
      "interface #{name}Props {",
    ]

    lines = lines ++ prop_lines
    lines = lines ++ [
      "}",
      "",
      "export function #{name}({",
    ]
    lines = lines ++ param_lines
    lines = lines ++ [
      "}: #{name}Props): JSX.Element {",
      "  return (",
      indented_root,
      "  );",
      "}"
    ]

    Enum.join(lines, "\n")
  end

  # ---------------------------------------------------------------------------
  # Value Helpers (private)
  # ---------------------------------------------------------------------------

  # Convert a dimension ResolvedValue to a CSS px/% string. Returns nil if not a dimension.
  defp dim(%{kind: :dimension, value: v, unit: :percent}), do: "#{v}%"
  defp dim(%{kind: :dimension, value: v, unit: _}),        do: "#{v}px"
  defp dim(_),                                              do: nil

  # Convert a size ResolvedValue to a CSS string (handles fill/wrap/dimensions).
  defp size_value(%{kind: :string, value: "fill"}), do: "100%"
  defp size_value(%{kind: :string, value: "wrap"}), do: "fit-content"
  defp size_value(%{kind: :string, value: v}),      do: v
  defp size_value(v) do
    case dim(v) do
      nil -> "auto"
      d   -> d
    end
  end

  # Format a color as CSS rgba() string.
  # Alpha is normalized from 0–255 to 0–1. We round to 3 decimal places.
  defp rgba(r, g, b, a) do
    alpha = Float.round(a / 255.0 * 1000) / 1000
    "rgba(#{r}, #{g}, #{b}, #{alpha})"
  end

  # Convert a ResolvedValue to a JSX children expression.
  # Used for Text { content: … } where content becomes JSX children.
  defp value_to_jsx(%{kind: :string, value: v}),          do: v
  defp value_to_jsx(%{kind: :number, value: v}),          do: "#{v}"
  defp value_to_jsx(%{kind: :bool, value: v}),            do: "#{v}"
  defp value_to_jsx(%{kind: :slot_ref, slot_name: name}), do: "{#{name}}"
  defp value_to_jsx(_),                                   do: ""

  # Convert a ResolvedValue to a JSX attribute value string.
  defp attr_value(%{kind: :string, value: v}),              do: "\"#{v}\""
  defp attr_value(%{kind: :slot_ref, slot_name: name}),     do: "{#{name}}"
  defp attr_value(_),                                       do: "\"\""

  # Helper: put a style entry in a frame.
  defp put_style(frame, key, value) do
    %{frame | styles: Map.put(frame.styles, key, value)}
  end

  # Helper: put a dimension style entry (noop if value is not a dimension).
  defp put_style_dim(frame, css_key, value) do
    case dim(value) do
      nil -> frame
      d   -> put_style(frame, css_key, ~s("#{d}"))
    end
  end

  # ---------------------------------------------------------------------------
  # Type System Helpers (private)
  # ---------------------------------------------------------------------------

  # Convert a MosaicType map to its TypeScript prop type string.
  defp slot_type_to_ts(%{kind: :text}),      do: "string"
  defp slot_type_to_ts(%{kind: :number}),    do: "number"
  defp slot_type_to_ts(%{kind: :bool}),      do: "boolean"
  defp slot_type_to_ts(%{kind: :image}),     do: "string"
  defp slot_type_to_ts(%{kind: :color}),     do: "string"
  defp slot_type_to_ts(%{kind: :node}),      do: "React.ReactNode"
  defp slot_type_to_ts(%{kind: :component, name: n}), do: "React.ReactElement<#{n}Props>"
  defp slot_type_to_ts(%{kind: :list, element_type: et}) do
    case et do
      %{kind: :text}              -> "string[]"
      %{kind: :number}            -> "number[]"
      %{kind: :bool}              -> "boolean[]"
      %{kind: :image}             -> "string[]"
      %{kind: :color}             -> "string[]"
      %{kind: :node}              -> "React.ReactNode[]"
      %{kind: :component, name: n} -> "Array<React.ReactElement<#{n}Props>>"
      _                           -> "unknown[]"
    end
  end
  defp slot_type_to_ts(_), do: "unknown"

  # Convert a MosaicValue default to a TypeScript literal string.
  defp default_value_literal(%{kind: :string, value: v}), do: "\"#{v}\""
  defp default_value_literal(%{kind: :number, value: v}), do: "#{v}"
  defp default_value_literal(%{kind: :bool, value: v}),   do: "#{v}"
  defp default_value_literal(_),                          do: "undefined"
end
