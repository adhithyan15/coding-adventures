package audiodevicesink

import (
	"strings"
	"testing"
)

func TestValidFormatRecordsPCMMetadata(t *testing.T) {
	format, err := NewPcmFormat(44100, 1, 16)
	if err != nil {
		t.Fatalf("format should be valid: %v", err)
	}

	if Version != "0.1.0" || format.SampleRateHz != 44100 || format.SampleWidthBytes() != 2 {
		t.Fatalf("unexpected format metadata: %#v", format)
	}
}

func TestFormatRejectsInvalidValues(t *testing.T) {
	cases := []struct {
		name string
		err  error
		want string
	}{
		{"zero rate", func() error { _, err := NewPcmFormat(0, 1, 16); return err }(), "sample_rate_hz"},
		{"high rate", func() error { _, err := NewPcmFormat(MaxSampleRateHz+1, 1, 16); return err }(), "384000"},
		{"channels", func() error { _, err := NewPcmFormat(44100, 2, 16); return err }(), "mono"},
		{"bits", func() error { _, err := NewPcmFormat(44100, 1, 24); return err }(), "16-bit"},
	}

	for _, tc := range cases {
		if tc.err == nil || !strings.Contains(tc.err.Error(), tc.want) {
			t.Fatalf("%s error = %v, want %q", tc.name, tc.err, tc.want)
		}
	}
}

func TestBufferReportsCountsAndDuration(t *testing.T) {
	format, _ := NewPcmFormat(4, 1, 16)
	buffer, err := NewPcmPlaybackBuffer([]int16{0, 1000, 0, -1000}, format)
	if err != nil {
		t.Fatalf("buffer should be valid: %v", err)
	}

	if buffer.SampleCount() != 4 || buffer.FrameCount() != 4 || buffer.DurationSeconds() != 1 || buffer.IsEmpty() {
		t.Fatalf("unexpected buffer metadata")
	}
	if len(buffer.Samples()) != 4 || buffer.Format() != format {
		t.Fatalf("buffer accessors failed")
	}
}

func TestBufferRejectsTooLongPayloads(t *testing.T) {
	format, _ := NewPcmFormat(1, 1, 16)
	_, err := NewPcmPlaybackBuffer(make([]int16, int(MaxBlockingDurationSeconds)+1), format)
	if err == nil || !strings.Contains(err.Error(), "limited") {
		t.Fatalf("expected length error, got %v", err)
	}
}

func TestNoopSinkReturnsReport(t *testing.T) {
	format, _ := NewPcmFormat(44100, 1, 16)
	buffer, _ := NewPcmPlaybackBuffer([]int16{0, 1000}, format)
	report, err := NewNoopAudioSink("test-noop").PlayBlocking(buffer)
	if err != nil {
		t.Fatalf("noop sink failed: %v", err)
	}

	if report.FramesPlayed != 2 || report.SampleRateHz != 44100 || report.BackendName != "test-noop" {
		t.Fatalf("unexpected report: %#v", report)
	}
}

func TestEmptyBuffersAndErrorStrings(t *testing.T) {
	format, _ := NewPcmFormat(44100, 1, 16)
	buffer, _ := NewPcmPlaybackBuffer(nil, format)
	report, _ := NewNoopAudioSink("empty").PlayBlocking(buffer)

	if !buffer.IsEmpty() || report.DurationSeconds != 0 {
		t.Fatalf("empty buffer report failed")
	}
	if BackendUnavailable("default device missing").Error() != "audio backend unavailable: default device missing" {
		t.Fatalf("backend unavailable string mismatch")
	}
	if UnsupportedPlatform("Core Audio requires macOS").Error() != "unsupported platform: Core Audio requires macOS" {
		t.Fatalf("unsupported platform string mismatch")
	}
	if BackendFailure("write failed").Error() != "audio backend failure: write failed" {
		t.Fatalf("backend failure string mismatch")
	}
}
