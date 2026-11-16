defmodule CodingAdventures.ImageCodecPng.PngError do
  @moduledoc """
  Exception representing malformed, invalid, or unsupported PNG input.
  Enforces closed error code mapping and payload-blind diagnostic messages.
  """

  defexception [:code, :reason, :message]

  alias CodingAdventures.ImageCodecPng.Messages

  @impl true
  def exception(opts) when is_list(opts) do
    if Keyword.keyword?(opts) do
      raw = Keyword.get(opts, :code) || Keyword.get(opts, :reason)
      code = Messages.error_code(raw)
      message = Messages.message_for(code)

      %__MODULE__{
        code: code,
        reason: raw,
        message: message
      }
    else
      exception(reason: nil)
    end
  end

  def exception(reason) when is_atom(reason) or is_binary(reason) do
    exception(reason: reason)
  end

  def exception(_), do: exception(reason: nil)
end
