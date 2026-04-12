defmodule CodingAdventures.InMemoryDataStoreProtocol do
  @moduledoc """
  Typed command protocol sitting above RESP.
  """

  alias CodingAdventures.RESPProtocol

  defmodule Command do
    @enforce_keys [:name, :args]
    defstruct [:name, :args]
  end

  def parse(resp_value), do: from_resp(resp_value)

  def from_resp({:array, [head | tail]}) do
    with {:ok, name} <- atomize_command_token(head),
         {:ok, args} <- Enum.reduce_while(tail, {:ok, []}, fn item, {:ok, acc} ->
           case atomize_command_token(item) do
             {:ok, token} -> {:cont, {:ok, acc ++ [token]}}
             {:error, reason} -> {:halt, {:error, reason}}
           end
         end) do
      {:ok, %Command{name: name, args: args}}
    end
  end

  def from_resp(:null_array), do: {:error, :empty_command}
  def from_resp(_), do: {:error, :invalid_command}

  def to_resp(%Command{name: name, args: args}) do
    RESPProtocol.array([RESPProtocol.bulk_string(name) | Enum.map(args, &RESPProtocol.bulk_string/1)])
  end

  def build(name, args \\ []) do
    %Command{name: String.upcase(to_string(name)), args: Enum.map(args, &to_string/1)}
  end

  defp atomize_command_token({:bulk_string, value}), do: {:ok, to_string(value)}
  defp atomize_command_token({:simple_string, value}), do: {:ok, to_string(value)}
  defp atomize_command_token(:null_bulk_string), do: {:error, :invalid_command}
  defp atomize_command_token(other), do: {:error, {:invalid_token, other}}
end
