defmodule CodingAdventures.DrawInstructions do
  @moduledoc """
  Backend-neutral draw scene primitives.

  This module separates producer logic from renderer logic:

  - producer packages decide what should be drawn
  - renderer packages decide how to serialize or paint that scene
  """

  def draw_rect(x, y, width, height, fill \\ "#000000", metadata \\ %{}) do
    %{
      kind: :rect,
      x: x,
      y: y,
      width: width,
      height: height,
      fill: fill,
      metadata: metadata
    }
  end

  def draw_text(x, y, value, metadata \\ %{}) do
    %{
      kind: :text,
      x: x,
      y: y,
      value: value,
      fill: "#000000",
      font_family: "monospace",
      font_size: 16,
      align: "middle",
      metadata: metadata
    }
  end

  def draw_group(children, metadata \\ %{}) do
    %{kind: :group, children: children, metadata: metadata}
  end

  def create_scene(width, height, instructions, background \\ "#ffffff", metadata \\ %{}) do
    %{
      width: width,
      height: height,
      background: background,
      instructions: instructions,
      metadata: metadata
    }
  end

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
