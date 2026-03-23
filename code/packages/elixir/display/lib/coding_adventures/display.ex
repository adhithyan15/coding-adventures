defmodule CodingAdventures.Display do
  @moduledoc "VGA text-mode framebuffer display simulator."

  @bytes_per_cell 2
  @default_attribute 0x07

  defstruct [:config, :memory, :cursor]

  defmodule Config do
    defstruct columns: 80, rows: 25, framebuffer_base: 0xFFFB0000, default_attribute: 0x07
  end

  defmodule Snapshot do
    defstruct [:lines, :cursor, :rows, :columns]

    def contains(%__MODULE__{lines: lines}, text) do
      Enum.any?(lines, &String.contains?(&1, text))
    end

    def line_at(%__MODULE__{lines: lines}, row) when row >= 0 and row < length(lines) do
      Enum.at(lines, row)
    end

    def line_at(_, _), do: ""
  end

  def new(%Config{} = config) do
    size = config.columns * config.rows * @bytes_per_cell
    memory = :binary.copy(<<0>>, size)
    d = %__MODULE__{config: config, memory: memory, cursor: {0, 0}}
    clear(d)
  end

  def new_default, do: new(%Config{})

  def put_char(%__MODULE__{config: config, cursor: {row, col}} = d, ch) do
    cond do
      ch == ?\n -> scroll_if_needed(%{d | cursor: {row + 1, 0}})
      ch == ?\r -> %{d | cursor: {row, 0}}
      ch == ?\t ->
        new_col = (div(col, 8) + 1) * 8
        if new_col >= config.columns do
          scroll_if_needed(%{d | cursor: {row + 1, 0}})
        else
          %{d | cursor: {row, new_col}}
        end
      ch == ?\b -> %{d | cursor: {row, max(col - 1, 0)}}
      true ->
        offset = (row * config.columns + col) * @bytes_per_cell
        memory = write_at(d.memory, offset, <<ch, config.default_attribute>>)
        new_col = col + 1
        if new_col >= config.columns do
          scroll_if_needed(%{d | memory: memory, cursor: {row + 1, 0}})
        else
          %{d | memory: memory, cursor: {row, new_col}}
        end
    end
  end

  def puts(d, string) do
    string
    |> String.to_charlist()
    |> Enum.reduce(d, fn ch, acc -> put_char(acc, ch) end)
  end

  def clear(%__MODULE__{config: config} = d) do
    total = config.columns * config.rows * @bytes_per_cell
    memory = for _ <- 1..div(total, 2), into: <<>>, do: <<0x20, config.default_attribute>>
    %{d | memory: memory, cursor: {0, 0}}
  end

  def scroll(%__MODULE__{config: config} = d) do
    bytes_per_row = config.columns * @bytes_per_cell
    total = config.rows * bytes_per_row
    <<_first_row::binary-size(bytes_per_row), rest::binary>> = d.memory
    blank_row = for _ <- 1..config.columns, into: <<>>, do: <<0x20, config.default_attribute>>
    memory = <<rest::binary, blank_row::binary>>
    memory = :binary.part(memory, 0, total)
    %{d | memory: memory, cursor: {config.rows - 1, 0}}
  end

  def snapshot(%__MODULE__{config: config, cursor: cursor, memory: memory}) do
    lines =
      for row <- 0..(config.rows - 1) do
        chars =
          for col <- 0..(config.columns - 1) do
            offset = (row * config.columns + col) * @bytes_per_cell
            :binary.at(memory, offset)
          end
        chars |> List.to_string() |> String.trim_trailing()
      end

    %Snapshot{lines: lines, cursor: cursor, rows: config.rows, columns: config.columns}
  end

  def get_cell(%__MODULE__{config: config, memory: memory}, row, col)
      when row >= 0 and row < config.rows and col >= 0 and col < config.columns do
    offset = (row * config.columns + col) * @bytes_per_cell
    {:binary.at(memory, offset), :binary.at(memory, offset + 1)}
  end

  def get_cell(%__MODULE__{config: config}, _, _), do: {0x20, config.default_attribute}

  def set_cursor(%__MODULE__{config: config} = d, row, col) do
    row = max(0, min(row, config.rows - 1))
    col = max(0, min(col, config.columns - 1))
    %{d | cursor: {row, col}}
  end

  defp scroll_if_needed(%__MODULE__{config: config, cursor: {row, _}} = d) when row >= config.rows, do: scroll(d)
  defp scroll_if_needed(d), do: d

  defp write_at(binary, offset, bytes) do
    len = byte_size(bytes)
    <<before::binary-size(offset), _old::binary-size(len), rest::binary>> = binary
    <<before::binary, bytes::binary, rest::binary>>
  end
end
