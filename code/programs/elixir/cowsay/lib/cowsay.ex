defmodule CodingAdventures.Cowsay do
  alias CodingAdventures.CliBuilder
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  def main(argv) do
    root = find_root()
    spec_path = Path.join([root, "code", "specs", "cowsay.json"])

    # Parser.parse/2 expects the first element to be the program name if it matches
    # but we can also just pass the argv as is if it doesn't match.
    # We'll prepend the program name "cowsay" to be safe and ensure it handles it.
    case CliBuilder.parse(spec_path, ["cowsay" | argv]) do
      {:ok, %ParseResult{} = result} ->
        handle_result(result, root)

      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: v}} ->
        IO.puts(v)

      {:error, %ParseErrors{message: msg}} ->
        IO.puts(:stderr, "error: #{msg}")
        System.halt(1)
    end
  end

  defp handle_result(result, root) do
    flags = result.flags
    args = result.arguments

    # Handle message
    message =
      case args["message"] do
        [] ->
          if not IO.atty?(:stdio) do
            IO.read(:all) |> String.trim()
          else
            ""
          end

        parts ->
          Enum.join(parts, " ")
      end

    if message != "" do
      # Handle modes
      {eyes, tongue} = get_modes(flags)

      # Handle wrapping
      lines =
        if flags["nowrap"] do
          String.split(message, "\n")
        else
          width = Map.get(flags, "width", 40)

          message
          |> String.split("\n")
          |> Enum.flat_map(fn line ->
            if line == "", do: [""], else: wrap_text(line, width)
          end)
        end

      # Handle speech vs thought
      is_think = flags["think"] or Map.get(flags, "explicit_flags", []) |> Enum.member?("think")
      # Note: detecting from script name is harder in Elixir mix run.
      
      thoughts = if is_think, do: "o", else: "\\"

      # Generate bubble
      bubble = format_bubble(lines, is_think)

      # Load and render cow
      cowfile = Map.get(flags, "cowfile", "default")
      cow_template = load_cow(cowfile, root)

      # Replace placeholders
      cow =
        cow_template
        |> String.replace("$eyes", eyes)
        |> String.replace("$tongue", tongue)
        |> String.replace("$thoughts", thoughts)
        |> String.replace("\\\\", "\\")

      IO.puts(bubble)
      IO.puts(cow)
    end
  end

  defp get_modes(flags) do
    eyes = Map.get(flags, "eyes", "oo")
    tongue = Map.get(flags, "tongue", "  ")

    cond do
      flags["borg"] -> {"==", tongue}
      flags["dead"] -> {"XX", "U "}
      flags["greedy"] -> {"$$", tongue}
      flags["paranoid"] -> {"@@", tongue}
      flags["stoned"] -> {"xx", "U "}
      flags["tired"] -> {"--", tongue}
      flags["wired"] -> {"OO", tongue}
      flags["youthful"] -> {"..", tongue}
      true -> {String.pad_trailing(eyes, 2), String.pad_trailing(tongue, 2)}
    end
    |> then(fn {e, t} ->
      {String.slice(e, 0, 2), String.slice(t, 0, 2)}
    end)
  end

  defp wrap_text(text, width) do
    words = String.split(text)

    Enum.reduce(words, {[], ""}, fn word, {lines, current} ->
      if String.length(current) + String.length(word) + 1 <= width do
        new_current = if current == "", do: word, else: current <> " " <> word
        {lines, new_current}
      else
        {lines ++ [current], word}
      end
    end)
    |> then(fn {lines, last} -> lines ++ [last] end)
  end

  defp format_bubble(lines, is_think) do
    max_len = lines |> Enum.map(&String.length/1) |> Enum.max()
    border_top = " " <> String.duplicate("_", max_len + 2)
    border_bottom = " " <> String.duplicate("-", max_len + 2)

    middle =
      if length(lines) == 1 do
        {start, end_char} = if is_think, do: {"(", ")"}, else: {"<", ">"}
        ["#{start} #{String.pad_trailing(hd(lines), max_len)} #{end_char}"]
      else
        lines
        |> Enum.with_index()
        |> Enum.map(fn {line, i} ->
          {start, end_char} =
            cond do
              is_think -> {"(", ")"}
              i == 0 -> {"/", "\\"}
              i == length(lines) - 1 -> {"\\", "/"}
              true -> {"|", "|"}
            end

          "#{start} #{String.pad_trailing(line, max_len)} #{end_char}"
        end)
      end

    [border_top | middle] ++ [border_bottom] |> Enum.join("\n")
  end

  defp load_cow(cow_name, root) do
    cow_path = Path.join([root, "code", "specs", "cows", "#{cow_name}.cow"])

    cow_path =
      if File.exists?(cow_path) do
        cow_path
      else
        Path.join([root, "code", "specs", "cows", "default.cow"])
      end

    content = File.read!(cow_path)

    case Regex.run(~r/<<EOC;\n([\s\S]*?)EOC/s, content) do
      [_, cow] -> cow
      _ -> content
    end
  end

  defp find_root do
    Path.expand(".")
    |> Path.split()
    |> Enum.reverse()
    |> find_root_recursive()
  end

  defp find_root_recursive([]), do: "."

  defp find_root_recursive(parts) do
    path = parts |> Enum.reverse() |> Path.join()

    if File.exists?(Path.join(path, "code/specs/cowsay.json")) do
      path
    else
      find_root_recursive(tl(parts))
    end
  end
end
