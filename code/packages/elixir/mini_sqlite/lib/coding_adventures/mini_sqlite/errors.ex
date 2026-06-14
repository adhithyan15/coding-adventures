defmodule CodingAdventures.MiniSqlite.Errors do
  @moduledoc "Exception types returned by the mini-sqlite facade."

  defmodule ProgrammingError do
    defexception [:message]
  end

  defmodule OperationalError do
    defexception [:message]
  end

  defmodule IntegrityError do
    defexception [:message]
  end

  defmodule NotSupportedError do
    defexception [:message]
  end
end
