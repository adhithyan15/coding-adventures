package oscillator

import (
	"fmt"
	"math"
	"strings"
	"testing"
)

const absTolerance = 1e-9

func approxEqual(got float64, want float64) bool {
	return math.Abs(got-want) <= absTolerance
}

func requireApprox(t *testing.T, got float64, want float64) {
	t.Helper()
	if !approxEqual(got, want) {
		t.Fatalf("got %.12f, want %.12f", got, want)
	}
}

func requireNoError(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
}

func TestSineOscillatorDefaultParityVector(t *testing.T) {
	signal, err := NewSineOscillator(1.0)
	requireNoError(t, err)

	cases := map[float64]float64{
		0.00: 0.0,
		0.25: 1.0,
		0.50: 0.0,
		0.75: -1.0,
		1.00: 0.0,
	}
	for time, want := range cases {
		got, err := signal.ValueAt(time)
		requireNoError(t, err)
		requireApprox(t, got, want)
	}
}

func TestSineOscillatorAmplitudeAndOffsetParityVector(t *testing.T) {
	signal, err := NewSineOscillatorWithOptions(1.0, 2.0, 0.0, 3.0)
	requireNoError(t, err)

	cases := map[float64]float64{
		0.00: 3.0,
		0.25: 5.0,
		0.75: 1.0,
	}
	for time, want := range cases {
		got, err := signal.ValueAt(time)
		requireNoError(t, err)
		requireApprox(t, got, want)
	}
}

func TestSineOscillatorPhaseCyclesParityVector(t *testing.T) {
	signal, err := NewSineOscillatorWithOptions(1.0, 1.0, 0.25, 0.0)
	requireNoError(t, err)

	got, err := signal.ValueAt(0.0)
	requireNoError(t, err)
	requireApprox(t, got, 1.0)

	got, err = signal.ValueAt(0.25)
	requireNoError(t, err)
	requireApprox(t, got, 0.0)
}

func TestSineOscillatorZeroFrequencyIsConstantFromInitialPhase(t *testing.T) {
	signal, err := NewSineOscillatorWithOptions(0.0, 2.0, 0.25, 3.0)
	requireNoError(t, err)

	first, err := signal.ValueAt(0.0)
	requireNoError(t, err)
	second, err := signal.ValueAt(123.456)
	requireNoError(t, err)

	requireApprox(t, first, 5.0)
	requireApprox(t, second, 5.0)
}

func TestSineOscillatorAllowsNegativeTime(t *testing.T) {
	signal, err := NewSineOscillator(1.0)
	requireNoError(t, err)

	got, err := signal.ValueAt(-0.25)
	requireNoError(t, err)
	requireApprox(t, got, -1.0)
}

func TestSquareOscillatorParityVector(t *testing.T) {
	signal, err := NewSquareOscillatorWithOptions(2.0, 0.0, 1.0, 0.5, 0.0)
	requireNoError(t, err)

	cases := map[float64]float64{
		0.000: 1.0,
		0.125: 1.0,
		0.250: 0.0,
		0.375: 0.0,
		0.500: 1.0,
	}
	for time, want := range cases {
		got, err := signal.ValueAt(time)
		requireNoError(t, err)
		if got != want {
			t.Fatalf("at time %.3f got %.1f, want %.1f", time, got, want)
		}
	}
}

func TestSquareOscillatorNegativeTimeUsesPortableFractionalPart(t *testing.T) {
	signal, err := NewSquareOscillatorWithOptions(1.0, 0.0, 1.0, 0.5, 0.0)
	requireNoError(t, err)

	got, err := signal.ValueAt(-0.25)
	requireNoError(t, err)
	if got != 0.0 {
		t.Fatalf("got %.1f, want 0.0", got)
	}

	got, err = signal.ValueAt(-0.75)
	requireNoError(t, err)
	if got != 1.0 {
		t.Fatalf("got %.1f, want 1.0", got)
	}
}

func TestSquareOscillatorPhaseCyclesShiftsWave(t *testing.T) {
	signal, err := NewSquareOscillatorWithOptions(1.0, -1.0, 1.0, 0.5, 0.5)
	requireNoError(t, err)

	got, err := signal.ValueAt(0.0)
	requireNoError(t, err)
	if got != -1.0 {
		t.Fatalf("got %.1f, want -1.0", got)
	}

	got, err = signal.ValueAt(0.5)
	requireNoError(t, err)
	if got != 1.0 {
		t.Fatalf("got %.1f, want 1.0", got)
	}
}

func TestSquareOscillatorZeroFrequencyUsesInitialPhase(t *testing.T) {
	highSignal, err := NewSquareOscillatorWithOptions(0.0, 0.0, 1.0, 0.5, 0.25)
	requireNoError(t, err)
	lowSignal, err := NewSquareOscillatorWithOptions(0.0, 0.0, 1.0, 0.5, 0.75)
	requireNoError(t, err)

	high, err := highSignal.ValueAt(999.0)
	requireNoError(t, err)
	low, err := lowSignal.ValueAt(999.0)
	requireNoError(t, err)

	if high != 1.0 || low != 0.0 {
		t.Fatalf("got high %.1f and low %.1f", high, low)
	}
}

func TestNewSquareOscillatorUsesDefaultShape(t *testing.T) {
	signal, err := NewSquareOscillator(1.0)
	requireNoError(t, err)

	if signal.Low != -1.0 || signal.High != 1.0 || signal.DutyCycle != 0.5 {
		t.Fatalf("unexpected defaults: %#v", signal)
	}
}

func TestUniformSamplerParityVector(t *testing.T) {
	signal, err := NewSineOscillator(1.0)
	requireNoError(t, err)
	sampler, err := NewUniformSampler(4.0)
	requireNoError(t, err)

	buffer, err := sampler.Sample(signal, 1.0)
	requireNoError(t, err)

	wantTimes := []float64{0.0, 0.25, 0.5, 0.75}
	for index, want := range wantTimes {
		got, err := buffer.TimeAt(index)
		requireNoError(t, err)
		requireApprox(t, got, want)
	}

	wantValues := []float64{0.0, 1.0, 0.0, -1.0}
	for index, want := range wantValues {
		requireApprox(t, buffer.Samples[index], want)
	}

	if buffer.SampleCount() != 4 {
		t.Fatalf("got sample count %d, want 4", buffer.SampleCount())
	}
	requireApprox(t, buffer.SamplePeriodSeconds(), 0.25)
	requireApprox(t, buffer.DurationSeconds(), 1.0)
	requireApprox(t, sampler.NyquistFrequency(), 2.0)
}

func TestSamplerStartTimeOffsetsGrid(t *testing.T) {
	signal, err := NewSineOscillator(1.0)
	requireNoError(t, err)
	sampler, err := NewUniformSampler(4.0)
	requireNoError(t, err)

	buffer, err := sampler.SampleFrom(signal, 0.5, 0.25)
	requireNoError(t, err)

	requireApprox(t, buffer.Samples[0], 1.0)
	requireApprox(t, buffer.Samples[1], 0.0)

	firstTime, err := buffer.TimeAt(0)
	requireNoError(t, err)
	secondTime, err := buffer.TimeAt(1)
	requireNoError(t, err)
	requireApprox(t, firstTime, 0.25)
	requireApprox(t, secondTime, 0.5)
}

func TestSamplerExplicitCountMatchesSampleValues(t *testing.T) {
	signal, err := NewSineOscillator(1.0)
	requireNoError(t, err)
	sampler, err := NewUniformSampler(4.0)
	requireNoError(t, err)

	buffer, err := sampler.SampleCount(signal, 3, 0.0)
	requireNoError(t, err)
	values, err := sampler.SampleValues(signal, 3, 0.0)
	requireNoError(t, err)

	for index, value := range values {
		requireApprox(t, buffer.Samples[index], value)
	}
	requireApprox(t, buffer.Samples[0], 0.0)
	requireApprox(t, buffer.Samples[1], 1.0)
	requireApprox(t, buffer.Samples[2], 0.0)
}

func TestSamplerZeroDurationProducesEmptyBuffer(t *testing.T) {
	signal, err := NewSineOscillator(1.0)
	requireNoError(t, err)
	sampler, err := NewUniformSampler(44100.0)
	requireNoError(t, err)

	buffer, err := sampler.Sample(signal, 0.0)
	requireNoError(t, err)

	if len(buffer.Samples) != 0 || buffer.SampleCount() != 0 {
		t.Fatalf("expected empty buffer, got %#v", buffer.Samples)
	}
	requireApprox(t, buffer.DurationSeconds(), 0.0)
}

func TestSampleBufferDerivesMetadata(t *testing.T) {
	buffer, err := NewSampleBuffer([]float64{0.0, 1.0, 0.0}, 2.0, 10.0)
	requireNoError(t, err)

	if buffer.SampleCount() != 3 {
		t.Fatalf("got sample count %d, want 3", buffer.SampleCount())
	}
	requireApprox(t, buffer.SamplePeriodSeconds(), 0.5)
	requireApprox(t, buffer.DurationSeconds(), 1.5)

	time, err := buffer.TimeAt(2)
	requireNoError(t, err)
	requireApprox(t, time, 11.0)
}

func TestSampleBufferCopiesValues(t *testing.T) {
	input := []float64{0.0, 1.0}
	buffer, err := NewSampleBuffer(input, 1.0, 0.0)
	requireNoError(t, err)

	input[0] = 99.0
	if buffer.Samples[0] != 0.0 {
		t.Fatalf("buffer aliased input slice")
	}

	values := buffer.Values()
	values[1] = 99.0
	if buffer.Samples[1] != 1.0 {
		t.Fatalf("Values aliased buffer slice")
	}
}

func TestSampleCountForDuration(t *testing.T) {
	cases := []struct {
		duration   float64
		sampleRate float64
		want       int
	}{
		{1.0, 44100.0, 44100},
		{0.5, 48000.0, 24000},
		{0.01, 48000.0, 480},
		{0.0, 44100.0, 0},
		{1.0 / 3.0, 10.0, 3},
	}

	for _, test := range cases {
		got, err := SampleCountForDuration(test.duration, test.sampleRate)
		requireNoError(t, err)
		if got != test.want {
			t.Fatalf("got %d, want %d", got, test.want)
		}
	}
}

func TestSampleCountToleratesNearIntegerProducts(t *testing.T) {
	got, err := SampleCountForDuration(0.1+0.2, 1600.0)
	requireNoError(t, err)
	if got != 480 {
		t.Fatalf("got %d, want 480", got)
	}
}

func TestTimeAtSample(t *testing.T) {
	time, err := TimeAtSample(3, 4.0, 0.0)
	requireNoError(t, err)
	requireApprox(t, time, 0.75)

	time, err = TimeAtSample(3, 4.0, 10.0)
	requireNoError(t, err)
	requireApprox(t, time, 10.75)
}

func TestNyquistFrequency(t *testing.T) {
	got, err := NyquistFrequency(44100.0)
	requireNoError(t, err)
	requireApprox(t, got, 22050.0)
}

func TestSineRejectsInvalidParameters(t *testing.T) {
	cases := [][]float64{
		{-1.0, 1.0, 0.0, 0.0},
		{math.NaN(), 1.0, 0.0, 0.0},
		{math.Inf(1), 1.0, 0.0, 0.0},
		{1.0, -1.0, 0.0, 0.0},
		{1.0, 1.0, math.NaN(), 0.0},
		{1.0, 1.0, 0.0, math.Inf(1)},
	}

	for _, test := range cases {
		if _, err := NewSineOscillatorWithOptions(test[0], test[1], test[2], test[3]); err == nil {
			t.Fatalf("expected invalid sine parameters %#v to fail", test)
		}
	}
}

func TestValueAtRejectsInvalidTime(t *testing.T) {
	sine, err := NewSineOscillator(1.0)
	requireNoError(t, err)
	if _, err := sine.ValueAt(math.NaN()); err == nil {
		t.Fatal("expected sine ValueAt to reject NaN")
	}

	square, err := NewSquareOscillator(1.0)
	requireNoError(t, err)
	if _, err := square.ValueAt(math.Inf(1)); err == nil {
		t.Fatal("expected square ValueAt to reject infinity")
	}
}

func TestSquareRejectsInvalidParameters(t *testing.T) {
	cases := [][]float64{
		{-1.0, 0.0, 1.0, 0.5, 0.0},
		{1.0, math.NaN(), 1.0, 0.5, 0.0},
		{1.0, 0.0, math.Inf(1), 0.5, 0.0},
		{1.0, 0.0, 1.0, 0.0, 0.0},
		{1.0, 0.0, 1.0, 1.0, 0.0},
		{1.0, 0.0, 1.0, math.NaN(), 0.0},
		{1.0, 0.0, 1.0, 0.5, math.Inf(1)},
	}

	for _, test := range cases {
		if _, err := NewSquareOscillatorWithOptions(test[0], test[1], test[2], test[3], test[4]); err == nil {
			t.Fatalf("expected invalid square parameters %#v to fail", test)
		}
	}
}

func TestSamplerRejectsInvalidSampleRates(t *testing.T) {
	for _, sampleRate := range []float64{0.0, -1.0, math.NaN(), math.Inf(1)} {
		if _, err := NewUniformSampler(sampleRate); err == nil {
			t.Fatalf("expected sample rate %v to fail", sampleRate)
		}
	}
}

func TestHelpersRejectInvalidInputs(t *testing.T) {
	if _, err := SampleCountForDuration(-1.0, 4.0); err == nil {
		t.Fatal("expected negative duration to fail")
	}
	if _, err := SampleCountForDuration(math.NaN(), 4.0); err == nil {
		t.Fatal("expected NaN duration to fail")
	}
	if _, err := SampleCountForDuration(1.0, 0.0); err == nil {
		t.Fatal("expected zero sample rate to fail")
	}
	if _, err := TimeAtSample(-1, 4.0, 0.0); err == nil {
		t.Fatal("expected negative index to fail")
	}
	if _, err := TimeAtSample(1, math.Inf(1), 0.0); err == nil {
		t.Fatal("expected infinite sample rate to fail")
	}
	if _, err := NyquistFrequency(0.0); err == nil {
		t.Fatal("expected zero sample rate to fail")
	}
}

func TestSampleBufferRejectsInvalidValues(t *testing.T) {
	if _, err := NewSampleBuffer([]float64{0.0, math.NaN()}, 1.0, 0.0); err == nil {
		t.Fatal("expected NaN sample to fail")
	} else if !strings.Contains(err.Error(), "samples[1]") {
		t.Fatalf("expected sample index in error, got %v", err)
	}
	if _, err := NewSampleBuffer([]float64{0.0}, 0.0, 0.0); err == nil {
		t.Fatal("expected zero sample rate to fail")
	}
	if _, err := NewSampleBuffer([]float64{0.0}, 1.0, math.Inf(1)); err == nil {
		t.Fatal("expected infinite start time to fail")
	}
}

func TestSampleBufferTimeAtRejectsInvalidIndex(t *testing.T) {
	buffer, err := NewSampleBuffer([]float64{0.0, 1.0}, 2.0, 0.0)
	requireNoError(t, err)

	for _, index := range []int{-1, 2} {
		if _, err := buffer.TimeAt(index); err == nil {
			t.Fatalf("expected index %d to fail", index)
		}
	}
}

func TestSamplerRejectsInvalidExplicitCountAndStart(t *testing.T) {
	signal, err := NewSineOscillator(1.0)
	requireNoError(t, err)
	sampler, err := NewUniformSampler(4.0)
	requireNoError(t, err)

	if _, err := sampler.SampleValues(signal, -1, 0.0); err == nil {
		t.Fatal("expected negative sample count to fail")
	}
	if _, err := sampler.SampleValues(signal, 1, math.NaN()); err == nil {
		t.Fatal("expected NaN start time to fail")
	}
}

type failingSignal struct{}

func (failingSignal) ValueAt(float64) (float64, error) {
	return 0, fmt.Errorf("boom")
}

func TestSamplerPropagatesSignalErrors(t *testing.T) {
	sampler, err := NewUniformSampler(4.0)
	requireNoError(t, err)

	if _, err := sampler.SampleValues(failingSignal{}, 1, 0.0); err == nil {
		t.Fatal("expected signal error to propagate")
	}
}
