defmodule CodingAdventures.AudioDeviceSink.Error do
  @moduledoc """
  Error category and message for audio sink validation and backend failures.
  """

  defexception [:kind, :detail]

  @impl true
  def message(%__MODULE__{kind: :invalid_format, detail: detail}),
    do: "invalid PCM format: #{detail}"

  def message(%__MODULE__{kind: :invalid_samples, detail: detail}),
    do: "invalid PCM samples: #{detail}"

  def message(%__MODULE__{kind: :unsupported_platform, detail: detail}),
    do: "unsupported platform: #{detail}"

  def message(%__MODULE__{kind: :backend_unavailable, detail: detail}),
    do: "audio backend unavailable: #{detail}"

  def message(%__MODULE__{kind: :backend_failure, detail: detail}),
    do: "audio backend failure: #{detail}"

  def message(%__MODULE__{detail: detail}), do: to_string(detail)
end

defmodule CodingAdventures.AudioDeviceSink.PcmFormat do
  @moduledoc """
  Metadata that tells a sink how to interpret signed integer samples.
  """

  alias CodingAdventures.AudioDeviceSink.Error

  defstruct [:sample_rate_hz, :channel_count, :bit_depth]

  @max_sample_rate_hz 384_000
  @supported_bit_depth 16
  @supported_channel_count 1

  @type t :: %__MODULE__{
          sample_rate_hz: pos_integer(),
          channel_count: pos_integer(),
          bit_depth: pos_integer()
        }

  def new(sample_rate_hz, channel_count, bit_depth) do
    format = %__MODULE__{
      sample_rate_hz: sample_rate_hz,
      channel_count: channel_count,
      bit_depth: bit_depth
    }

    with :ok <- validate(format), do: {:ok, format}
  end

  def validate(%__MODULE__{sample_rate_hz: rate}) when not is_integer(rate) or rate <= 0 do
    {:error, %Error{kind: :invalid_format, detail: "sample_rate_hz must be greater than zero"}}
  end

  def validate(%__MODULE__{sample_rate_hz: rate}) when rate > @max_sample_rate_hz do
    {:error,
     %Error{kind: :invalid_format, detail: "sample_rate_hz must be <= #{@max_sample_rate_hz}"}}
  end

  def validate(%__MODULE__{channel_count: count}) when count != @supported_channel_count do
    {:error,
     %Error{
       kind: :invalid_format,
       detail: "only mono PCM is supported in V1, got #{count} channels"
     }}
  end

  def validate(%__MODULE__{bit_depth: depth}) when depth != @supported_bit_depth do
    {:error,
     %Error{
       kind: :invalid_format,
       detail: "only signed 16-bit PCM is supported in V1, got #{depth} bits"
     }}
  end

  def validate(%__MODULE__{}), do: :ok

  def sample_width_bytes(%__MODULE__{bit_depth: bit_depth}), do: div(bit_depth, 8)
end

defmodule CodingAdventures.AudioDeviceSink.PcmPlaybackBuffer do
  @moduledoc """
  Owned signed PCM samples plus the format needed by an audio backend.
  """

  alias CodingAdventures.AudioDeviceSink.Error
  alias CodingAdventures.AudioDeviceSink.PcmFormat

  defstruct [:samples, :format]

  @pcm16_min -32_768
  @pcm16_max 32_767
  @max_blocking_duration_seconds 10 * 60

  @type t :: %__MODULE__{samples: [integer()], format: PcmFormat.t()}

  def new(samples, %PcmFormat{} = format) do
    with :ok <- PcmFormat.validate(format),
         {:ok, normalized} <- normalize_samples(samples, format.sample_rate_hz) do
      {:ok, %__MODULE__{samples: normalized, format: format}}
    end
  end

  def sample_count(%__MODULE__{samples: samples}), do: length(samples)

  def frame_count(%__MODULE__{} = buffer),
    do: div(sample_count(buffer), buffer.format.channel_count)

  def empty?(%__MODULE__{} = buffer), do: sample_count(buffer) == 0

  def duration_seconds(%__MODULE__{} = buffer),
    do: frame_count(buffer) / buffer.format.sample_rate_hz

  defp normalize_samples(samples, sample_rate_hz) do
    max_samples = sample_rate_hz * @max_blocking_duration_seconds

    result =
      samples
      |> Enum.with_index()
      |> Enum.reduce_while({:ok, []}, fn {sample, index}, {:ok, acc} ->
        cond do
          not is_integer(sample) ->
            {:halt,
             {:error,
              %Error{kind: :invalid_samples, detail: "samples[#{index}] must be an integer"}}}

          sample < @pcm16_min or sample > @pcm16_max ->
            {:halt,
             {:error,
              %Error{
                kind: :invalid_samples,
                detail: "samples[#{index}] must fit signed 16-bit PCM, got #{sample}"
              }}}

          length(acc) + 1 > max_samples ->
            {:halt,
             {:error,
              %Error{
                kind: :invalid_samples,
                detail:
                  "blocking playback is limited to #{@max_blocking_duration_seconds} seconds"
              }}}

          true ->
            {:cont, {:ok, [sample | acc]}}
        end
      end)

    case result do
      {:ok, reversed} -> {:ok, Enum.reverse(reversed)}
      error -> error
    end
  end
end

defmodule CodingAdventures.AudioDeviceSink.PlaybackReport do
  @moduledoc """
  Summary returned after a sink accepts or completes playback.
  """

  alias CodingAdventures.AudioDeviceSink.PcmPlaybackBuffer

  defstruct [:frames_played, :sample_rate_hz, :channel_count, :duration_seconds, :backend_name]

  def for_buffer(%PcmPlaybackBuffer{} = buffer, backend_name) do
    %__MODULE__{
      frames_played: PcmPlaybackBuffer.frame_count(buffer),
      sample_rate_hz: buffer.format.sample_rate_hz,
      channel_count: buffer.format.channel_count,
      duration_seconds: PcmPlaybackBuffer.duration_seconds(buffer),
      backend_name: backend_name
    }
  end
end

defmodule CodingAdventures.AudioDeviceSink.NoopAudioSink do
  @moduledoc """
  Test sink that accepts buffers without touching a device.
  """

  alias CodingAdventures.AudioDeviceSink.PcmPlaybackBuffer
  alias CodingAdventures.AudioDeviceSink.PlaybackReport

  defstruct backend_name: "noop"

  def new(backend_name \\ "noop"), do: %__MODULE__{backend_name: backend_name}

  def play_blocking(%__MODULE__{backend_name: backend_name}, %PcmPlaybackBuffer{} = buffer) do
    {:ok, PlaybackReport.for_buffer(buffer, backend_name)}
  end
end

defmodule CodingAdventures.AudioDeviceSink do
  @moduledoc """
  Facade for backend-neutral PCM playback primitives.
  """

  @version "0.1.0"
  @max_blocking_duration_seconds 10 * 60
  @max_sample_rate_hz 384_000
  @pcm16_min -32_768
  @pcm16_max 32_767

  def version, do: @version
  def max_blocking_duration_seconds, do: @max_blocking_duration_seconds
  def max_sample_rate_hz, do: @max_sample_rate_hz
  def pcm16_min, do: @pcm16_min
  def pcm16_max, do: @pcm16_max
end
