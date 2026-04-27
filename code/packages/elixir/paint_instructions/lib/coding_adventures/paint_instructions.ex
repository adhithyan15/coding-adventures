defmodule CodingAdventures.PaintInstructions do
  @moduledoc """
  Backend-neutral paint scene primitives.
  """

  @type metadata_value :: String.t() | number() | boolean()
  @type metadata :: %{optional(atom()) => metadata_value()}

  @type paint_rect :: %{
          kind: :rect,
          x: number(),
          y: number(),
          width: number(),
          height: number(),
          fill: String.t(),
          metadata: metadata()
        }

  @type paint_instruction :: paint_rect()

  @type paint_scene :: %{
          width: number(),
          height: number(),
          background: String.t(),
          instructions: [paint_instruction()],
          metadata: metadata()
        }

  @spec paint_rect(number(), number(), number(), number(), String.t(), metadata()) :: paint_rect()
  def paint_rect(x, y, width, height, fill \\ "#000000", metadata \\ %{}) do
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

  @spec paint_scene(number(), number(), [paint_instruction()], String.t(), metadata()) ::
          paint_scene()
  def paint_scene(width, height, instructions, background \\ "#ffffff", metadata \\ %{}) do
    %{
      width: width,
      height: height,
      background: background,
      instructions: instructions,
      metadata: metadata
    }
  end

  @spec create_scene(number(), number(), [paint_instruction()], String.t(), metadata()) ::
          paint_scene()
  def create_scene(width, height, instructions, background \\ "#ffffff", metadata \\ %{}) do
    paint_scene(width, height, instructions, background, metadata)
  end
end
