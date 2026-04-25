defmodule CodingAdventures.AudioDeviceSinkTest do
  use ExUnit.Case

  alias CodingAdventures.AudioDeviceSink
  alias CodingAdventures.AudioDeviceSink.Error
  alias CodingAdventures.AudioDeviceSink.NoopAudioSink
  alias CodingAdventures.AudioDeviceSink.PcmFormat
  alias CodingAdventures.AudioDeviceSink.PcmPlaybackBuffer

  test "valid format records pcm metadata" do
    assert {:ok, format} = PcmFormat.new(44_100, 1, 16)

    assert AudioDeviceSink.version() == "0.1.0"
    assert format.sample_rate_hz == 44_100
    assert format.channel_count == 1
    assert format.bit_depth == 16
    assert PcmFormat.sample_width_bytes(format) == 2
  end

  test "format rejects invalid values" do
    assert {:error, %Error{} = zero} = PcmFormat.new(0, 1, 16)
    assert Exception.message(zero) =~ "sample_rate_hz"

    assert {:error, %Error{} = high} =
             PcmFormat.new(AudioDeviceSink.max_sample_rate_hz() + 1, 1, 16)

    assert Exception.message(high) =~ "384000"

    assert {:error, %Error{} = channels} = PcmFormat.new(44_100, 2, 16)
    assert Exception.message(channels) =~ "mono"

    assert {:error, %Error{} = bits} = PcmFormat.new(44_100, 1, 24)
    assert Exception.message(bits) =~ "16-bit"
  end

  test "buffer reports counts and duration" do
    {:ok, format} = PcmFormat.new(4, 1, 16)
    assert {:ok, buffer} = PcmPlaybackBuffer.new([0, 1000, 0, -1000], format)

    assert PcmPlaybackBuffer.sample_count(buffer) == 4
    assert PcmPlaybackBuffer.frame_count(buffer) == 4
    assert PcmPlaybackBuffer.duration_seconds(buffer) == 1.0
    refute PcmPlaybackBuffer.empty?(buffer)
  end

  test "buffer rejects invalid samples and too-long payloads" do
    {:ok, format} = PcmFormat.new(1, 1, 16)

    assert {:error, %Error{} = non_integer} = PcmPlaybackBuffer.new([1.5], format)
    assert Exception.message(non_integer) =~ "integer"

    assert {:error, %Error{} = out_of_range} =
             PcmPlaybackBuffer.new([AudioDeviceSink.pcm16_max() + 1], format)

    assert Exception.message(out_of_range) =~ "signed 16-bit"

    samples = List.duplicate(0, AudioDeviceSink.max_blocking_duration_seconds() + 1)
    assert {:error, %Error{} = too_long} = PcmPlaybackBuffer.new(samples, format)
    assert Exception.message(too_long) =~ "limited"
  end

  test "noop sink returns a report and accepts empty buffers" do
    {:ok, format} = PcmFormat.new(44_100, 1, 16)
    {:ok, buffer} = PcmPlaybackBuffer.new([0, 1000], format)

    assert {:ok, report} = NoopAudioSink.new("test-noop") |> NoopAudioSink.play_blocking(buffer)
    assert report.frames_played == 2
    assert report.sample_rate_hz == 44_100
    assert report.channel_count == 1
    assert report.backend_name == "test-noop"

    {:ok, empty} = PcmPlaybackBuffer.new([], format)
    assert PcmPlaybackBuffer.empty?(empty)
    assert {:ok, empty_report} = NoopAudioSink.new("empty") |> NoopAudioSink.play_blocking(empty)
    assert empty_report.duration_seconds == 0.0
  end

  test "error strings name categories" do
    assert Exception.message(%Error{kind: :backend_unavailable, detail: "default device missing"}) ==
             "audio backend unavailable: default device missing"

    assert Exception.message(%Error{
             kind: :unsupported_platform,
             detail: "Core Audio requires macOS"
           }) ==
             "unsupported platform: Core Audio requires macOS"

    assert Exception.message(%Error{kind: :backend_failure, detail: "write failed"}) ==
             "audio backend failure: write failed"
  end
end
