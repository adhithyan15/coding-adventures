defmodule CodingAdventures.InMemoryDataStore do
  @moduledoc """
  Composition layer that turns RESP bytes into datastore responses.
  """

  alias CodingAdventures.InMemoryDataStoreEngine
  alias CodingAdventures.InMemoryDataStoreProtocol
  alias CodingAdventures.RESPProtocol

  defstruct [:engine]

  def new(opts \\ []) do
    %__MODULE__{engine: InMemoryDataStoreEngine.new(opts)}
  end

  def execute_command(%__MODULE__{engine: engine} = data_store, command_name, args \\ []) do
    {engine, reply} = InMemoryDataStoreEngine.execute(engine, InMemoryDataStoreProtocol.build(command_name, args))
    {%{data_store | engine: engine}, reply}
  end

  def execute_resp(%__MODULE__{} = data_store, binary) when is_binary(binary) do
    with {:ok, resp_value, rest} <- RESPProtocol.decode(binary),
         :ok <- ensure_no_trailing(rest),
         {:ok, command} <- InMemoryDataStoreProtocol.from_resp(resp_value) do
      {data_store, reply} = execute_command(data_store, command.name, command.args)
      {data_store, RESPProtocol.encode(reply)}
    else
      {:error, reason} -> {data_store, RESPProtocol.encode(RESPProtocol.error("ERR #{inspect(reason)}"))}
      {:error, reason, _rest} -> {data_store, RESPProtocol.encode(RESPProtocol.error("ERR #{inspect(reason)}"))}
    end
  end

  defp ensure_no_trailing(""), do: :ok
  defp ensure_no_trailing(_), do: {:error, :trailing_input}
end
