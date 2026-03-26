defmodule CodingAdventures.DrawInstructionsSvg do
  @moduledoc """
  SVG renderer for backend-neutral draw instructions.
  """

  def render(scene) do
    label = Map.get(scene.metadata, :label, "draw instructions scene") |> xml_escape()

    [
      ~s(<svg xmlns="http://www.w3.org/2000/svg" width="#{scene.width}" height="#{scene.height}" viewBox="0 0 #{scene.width} #{scene.height}" role="img" aria-label="#{label}">),
      ~s(  <rect x="0" y="0" width="#{scene.width}" height="#{scene.height}" fill="#{xml_escape(scene.background)}" />),
      Enum.map(scene.instructions, &render_instruction/1),
      "</svg>"
    ]
    |> List.flatten()
    |> Enum.join("\n")
  end

  defp render_instruction(%{kind: :rect} = instruction) do
    ~s(  <rect x="#{instruction.x}" y="#{instruction.y}" width="#{instruction.width}" height="#{instruction.height}" fill="#{xml_escape(instruction.fill)}"#{metadata_to_attributes(instruction.metadata)} />)
  end

  defp render_instruction(%{kind: :text} = instruction) do
    ~s(  <text x="#{instruction.x}" y="#{instruction.y}" text-anchor="#{instruction.align}" font-family="#{xml_escape(instruction.font_family)}" font-size="#{instruction.font_size}" fill="#{xml_escape(instruction.fill)}"#{metadata_to_attributes(instruction.metadata)}>#{xml_escape(instruction.value)}</text>)
  end

  defp render_instruction(%{kind: :group} = instruction) do
    children = Enum.map_join(instruction.children, "\n", &render_instruction/1)
    "  <g#{metadata_to_attributes(instruction.metadata)}>\n#{children}\n  </g>"
  end

  defp metadata_to_attributes(metadata) do
    Enum.map_join(metadata, "", fn {key, value} -> ~s( data-#{key}="#{xml_escape(to_string(value))}") end)
  end

  defp xml_escape(value) do
    value
    |> String.replace("&", "&amp;")
    |> String.replace("<", "&lt;")
    |> String.replace(">", "&gt;")
    |> String.replace("\"", "&quot;")
    |> String.replace("'", "&apos;")
  end
end
