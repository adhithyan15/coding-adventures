defmodule CodingAdventures.MosaicEmitWebcomponent do
  @moduledoc """
  Web Components backend for the Mosaic compiler — emits TypeScript Custom Element classes (.ts).

  This module implements the `CodingAdventures.MosaicVM.Renderer` behaviour and is
  driven by `CodingAdventures.MosaicVM`. The renderer produces a single TypeScript
  file containing a Custom Element that:

  - Extends `HTMLElement`
  - Uses Shadow DOM for style encapsulation
  - Exposes Mosaic slots as property setters/getters
  - Rebuilds shadow DOM content via `_render()` on any property change
  - Observes HTML attributes for primitive (text/number/bool/image/color) slots

  ## Architecture: Fragment Tree

  Unlike the React backend (which builds JSX via a string stack), the Web
  Components renderer builds a flat list of `RenderFragment` structs during the VM
  traversal and serializes them into `html +=` statements in `_render()`.

  ## Tag Name Convention

  PascalCase component names map to kebab-case element names with a `mosaic-` prefix:

      "ProfileCard" → "mosaic-profile-card"
      "Button"      → "mosaic-button"
      "HowItWorks"  → "mosaic-how-it-works"

  ## Security

  All text slot values are passed through `_escapeHtml()` before insertion into
  innerHTML. URL values (image source slots) are validated to reject `javascript:`
  scheme URIs.

  ## Usage

      {:ok, result} = CodingAdventures.MosaicVM.run(ir, __MODULE__, initial_state())
      # result.files[0].filename == "mosaic-my-component.ts"
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
      # Stack of frames for building during traversal.
      # Each frame is a map with :kind and :fragments list.
      stack: [],
      needs_type_scale_css: false
    }
  end

  # ---------------------------------------------------------------------------
  # MosaicVM.Renderer callbacks
  # ---------------------------------------------------------------------------

  @impl true
  def begin_component(state, name, slots) do
    %{state |
      component_name: name,
      slots: slots,
      stack: [%{kind: :component, fragments: []}],
      needs_type_scale_css: false
    }
  end

  @impl true
  def end_component(state) do
    content = build_file(state)
    tag_name = to_kebab_case(state.component_name)
    {:ok, %{files: [%{filename: "mosaic-#{tag_name}.ts", content: content}]}}
  end

  @impl true
  def begin_node(state, node_type, is_primitive, props) do
    {frame, updated_state} = build_node_frame(node_type, is_primitive, props, state)
    %{updated_state | stack: updated_state.stack ++ [frame]}
  end

  @impl true
  def end_node(state) do
    {stack_init, [frame]} = Enum.split(state.stack, length(state.stack) - 1)
    parent_frame = List.last(stack_init)

    new_fragments =
      if frame.self_closing do
        [{:self_closing, frame.open_html}]
      else
        close_tag = tag_to_html(frame.tag)
        open_close = [{:open_tag, frame.open_html}]

        content_frags =
          cond do
            frame.text_literal != nil ->
              # Static text content — escape it.
              [{:open_tag, escape_html_literal(frame.text_literal)}]
            frame.text_slot_expr != nil ->
              # Dynamic text slot — use escapeHtml.
              [{:slot_ref, frame.text_slot_expr}]
            true ->
              # Block children already accumulated in frame.fragments.
              frame.fragments
          end

        open_close ++ content_frags ++ [{:close_tag, close_tag}]
      end

    updated_parent = %{parent_frame | fragments: parent_frame.fragments ++ new_fragments}
    updated_stack = List.replace_at(stack_init, length(stack_init) - 1, updated_parent)
    %{state | stack: updated_stack}
  end

  @impl true
  def render_slot_child(state, slot_name, _slot_type) do
    # Light DOM projection via named <slot> element.
    append_fragment(state, {:slot_proj, slot_name})
  end

  @impl true
  def begin_when(state, slot_name) do
    frame = %{kind: :when, slot_name: slot_name, fragments: []}
    %{state | stack: state.stack ++ [frame]}
  end

  @impl true
  def end_when(state) do
    {stack_init, [frame]} = Enum.split(state.stack, length(state.stack) - 1)
    state1 = %{state | stack: stack_init}
    state2 = append_fragment(state1, {:when_open, frame.slot_name})
    state3 = Enum.reduce(frame.fragments, state2, fn frag, acc -> append_fragment(acc, frag) end)
    append_fragment(state3, :when_close)
  end

  @impl true
  def begin_each(state, slot_name, item_name) do
    # Find out if the list contains node/component types for slot projection.
    slot = Enum.find(state.slots, fn s -> s.name == slot_name end)
    is_node_list =
      case slot do
        %{type: %{kind: :list, element_type: %{kind: et}}} ->
          et == :node || et == :component
        _ -> false
      end
    frame = %{kind: :each, slot_name: slot_name, item_name: item_name, is_node_list: is_node_list, fragments: []}
    %{state | stack: state.stack ++ [frame]}
  end

  @impl true
  def end_each(state) do
    {stack_init, [frame]} = Enum.split(state.stack, length(state.stack) - 1)
    state1 = %{state | stack: stack_init}
    state2 = append_fragment(state1, {:each_open, frame.slot_name, frame.item_name, frame.is_node_list})
    state3 = Enum.reduce(frame.fragments, state2, fn frag, acc -> append_fragment(acc, frag) end)
    append_fragment(state3, :each_close)
  end

  # ---------------------------------------------------------------------------
  # Node Frame Building (private)
  # ---------------------------------------------------------------------------

  defp build_node_frame(tag, is_primitive, props, state) do
    {html_tag, base_styles, self_closing} = primitive_defaults(tag, is_primitive)

    frame = %{
      kind: :node,
      tag: tag,
      html_tag: html_tag,
      styles: base_styles,
      attrs: [],
      class_names: [],
      text_literal: nil,
      text_slot_expr: nil,
      self_closing: self_closing,
      fragments: []
    }

    {frame2, state2} =
      Enum.reduce(props, {frame, state}, fn prop, {fr, st} ->
        apply_property(prop, tag, fr, st)
      end)

    # Post-process: a11y-role: heading on Text → <h2>
    frame3 =
      if tag == "Text" && "role=\"heading\"" in frame2.attrs do
        %{frame2 |
          html_tag: "h2",
          attrs: List.delete(frame2.attrs, "role=\"heading\"")
        }
      else
        frame2
      end

    # Build the opening HTML string.
    style_str = if length(frame3.styles) > 0, do: Enum.join(frame3.styles, ";"), else: ""
    parts = []
    parts = if style_str != "", do: parts ++ ["style=\"#{style_str}\""], else: parts
    parts = if length(frame3.class_names) > 0, do: parts ++ ["class=\"#{Enum.join(frame3.class_names, " ")}\""], else: parts
    parts = parts ++ frame3.attrs

    attr_str = if length(parts) > 0, do: " " <> Enum.join(parts, " "), else: ""
    open_html = "<#{frame3.html_tag}#{attr_str}>"

    final_frame = Map.put(frame3, :open_html, open_html)
    {final_frame, state2}
  end

  # Returns {html_tag, base_styles_list, self_closing}
  defp primitive_defaults(tag, true) do
    case tag do
      "Box"     -> {"div", ["position:relative"], false}
      "Column"  -> {"div", ["display:flex", "flex-direction:column"], false}
      "Row"     -> {"div", ["display:flex", "flex-direction:row"], false}
      "Text"    -> {"span", [], false}
      "Image"   -> {"img", [], true}
      "Spacer"  -> {"div", ["flex:1"], false}
      "Scroll"  -> {"div", ["overflow:auto"], false}
      "Divider" -> {"hr", ["border:none", "border-top:1px solid currentColor"], true}
      _         -> {"div", [], false}
    end
  end

  defp primitive_defaults(_tag, false), do: {"div", [], false}

  # ---------------------------------------------------------------------------
  # Property Application (private)
  # ---------------------------------------------------------------------------

  # Returns {updated_frame, updated_state}
  defp apply_property(%{name: name, value: value}, tag, frame, state) do
    case name do
      # Layout: spacing
      "padding"        -> {put_style_dim(frame, "padding", value), state}
      "padding-left"   -> {put_style_dim(frame, "padding-left", value), state}
      "padding-right"  -> {put_style_dim(frame, "padding-right", value), state}
      "padding-top"    -> {put_style_dim(frame, "padding-top", value), state}
      "padding-bottom" -> {put_style_dim(frame, "padding-bottom", value), state}
      "gap"            -> {put_style_dim(frame, "gap", value), state}

      # Layout: size
      "width"      -> {put_css_style(frame, "width", size_value(value)), state}
      "height"     -> {put_css_style(frame, "height", size_value(value)), state}
      "min-width"  -> {put_style_dim(frame, "min-width", value), state}
      "max-width"  -> {put_style_dim(frame, "max-width", value), state}
      "min-height" -> {put_style_dim(frame, "min-height", value), state}
      "max-height" -> {put_style_dim(frame, "max-height", value), state}

      # Layout: overflow
      "overflow" ->
        case value do
          %{kind: :string, value: v} ->
            overflow_map = %{"visible" => "visible", "hidden" => "hidden", "scroll" => "auto"}
            case Map.get(overflow_map, v) do
              nil -> {frame, state}
              css -> {put_css_style(frame, "overflow", css), state}
            end
          _ -> {frame, state}
        end

      # Layout: alignment
      "align" ->
        case value do
          %{kind: :string, value: v} -> {apply_align(frame, v, tag), state}
          _ -> {frame, state}
        end

      # Visual: background, border
      "background" ->
        case value do
          %{kind: :color, r: r, g: g, b: b, a: a} ->
            {put_css_style(frame, "background-color", rgba(r, g, b, a)), state}
          _ -> {frame, state}
        end

      "corner-radius" -> {put_style_dim(frame, "border-radius", value), state}

      "border-width" ->
        case dim(value) do
          nil -> {frame, state}
          d ->
            frame2 = frame |> put_css_style("border-width", d) |> put_css_style("border-style", "solid")
            {frame2, state}
        end

      "border-color" ->
        case value do
          %{kind: :color, r: r, g: g, b: b, a: a} ->
            {put_css_style(frame, "border-color", rgba(r, g, b, a)), state}
          _ -> {frame, state}
        end

      "opacity" ->
        case value do
          %{kind: :number, value: v} -> {put_css_style(frame, "opacity", "#{v}"), state}
          _ -> {frame, state}
        end

      # Visual: shadow
      "shadow" ->
        case value do
          %{kind: :enum, namespace: "elevation", member: m} ->
            shadow_map = %{
              "none"   => "none",
              "low"    => "0 1px 3px rgba(0,0,0,0.12)",
              "medium" => "0 4px 12px rgba(0,0,0,0.15)",
              "high"   => "0 8px 24px rgba(0,0,0,0.20)"
            }
            case Map.get(shadow_map, m) do
              nil -> {frame, state}
              s -> {put_css_style(frame, "box-shadow", s), state}
            end
          _ -> {frame, state}
        end

      # Visual: visibility
      "visible" ->
        case value do
          %{kind: :bool, value: false} -> {put_css_style(frame, "display", "none"), state}
          _ -> {frame, state}
        end

      # Text-specific
      "content" when tag == "Text" ->
        case value do
          %{kind: :string, value: v} ->
            {%{frame | text_literal: v}, state}
          %{kind: :slot_ref, slot_name: sn, is_loop_var: true} ->
            {%{frame | text_slot_expr: sn}, state}
          %{kind: :slot_ref, slot_name: sn} ->
            {%{frame | text_slot_expr: "this._escapeHtml(this._#{sn})"}, state}
          _ -> {frame, state}
        end

      "color" ->
        case value do
          %{kind: :color, r: r, g: g, b: b, a: a} ->
            {put_css_style(frame, "color", rgba(r, g, b, a)), state}
          _ -> {frame, state}
        end

      "text-align" ->
        case value do
          %{kind: :string, value: v} ->
            m = %{"start" => "left", "center" => "center", "end" => "right"}
            case Map.get(m, v) do
              nil -> {frame, state}
              css -> {put_css_style(frame, "text-align", css), state}
            end
          _ -> {frame, state}
        end

      "font-weight" ->
        case value do
          %{kind: :string, value: v} -> {put_css_style(frame, "font-weight", v), state}
          _ -> {frame, state}
        end

      "max-lines" ->
        case value do
          %{kind: :number, value: v} ->
            frame2 =
              frame
              |> put_css_style("-webkit-line-clamp", "#{v}")
              |> put_css_style("overflow", "hidden")
              |> put_css_style("display", "-webkit-box")
              |> put_css_style("-webkit-box-orient", "vertical")
            {frame2, state}
          _ -> {frame, state}
        end

      "style" ->
        case value do
          %{kind: :enum, namespace: ns, member: m} ->
            frame2 = %{frame | class_names: frame.class_names ++ ["mosaic-#{ns}-#{m}"]}
            state2 = %{state | needs_type_scale_css: true}
            {frame2, state2}
          %{kind: :string, value: v} ->
            frame2 = %{frame | class_names: frame.class_names ++ ["mosaic-#{v}"]}
            state2 = %{state | needs_type_scale_css: true}
            {frame2, state2}
          _ -> {frame, state}
        end

      # Image-specific
      "source" when tag == "Image" ->
        case value do
          %{kind: :string, value: v} ->
            {%{frame | attrs: frame.attrs ++ ["src=\"#{escape_attr(v)}\""]}, state}
          %{kind: :slot_ref, slot_name: sn} ->
            # Dynamic: use placeholder replaced at serialization time.
            {%{frame | attrs: frame.attrs ++ ["src=\"__IMG_SRC_#{sn}__\""]}, state}
          _ -> {frame, state}
        end

      "size" when tag == "Image" ->
        case dim(value) do
          nil -> {frame, state}
          d ->
            frame2 = frame |> put_css_style("width", d) |> put_css_style("height", d)
            {frame2, state}
        end

      "shape" when tag == "Image" ->
        case value do
          %{kind: :string, value: v} ->
            shape_map = %{"circle" => "50%", "rounded" => "8px"}
            case Map.get(shape_map, v) do
              nil -> {frame, state}
              r -> {put_css_style(frame, "border-radius", r), state}
            end
          _ -> {frame, state}
        end

      "fit" when tag == "Image" ->
        case value do
          %{kind: :string, value: v} -> {put_css_style(frame, "object-fit", v), state}
          _ -> {frame, state}
        end

      # Accessibility
      "a11y-label" ->
        case value do
          %{kind: :string, value: v} ->
            {%{frame | attrs: frame.attrs ++ ["aria-label=\"#{escape_attr(v)}\""]}, state}
          %{kind: :slot_ref, slot_name: sn} ->
            {%{frame | attrs: frame.attrs ++ ["aria-label=\"__ARIA_#{sn}__\""]}, state}
          _ -> {frame, state}
        end

      "a11y-role" ->
        case value do
          %{kind: :string, value: "none"}    -> {%{frame | attrs: frame.attrs ++ ["aria-hidden=\"true\""]}, state}
          %{kind: :string, value: "heading"} -> {%{frame | attrs: frame.attrs ++ ["role=\"heading\""]}, state}
          %{kind: :string, value: "image"}   -> {%{frame | attrs: frame.attrs ++ ["role=\"img\""]}, state}
          %{kind: :string, value: v}          -> {%{frame | attrs: frame.attrs ++ ["role=\"#{v}\""]}, state}
          _ -> {frame, state}
        end

      "a11y-hidden" ->
        case value do
          %{kind: :bool, value: true} -> {%{frame | attrs: frame.attrs ++ ["aria-hidden=\"true\""]}, state}
          _ -> {frame, state}
        end

      _ -> {frame, state}
    end
  end

  # ---------------------------------------------------------------------------
  # Alignment (private)
  # ---------------------------------------------------------------------------

  defp apply_align(frame, align_v, tag) do
    frame2 = if tag == "Box", do: put_css_style(frame, "display", "flex"), else: frame

    case {tag, align_v} do
      {"Column", "start"}             -> put_css_style(frame2, "align-items", "flex-start")
      {"Column", "center"}            -> put_css_style(frame2, "align-items", "center")
      {"Column", "end"}               -> put_css_style(frame2, "align-items", "flex-end")
      {"Column", "stretch"}           -> put_css_style(frame2, "align-items", "stretch")
      {"Column", "center-horizontal"} -> put_css_style(frame2, "align-items", "center")
      {"Column", "center-vertical"}   -> put_css_style(frame2, "justify-content", "center")
      {"Row", "start"}                -> put_css_style(frame2, "align-items", "flex-start")
      {"Row", "center"}               -> frame2 |> put_css_style("align-items", "center") |> put_css_style("justify-content", "center")
      {"Row", "end"}                  -> frame2 |> put_css_style("align-items", "flex-end") |> put_css_style("justify-content", "flex-end")
      {"Row", "stretch"}              -> put_css_style(frame2, "align-items", "stretch")
      {"Row", "center-horizontal"}    -> put_css_style(frame2, "justify-content", "center")
      {"Row", "center-vertical"}      -> put_css_style(frame2, "align-items", "center")
      {"Box", "start"}                -> put_css_style(frame2, "align-items", "flex-start")
      {"Box", "center"}               -> put_css_style(frame2, "align-items", "center")
      {"Box", "end"}                  -> put_css_style(frame2, "align-items", "flex-end")
      {"Box", "stretch"}              -> put_css_style(frame2, "align-items", "stretch")
      {"Box", "center-horizontal"}    -> put_css_style(frame2, "align-items", "center")
      {"Box", "center-vertical"}      -> put_css_style(frame2, "justify-content", "center")
      _                               -> frame2
    end
  end

  # ---------------------------------------------------------------------------
  # Fragment management (private)
  # ---------------------------------------------------------------------------

  # Append a fragment to the top-of-stack frame's fragments list.
  defp append_fragment(state, frag) do
    stack = state.stack
    top = List.last(stack)
    updated_top = %{top | fragments: top.fragments ++ [frag]}
    %{state | stack: List.replace_at(stack, length(stack) - 1, updated_top)}
  end

  # ---------------------------------------------------------------------------
  # Fragment Serialization (private)
  # ---------------------------------------------------------------------------

  # Escape a string for use inside a single-quoted JS string literal.
  defp single_quote_escape(s) do
    s |> String.replace("\\", "\\\\") |> String.replace("'", "\\'")
  end

  defp serialize_fragments(fragments, indent) do
    Enum.flat_map(fragments, fn frag ->
      case frag do
        {:open_tag, html} ->
          ["#{indent}html += '#{single_quote_escape(html)}';"]

        {:close_tag, tag} ->
          ["#{indent}html += '</#{tag}>';"]

        {:self_closing, html} ->
          ["#{indent}html += '#{single_quote_escape(html)}';"]

        {:slot_ref, expr} ->
          ["#{indent}html += `${#{expr}}`;"]

        {:slot_proj, slot_name} ->
          ["#{indent}html += '<slot name=\"#{slot_name}\"></slot>';"]

        {:when_open, field} ->
          ["#{indent}if (this._#{field}) {"]

        :when_close ->
          ["#{indent}}"]

        {:each_open, field, _item_name, true} ->
          # Node list: emit indexed named slots
          [
            "#{indent}this._#{field}.forEach((_item, _i) => {",
            "#{indent}  html += `<slot name=\"#{field}-${_i}\"></slot>`;"
          ]

        {:each_open, field, item_name, false} ->
          ["#{indent}this._#{field}.forEach(#{item_name} => {"]

        :each_close ->
          ["#{indent}});"]
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # File Assembly (private)
  # ---------------------------------------------------------------------------

  defp build_file(state) do
    name = state.component_name
    class_name = "Mosaic#{name}Element"
    tag_name = to_kebab_case(name)
    element_tag = "mosaic-#{tag_name}"

    # Slot categorization
    observable_slots = Enum.filter(state.slots, &observable_type?(&1.type))
    node_slots = Enum.filter(state.slots, fn s -> s.type.kind in [:node, :component] end)
    image_slots = Enum.filter(state.slots, fn s -> s.type.kind == :image end)
    list_slots = Enum.filter(state.slots, fn s -> s.type.kind == :list end)
    has_node_slots = length(node_slots) > 0

    _ = image_slots  # used in setters
    _ = list_slots   # categorized but setters handled generically below

    # Backing field declarations
    field_lines =
      Enum.map(state.slots, fn slot ->
        "  private #{backing_field(slot.name)}: #{ts_field_type(slot.type)} = #{default_value(slot)};"
      end)

    # observedAttributes
    observed_attr_names =
      observable_slots
      |> Enum.map(fn s -> "'#{s.name}'" end)
      |> Enum.join(", ")

    # attributeChangedCallback case lines
    attr_case_lines =
      Enum.map(observable_slots, fn slot ->
        field = backing_field(slot.name)
        setter =
          case slot.type.kind do
            :number ->
              "#{field} = parseFloat(value ?? '#{default_scalar(slot)}');"
            :bool ->
              "#{field} = value !== null;"
            _ ->
              scalar = default_scalar(slot)
              "#{field} = value ?? '#{String.replace(scalar, "'", "\\'")}'"  <> ";"
          end
        "    case '#{slot.name}': this.#{setter} break;"
      end)

    # Property setters/getters
    setter_lines =
      Enum.flat_map(state.slots, fn slot ->
        field = backing_field(slot.name)
        ts_type = ts_field_type(slot.type)
        case slot.type.kind do
          k when k in [:node, :component] ->
            ["  set #{slot.name}(v: HTMLElement) { this._projectSlot('#{slot.name}', v); }"]
          :image ->
            [
              "  set #{slot.name}(v: string) {",
              "    if (/^javascript:/i.test(v.trim())) return;",
              "    #{field} = v;",
              "    this._render();",
              "  }",
              "  get #{slot.name}(): string { return #{field}; }"
            ]
          :list ->
            ["  set #{slot.name}(v: #{ts_type}) { #{field} = v; this._render(); }"]
          _ ->
            [
              "  set #{slot.name}(v: #{ts_type}) { #{field} = v; this._render(); }",
              "  get #{slot.name}(): #{ts_type} { return #{field}; }"
            ]
        end
      end)

    # _render() body
    component_frame = List.first(state.stack)
    raw_render_lines = serialize_fragments(component_frame.fragments, "    ")

    # Replace image source and aria placeholders with actual field references.
    # In Elixir, String.replace with a regex and a function receives the full match string.
    # We extract the slot name by stripping the placeholder prefix/suffix.
    render_lines =
      Enum.map(raw_render_lines, fn line ->
        line
        |> String.replace(~r/__IMG_SRC_(\w+)__/, fn _full, slot_name ->
          "\" + this._validateUrl(this.#{backing_field(slot_name)}) + \""
        end)
        |> String.replace(~r/__ARIA_(\w+)__/, fn _full, slot_name ->
          "\" + this._escapeHtml(this.#{backing_field(slot_name)}) + \""
        end)
      end)

    # Assemble the file.
    lines = [
      "// AUTO-GENERATED from #{name}.mosaic — do not edit",
      "// Generated by mosaic-emit-webcomponent v1.0",
      "// Source: #{name}.mosaic",
      "//",
      "// To modify this component, edit #{name}.mosaic and re-run the compiler.",
      ""
    ]

    lines =
      if state.needs_type_scale_css do
        lines ++ [
          "const MOSAIC_TYPE_SCALE_CSS = `",
          "  .mosaic-heading-large { font-size: 2rem; font-weight: 700; line-height: 1.2; }",
          "  .mosaic-heading-medium { font-size: 1.5rem; font-weight: 600; line-height: 1.3; }",
          "  .mosaic-heading-small { font-size: 1.25rem; font-weight: 600; line-height: 1.4; }",
          "  .mosaic-body-large { font-size: 1rem; line-height: 1.6; }",
          "  .mosaic-body-medium { font-size: 0.875rem; line-height: 1.6; }",
          "  .mosaic-body-small { font-size: 0.75rem; line-height: 1.5; }",
          "  .mosaic-label { font-size: 0.875rem; font-weight: 500; }",
          "  .mosaic-caption { font-size: 0.75rem; color: #666; }",
          "`;",
          ""
        ]
      else
        lines
      end

    lines = lines ++ ["export class #{class_name} extends HTMLElement {", "  private _shadow: ShadowRoot;", ""]

    lines =
      if length(field_lines) > 0 do
        lines ++ ["  // Backing fields for Mosaic slots"] ++ field_lines ++ [""]
      else
        lines
      end

    lines = lines ++ [
      "  constructor() {",
      "    super();",
      "    this._shadow = this.attachShadow({ mode: 'open' });",
      "  }",
      ""
    ]

    lines =
      if length(observable_slots) > 0 do
        lines ++ [
          "  static get observedAttributes(): string[] {",
          "    return [#{observed_attr_names}];",
          "  }",
          "",
          "  attributeChangedCallback(name: string, _old: string | null, value: string | null): void {",
          "    switch (name) {",
        ] ++ attr_case_lines ++ [
          "    }",
          "    this._render();",
          "  }",
          ""
        ]
      else
        lines
      end

    lines =
      if length(setter_lines) > 0 do
        lines ++ ["  // Property setters and getters"] ++ setter_lines ++ [""]
      else
        lines
      end

    lines =
      if has_node_slots do
        lines ++ [
          "  // Light DOM slot projection for node/component-type slots",
          "  private _projectSlot(name: string, node: Element): void {",
          "    const prev = this.querySelector(`[data-mosaic-slot=\"${name}\"]`);",
          "    if (prev) prev.remove();",
          "    node.setAttribute('slot', name);",
          "    node.setAttribute('data-mosaic-slot', name);",
          "    this.appendChild(node);",
          "  }",
          ""
        ]
      else
        lines
      end

    lines = lines ++ [
      "  private _escapeHtml(s: string): string {",
      "    return s",
      "      .replace(/&/g, '&amp;')",
      "      .replace(/</g, '&lt;')",
      "      .replace(/>/g, '&gt;')",
      "      .replace(/\"/g, '&quot;')",
      "      .replace(/'/g, '&#39;');",
      "  }",
      "",
      "  connectedCallback(): void { this._render(); }",
      ""
    ]

    lines =
      if has_node_slots do
        lines ++ [
          "  disconnectedCallback(): void {",
          "    [...this.querySelectorAll('[data-mosaic-slot]')].forEach((el) => el.remove());",
          "  }",
          ""
        ]
      else
        lines
      end

    lines = lines ++ [
      "  private _render(): void {",
      "    let html = '';"
    ]

    lines =
      if state.needs_type_scale_css do
        lines ++ ["    html += `<style>${MOSAIC_TYPE_SCALE_CSS}</style>`;"]
      else
        lines
      end

    lines = lines ++ render_lines ++ [
      "    this._shadow.innerHTML = html;",
      "  }",
      "}",
      "",
      "customElements.define('#{element_tag}', #{class_name});"
    ]

    Enum.join(lines, "\n")
  end

  # ---------------------------------------------------------------------------
  # Type helpers (private)
  # ---------------------------------------------------------------------------

  defp ts_field_type(%{kind: :text}),      do: "string"
  defp ts_field_type(%{kind: :number}),    do: "number"
  defp ts_field_type(%{kind: :bool}),      do: "boolean"
  defp ts_field_type(%{kind: :image}),     do: "string"
  defp ts_field_type(%{kind: :color}),     do: "string"
  defp ts_field_type(%{kind: :node}),      do: "HTMLElement | null"
  defp ts_field_type(%{kind: :component}), do: "HTMLElement | null"
  defp ts_field_type(%{kind: :list, element_type: et}) do
    case et do
      %{kind: k} when k in [:node, :component] -> "Element[]"
      %{kind: :text}   -> "string[]"
      %{kind: :number} -> "number[]"
      %{kind: :bool}   -> "boolean[]"
      _                -> "unknown[]"
    end
  end
  defp ts_field_type(_), do: "unknown"

  defp default_value(%{type: %{kind: :text}, default_value: %{kind: :string, value: v}}), do: "'#{v}'"
  defp default_value(%{type: %{kind: :number}, default_value: %{kind: :number, value: v}}), do: "#{v}"
  defp default_value(%{type: %{kind: :bool}, default_value: %{kind: :bool, value: v}}), do: "#{v}"
  defp default_value(%{type: %{kind: :text}}),      do: "''"
  defp default_value(%{type: %{kind: :number}}),    do: "0"
  defp default_value(%{type: %{kind: :bool}}),      do: "false"
  defp default_value(%{type: %{kind: :image}}),     do: "''"
  defp default_value(%{type: %{kind: :color}}),     do: "''"
  defp default_value(%{type: %{kind: :node}}),      do: "null"
  defp default_value(%{type: %{kind: :component}}), do: "null"
  defp default_value(%{type: %{kind: :list}}),      do: "[]"
  defp default_value(_),                            do: "null"

  defp default_scalar(%{default_value: %{kind: :string, value: v}}), do: v
  defp default_scalar(%{default_value: %{kind: :number, value: v}}), do: "#{v}"
  defp default_scalar(%{default_value: %{kind: :bool, value: v}}),   do: "#{v}"
  defp default_scalar(%{type: %{kind: :number}}), do: "0"
  defp default_scalar(_), do: ""


  defp observable_type?(%{kind: k}), do: k in [:text, :number, :bool, :image, :color]

  defp backing_field(slot_name), do: "_#{slot_name}"

  defp tag_to_html(tag) do
    case tag do
      "Column"  -> "div"
      "Row"     -> "div"
      "Box"     -> "div"
      "Spacer"  -> "div"
      "Scroll"  -> "div"
      "Text"    -> "span"
      "Image"   -> "img"
      "Divider" -> "hr"
      _         -> "div"
    end
  end

  # ---------------------------------------------------------------------------
  # Value helpers (private)
  # ---------------------------------------------------------------------------

  defp dim(%{kind: :dimension, value: v, unit: :percent}), do: "#{v}%"
  defp dim(%{kind: :dimension, value: v, unit: _}),        do: "#{v}px"
  defp dim(_),                                              do: nil

  defp size_value(%{kind: :string, value: "fill"}), do: "100%"
  defp size_value(%{kind: :string, value: "wrap"}), do: "fit-content"
  defp size_value(%{kind: :string, value: v}),      do: v
  defp size_value(v), do: dim(v) || "auto"

  defp rgba(r, g, b, a) do
    alpha = Float.round(a / 255.0 * 1000) / 1000
    "rgba(#{r}, #{g}, #{b}, #{alpha})"
  end

  defp escape_attr(value) do
    value
    |> String.replace("&", "&amp;")
    |> String.replace("<", "&lt;")
    |> String.replace(">", "&gt;")
    |> String.replace("\"", "&quot;")
  end

  defp escape_html_literal(s) do
    s
    |> String.replace("&", "&amp;")
    |> String.replace("<", "&lt;")
    |> String.replace(">", "&gt;")
  end

  # Convert PascalCase to kebab-case.
  # "ProfileCard" → "profile-card", "HowItWorks" → "how-it-works"
  defp to_kebab_case(name) do
    name
    |> String.replace(~r/([A-Z])/, "-\\1")
    |> String.downcase()
    |> String.trim_leading("-")
  end

  # ---------------------------------------------------------------------------
  # Style helpers (private)
  # ---------------------------------------------------------------------------

  # Append a CSS "key:value" entry to frame.styles list.
  defp put_css_style(frame, key, value) do
    %{frame | styles: frame.styles ++ ["#{key}:#{value}"]}
  end

  # Append a dimension style (noop if value is not a dimension).
  defp put_style_dim(frame, css_key, value) do
    case dim(value) do
      nil -> frame
      d   -> put_css_style(frame, css_key, d)
    end
  end
end
