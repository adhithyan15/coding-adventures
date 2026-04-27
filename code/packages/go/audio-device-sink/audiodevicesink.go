// Package audiodevicesink defines backend-neutral PCM playback primitives.
package audiodevicesink

import "fmt"

const Version = "0.1.0"
const MaxBlockingDurationSeconds = 10.0 * 60.0
const MaxSampleRateHz uint32 = 384000
const SupportedBitDepth uint16 = 16
const SupportedChannelCount uint16 = 1

type PcmFormat struct {
	SampleRateHz uint32
	ChannelCount uint16
	BitDepth     uint16
}

func NewPcmFormat(sampleRateHz uint32, channelCount uint16, bitDepth uint16) (PcmFormat, error) {
	format := PcmFormat{SampleRateHz: sampleRateHz, ChannelCount: channelCount, BitDepth: bitDepth}
	return format, format.Validate()
}

func (f PcmFormat) Validate() error {
	if f.SampleRateHz == 0 {
		return InvalidFormat("sample_rate_hz must be greater than zero")
	}
	if f.SampleRateHz > MaxSampleRateHz {
		return InvalidFormat(fmt.Sprintf("sample_rate_hz must be <= %d, got %d", MaxSampleRateHz, f.SampleRateHz))
	}
	if f.ChannelCount != SupportedChannelCount {
		return InvalidFormat(fmt.Sprintf("only mono PCM is supported in V1, got %d channels", f.ChannelCount))
	}
	if f.BitDepth != SupportedBitDepth {
		return InvalidFormat(fmt.Sprintf("only signed 16-bit PCM is supported in V1, got %d bits", f.BitDepth))
	}
	return nil
}

func (f PcmFormat) SampleWidthBytes() int {
	return int(f.BitDepth / 8)
}

type PcmPlaybackBuffer struct {
	samples []int16
	format  PcmFormat
}

func NewPcmPlaybackBuffer(samples []int16, format PcmFormat) (*PcmPlaybackBuffer, error) {
	if err := format.Validate(); err != nil {
		return nil, err
	}
	copied := append([]int16(nil), samples...)
	buffer := &PcmPlaybackBuffer{samples: copied, format: format}
	if err := buffer.validateSize(); err != nil {
		return nil, err
	}
	return buffer, nil
}

func (b *PcmPlaybackBuffer) Samples() []int16 {
	return append([]int16(nil), b.samples...)
}

func (b *PcmPlaybackBuffer) Format() PcmFormat {
	return b.format
}

func (b *PcmPlaybackBuffer) SampleCount() int {
	return len(b.samples)
}

func (b *PcmPlaybackBuffer) FrameCount() int {
	return b.SampleCount() / int(b.format.ChannelCount)
}

func (b *PcmPlaybackBuffer) IsEmpty() bool {
	return b.SampleCount() == 0
}

func (b *PcmPlaybackBuffer) DurationSeconds() float64 {
	return float64(b.FrameCount()) / float64(b.format.SampleRateHz)
}

func (b *PcmPlaybackBuffer) validateSize() error {
	maxSamples := float64(b.format.SampleRateHz) * MaxBlockingDurationSeconds
	if float64(b.SampleCount()) > maxSamples {
		return InvalidSamples(fmt.Sprintf("blocking playback is limited to %.0f seconds", MaxBlockingDurationSeconds))
	}
	return nil
}

type PlaybackReport struct {
	FramesPlayed    int
	SampleRateHz    uint32
	ChannelCount    uint16
	DurationSeconds float64
	BackendName     string
}

func ReportForBuffer(buffer *PcmPlaybackBuffer, backendName string) PlaybackReport {
	return PlaybackReport{
		FramesPlayed:    buffer.FrameCount(),
		SampleRateHz:    buffer.format.SampleRateHz,
		ChannelCount:    buffer.format.ChannelCount,
		DurationSeconds: buffer.DurationSeconds(),
		BackendName:     backendName,
	}
}

type AudioSink interface {
	PlayBlocking(buffer *PcmPlaybackBuffer) (PlaybackReport, error)
}

type NoopAudioSink struct {
	BackendName string
}

func NewNoopAudioSink(backendName string) NoopAudioSink {
	return NoopAudioSink{BackendName: backendName}
}

func (s NoopAudioSink) PlayBlocking(buffer *PcmPlaybackBuffer) (PlaybackReport, error) {
	return ReportForBuffer(buffer, s.BackendName), nil
}

type AudioSinkError struct {
	Kind    string
	Message string
}

func (e AudioSinkError) Error() string {
	switch e.Kind {
	case "invalid_format":
		return "invalid PCM format: " + e.Message
	case "invalid_samples":
		return "invalid PCM samples: " + e.Message
	case "unsupported_platform":
		return "unsupported platform: " + e.Message
	case "backend_unavailable":
		return "audio backend unavailable: " + e.Message
	case "backend_failure":
		return "audio backend failure: " + e.Message
	default:
		return e.Message
	}
}

func InvalidFormat(message string) AudioSinkError {
	return AudioSinkError{Kind: "invalid_format", Message: message}
}

func InvalidSamples(message string) AudioSinkError {
	return AudioSinkError{Kind: "invalid_samples", Message: message}
}

func UnsupportedPlatform(message string) AudioSinkError {
	return AudioSinkError{Kind: "unsupported_platform", Message: message}
}

func BackendUnavailable(message string) AudioSinkError {
	return AudioSinkError{Kind: "backend_unavailable", Message: message}
}

func BackendFailure(message string) AudioSinkError {
	return AudioSinkError{Kind: "backend_failure", Message: message}
}
