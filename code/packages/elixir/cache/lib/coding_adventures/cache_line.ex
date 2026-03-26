defmodule CodingAdventures.CacheLine do
  @moduledoc """
  A single cache line in the hierarchy.
  """

  @enforce_keys [:data]
  defstruct valid: false, dirty: false, tag: 0, last_access: 0, data: []

  @type t :: %__MODULE__{
          valid: boolean(),
          dirty: boolean(),
          tag: non_neg_integer(),
          last_access: non_neg_integer(),
          data: [non_neg_integer()]
        }

  @spec new(pos_integer()) :: t()
  def new(line_size \\ 64) when is_integer(line_size) and line_size > 0 do
    %__MODULE__{data: List.duplicate(0, line_size)}
  end

  @spec fill(t(), non_neg_integer(), [non_neg_integer()], non_neg_integer()) :: t()
  def fill(%__MODULE__{} = line, tag, data, cycle)
      when is_integer(tag) and tag >= 0 and is_list(data) and is_integer(cycle) and cycle >= 0 do
    %{
      line
      | valid: true,
        dirty: false,
        tag: tag,
        data: Enum.to_list(data),
        last_access: cycle
    }
  end

  @spec touch(t(), non_neg_integer()) :: t()
  def touch(%__MODULE__{} = line, cycle) when is_integer(cycle) and cycle >= 0 do
    %{line | last_access: cycle}
  end

  @spec invalidate(t()) :: t()
  def invalidate(%__MODULE__{} = line) do
    %{line | valid: false, dirty: false}
  end

  @spec line_size(t()) :: non_neg_integer()
  def line_size(%__MODULE__{} = line), do: length(line.data)
end
