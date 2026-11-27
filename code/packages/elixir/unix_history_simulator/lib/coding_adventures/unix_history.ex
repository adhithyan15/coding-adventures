defmodule CodingAdventures.UnixHistory do
  @moduledoc """
  A small, reusable First Edition Unix filesystem and kernel model.

  The in-memory inode tree deliberately keeps path lookup, descriptor offsets,
  kernel events, and the teaching shell in separate functions. That makes the
  components useful to callers which do not want the command adapter.
  """
  defstruct fs: nil, slots: %{}, capacity: 4

  @type inode :: %{number: integer(), data: binary() | nil, entries: [{String.t(), integer()}]}
  @root %{number: 1, data: nil, entries: [{"hello", 2}, {"empty", 3}, {"etc", 4}]}
  @inodes %{
    1 => @root,
    2 => %{number: 2, data: "Hello, Unix!\n", entries: []},
    3 => %{number: 3, data: "", entries: []},
    4 => %{number: 4, data: nil, entries: [{"notes", 5}]},
    5 => %{
      number: 5,
      data: "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n",
      entries: []
    }
  }

  @doc "Creates an independent kernel with a four-slot descriptor table."
  def new(capacity \\ 4) when is_integer(capacity) and capacity > 0 and capacity <= 1024,
    do: %__MODULE__{fs: @inodes, capacity: capacity}

  @doc "Resolves a path, returning an inode or a stable Unix-style error string."
  def resolve(fs, path) do
    if not is_binary(path) or path == "" or
         (is_binary(path) and :binary.match(path, <<0>>) != :nomatch) do
      {:error, "EINVAL"}
    else
      do_resolve(fs, path)
    end
  end

  defp do_resolve(fs, path) do
    root = Map.fetch!(fs, 1)
    segments = String.split(path, "/", trim: true)

    result =
      Enum.reduce_while(segments, {root, []}, fn name, {node, parents} ->
        cond do
          node.data != nil ->
            {:halt, {:error, "ENOTDIR"}}

          name == "." ->
            {:cont, {node, parents}}

          name == ".." ->
            {:cont,
             case parents do
               [parent | rest] -> {parent, rest}
               [] -> {root, []}
             end}

          true ->
            case Enum.find(node.entries, fn {entry, _} -> entry == name end) do
              nil -> {:halt, {:error, "ENOENT"}}
              {_, ino} -> {:cont, {Map.fetch!(fs, ino), [node | parents]}}
            end
        end
      end)

    case result do
      {:error, error} ->
        {:error, error}

      {node, _} ->
        if String.ends_with?(path, "/") and node.data != nil,
          do: {:error, "ENOTDIR"},
          else: {:ok, node}
    end
  end

  @doc "Opens a regular file in the lowest free descriptor slot."
  def open(%__MODULE__{} = state, path) do
    case resolve(state.fs, path) do
      {:error, error} ->
        {state, %{"call" => "open", "path" => path, "error" => error}}

      {:ok, %{data: nil}} ->
        {state, %{"call" => "open", "path" => path, "error" => "EISDIR"}}

      {:ok, inode} ->
        fd = Enum.find(0..(state.capacity - 1), &(not Map.has_key?(state.slots, &1)))

        if fd == nil do
          {state, %{"call" => "open", "path" => path, "error" => "EMFILE"}}
        else
          next = put_in(state.slots[fd], {inode.number, 0})

          {next,
           %{"call" => "open", "path" => path, "inode" => inode.number, "fd" => fd, "offset" => 0}}
        end
    end
  end

  @doc "Reads bytes and advances only the selected descriptor's offset."
  def read(%__MODULE__{} = state, fd, count) do
    case Map.get(state.slots, fd) do
      nil ->
        {state, %{"call" => "read", "fd" => fd, "error" => "EBADF"}, <<>>}

      _ when count < 0 ->
        {state, %{"call" => "read", "fd" => fd, "error" => "EINVAL"}, <<>>}

      {ino, offset} ->
        data =
          state.fs[ino].data
          |> binary_part(
            min(offset, byte_size(state.fs[ino].data)),
            min(count, max(0, byte_size(state.fs[ino].data) - offset))
          )

        next_offset = offset + byte_size(data)
        next = put_in(state.slots[fd], {ino, next_offset})

        {next,
         %{
           "call" => "read",
           "fd" => fd,
           "inode" => ino,
           "offset" => offset,
           "bytes" => byte_size(data),
           "nextOffset" => next_offset
         }, data}
    end
  end

  @doc "Closes one descriptor without affecting other opens."
  def close(%__MODULE__{} = state, fd) do
    if Map.has_key?(state.slots, fd),
      do: {%{state | slots: Map.delete(state.slots, fd)}, %{"call" => "close", "fd" => fd}},
      else: {state, %{"call" => "close", "fd" => fd, "error" => "EBADF"}}
  end

  @doc "Lists directory names in image order."
  def list(%__MODULE__{} = state, path) do
    case resolve(state.fs, path) do
      {:error, error} ->
        %{"call" => "list", "path" => path, "error" => error}

      {:ok, %{data: data}} when data != nil ->
        %{"call" => "list", "path" => path, "error" => "ENOTDIR"}

      {:ok, node} ->
        %{
          "call" => "list",
          "path" => path,
          "inode" => node.number,
          "names" => Enum.map(node.entries, &elem(&1, 0))
        }
    end
  end

  @doc "Runs the small cat/ls teaching shell and returns JSON-compatible observations."
  def run_command(command) do
    words = String.split(command, ~r/[ \t\n\r\f\v]+/, trim: true)

    {output, events} =
      case words do
        ["cat", path] ->
          {state, opened} = open(new(), path)

          if Map.has_key?(opened, "fd") do
            fd = opened["fd"]
            {state, chunks, reads} = read_all(state, fd, [], [])
            {_state, closed} = close(state, fd)
            {IO.iodata_to_binary(Enum.reverse(chunks)), [opened | reads] ++ [closed]}
          else
            {"", [opened]}
          end

        ["ls"] ->
          list = list(new(), "/")
          {names_output(list), [list]}

        ["ls", path] ->
          list = list(new(), path)
          {names_output(list), [list]}

        [] ->
          {"", []}

        _ ->
          {"usage: cat PATH or ls [PATH]\n", []}
      end

    errors = for event <- events, error = event["error"], do: "error: #{error}\n"
    %{"output" => output <> Enum.join(errors), "events" => events}
  end

  defp read_all(state, fd, chunks, events) do
    {next, event, bytes} = read(state, fd, 8)
    all_events = events ++ [event]

    if byte_size(bytes) == 0,
      do: {next, chunks, all_events},
      else: read_all(next, fd, [bytes | chunks], all_events)
  end

  defp names_output(%{"names" => names}), do: Enum.map_join(names, "", &(&1 <> "\n"))
  defp names_output(_), do: ""
end
