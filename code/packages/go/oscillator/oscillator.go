// Package oscillator models virtual continuous oscillators and uniform samplers.
//
// An oscillator here is not a sound card, timer, pin, DAC, or radio. It is a
// pure mathematical signal: given a time in seconds, it returns the value the
// signal would have at that instant.
//
// A sampler sits one layer above that. It chooses concrete times, asks the
// signal for values, and stores those values in a SampleBuffer. That is the
// bridge from a smooth virtual signal to the stream of numbers that later
// audio, radio, DAC, or clock-edge packages can consume.
package oscillator

import (
	"fmt"
	"math"

	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

const (
	twoPI            = 2.0 * trig.PI
	integerTolerance = 1e-9
)

// ContinuousSignal is anything that can report its value at a time in seconds.
//
// ValueAt returns an error for non-finite times instead of silently producing a
// NaN. That keeps the Go implementation aligned with OSC00's rule that invalid
// input must be rejected explicitly.
type ContinuousSignal interface {
	ValueAt(timeSeconds float64) (float64, error)
}

// finiteFloat validates the "finite real number" rule shared by every public
// constructor and helper in this package.
func finiteFloat(name string, value float64) (float64, error) {
	if math.IsNaN(value) || math.IsInf(value, 0) {
		return 0, fmt.Errorf("%s must be finite, got %v", name, value)
	}
	return value, nil
}

func nonNegativeFloat(name string, value float64) (float64, error) {
	converted, err := finiteFloat(name, value)
	if err != nil {
		return 0, err
	}
	if converted < 0.0 {
		return 0, fmt.Errorf("%s must be >= 0.0, got %v", name, converted)
	}
	return converted, nil
}

func positiveFloat(name string, value float64) (float64, error) {
	converted, err := finiteFloat(name, value)
	if err != nil {
		return 0, err
	}
	if converted <= 0.0 {
		return 0, fmt.Errorf("%s must be > 0.0, got %v", name, converted)
	}
	return converted, nil
}

// fractionalPart returns the fractional part in [0.0, 1.0), even for negative
// inputs. This mirrors the portable OSC00 definition:
//
//	fractional_part(x) = x - floor(x)
func fractionalPart(value float64) float64 {
	return value - math.Floor(value)
}

// NyquistFrequency returns half the sample rate, the highest cleanly
// representable frequency for an ideal uniform sampler.
func NyquistFrequency(sampleRateHz float64) (float64, error) {
	sampleRate, err := positiveFloat("sample_rate_hz", sampleRateHz)
	if err != nil {
		return 0, err
	}
	return sampleRate / 2.0, nil
}

// SampleCountForDuration returns the default sample count for a half-open
// sampling interval.
//
// Mathematically this is:
//
//	floor(duration_seconds * sample_rate_hz)
//
// The tiny integer tolerance prevents floating-point spelling accidents such as
// 479.99999999999994 becoming 479 when the intended mathematical product is 480.
func SampleCountForDuration(durationSeconds float64, sampleRateHz float64) (int, error) {
	duration, err := nonNegativeFloat("duration_seconds", durationSeconds)
	if err != nil {
		return 0, err
	}
	sampleRate, err := positiveFloat("sample_rate_hz", sampleRateHz)
	if err != nil {
		return 0, err
	}

	rawCount := duration * sampleRate
	nearestInteger := math.Round(rawCount)
	if math.Abs(rawCount-nearestInteger) <= integerTolerance {
		return int(nearestInteger), nil
	}
	return int(math.Floor(rawCount)), nil
}

// TimeAtSample returns the time for sample index n on a uniform sample grid.
func TimeAtSample(index int, sampleRateHz float64, startTimeSeconds float64) (float64, error) {
	if index < 0 {
		return 0, fmt.Errorf("index must be >= 0, got %d", index)
	}
	sampleRate, err := positiveFloat("sample_rate_hz", sampleRateHz)
	if err != nil {
		return 0, err
	}
	startTime, err := finiteFloat("start_time_seconds", startTimeSeconds)
	if err != nil {
		return 0, err
	}
	return startTime + float64(index)/sampleRate, nil
}

// SineOscillator is the smoothest basic oscillator.
//
// A 1 Hz default sine oscillator starts at zero, reaches its peak at 0.25
// seconds, crosses zero at 0.5 seconds, and reaches its trough at 0.75 seconds.
// Increasing the frequency squeezes more cycles into each second.
type SineOscillator struct {
	FrequencyHz float64
	Amplitude   float64
	PhaseCycles float64
	Offset      float64
}

// NewSineOscillator creates a sine oscillator with unit amplitude, zero phase,
// and zero offset.
func NewSineOscillator(frequencyHz float64) (SineOscillator, error) {
	return NewSineOscillatorWithOptions(frequencyHz, 1.0, 0.0, 0.0)
}

// NewSineOscillatorWithOptions creates a sine oscillator with explicit
// amplitude, phase in cycles, and offset.
func NewSineOscillatorWithOptions(
	frequencyHz float64,
	amplitude float64,
	phaseCycles float64,
	offset float64,
) (SineOscillator, error) {
	frequency, err := nonNegativeFloat("frequency_hz", frequencyHz)
	if err != nil {
		return SineOscillator{}, err
	}
	amp, err := nonNegativeFloat("amplitude", amplitude)
	if err != nil {
		return SineOscillator{}, err
	}
	phase, err := finiteFloat("phase_cycles", phaseCycles)
	if err != nil {
		return SineOscillator{}, err
	}
	center, err := finiteFloat("offset", offset)
	if err != nil {
		return SineOscillator{}, err
	}

	return SineOscillator{
		FrequencyHz: frequency,
		Amplitude:   amp,
		PhaseCycles: phase,
		Offset:      center,
	}, nil
}

// ValueAt evaluates:
//
//	offset + amplitude * sin(2*pi*(frequency_hz*time_seconds + phase_cycles))
func (s SineOscillator) ValueAt(timeSeconds float64) (float64, error) {
	time, err := finiteFloat("time_seconds", timeSeconds)
	if err != nil {
		return 0, err
	}
	phase := s.FrequencyHz*time + s.PhaseCycles
	return s.Offset + s.Amplitude*trig.Sin(twoPI*phase), nil
}

// SquareOscillator switches between high and low values.
//
// This is the oscillator shape underneath digital-looking signals. A clock
// package can hide it internally and expose friendly ClockEdge records to CPU,
// bus, or flip-flop consumers.
type SquareOscillator struct {
	FrequencyHz float64
	Low         float64
	High        float64
	DutyCycle   float64
	PhaseCycles float64
}

// NewSquareOscillator creates a square oscillator with low=-1, high=1, a 50%
// duty cycle, and zero phase.
func NewSquareOscillator(frequencyHz float64) (SquareOscillator, error) {
	return NewSquareOscillatorWithOptions(frequencyHz, -1.0, 1.0, 0.5, 0.0)
}

// NewSquareOscillatorWithOptions creates a square oscillator with explicit
// levels, duty cycle, and phase.
func NewSquareOscillatorWithOptions(
	frequencyHz float64,
	low float64,
	high float64,
	dutyCycle float64,
	phaseCycles float64,
) (SquareOscillator, error) {
	frequency, err := nonNegativeFloat("frequency_hz", frequencyHz)
	if err != nil {
		return SquareOscillator{}, err
	}
	lowValue, err := finiteFloat("low", low)
	if err != nil {
		return SquareOscillator{}, err
	}
	highValue, err := finiteFloat("high", high)
	if err != nil {
		return SquareOscillator{}, err
	}
	duty, err := finiteFloat("duty_cycle", dutyCycle)
	if err != nil {
		return SquareOscillator{}, err
	}
	if duty <= 0.0 || duty >= 1.0 {
		return SquareOscillator{}, fmt.Errorf(
			"duty_cycle must satisfy 0.0 < duty_cycle < 1.0, got %v",
			duty,
		)
	}
	phase, err := finiteFloat("phase_cycles", phaseCycles)
	if err != nil {
		return SquareOscillator{}, err
	}

	return SquareOscillator{
		FrequencyHz: frequency,
		Low:         lowValue,
		High:        highValue,
		DutyCycle:   duty,
		PhaseCycles: phase,
	}, nil
}

// ValueAt returns High during the duty-cycle window and Low otherwise.
func (s SquareOscillator) ValueAt(timeSeconds float64) (float64, error) {
	time, err := finiteFloat("time_seconds", timeSeconds)
	if err != nil {
		return 0, err
	}
	position := fractionalPart(s.FrequencyHz*time + s.PhaseCycles)
	if position < s.DutyCycle {
		return s.High, nil
	}
	return s.Low, nil
}

// SampleBuffer stores sampled values and enough timing metadata to interpret
// them.
type SampleBuffer struct {
	Samples          []float64
	SampleRateHz     float64
	StartTimeSeconds float64
}

// NewSampleBuffer validates sample values and copies them into an immutable-ish
// package-owned slice. Go slices are mutable by nature, but copying on input and
// output prevents accidental aliasing with caller-owned buffers.
func NewSampleBuffer(
	samples []float64,
	sampleRateHz float64,
	startTimeSeconds float64,
) (SampleBuffer, error) {
	copied := make([]float64, len(samples))
	for index, sample := range samples {
		value, err := finiteFloat(fmt.Sprintf("samples[%d]", index), sample)
		if err != nil {
			return SampleBuffer{}, err
		}
		copied[index] = value
	}

	sampleRate, err := positiveFloat("sample_rate_hz", sampleRateHz)
	if err != nil {
		return SampleBuffer{}, err
	}
	startTime, err := finiteFloat("start_time_seconds", startTimeSeconds)
	if err != nil {
		return SampleBuffer{}, err
	}

	return SampleBuffer{
		Samples:          copied,
		SampleRateHz:     sampleRate,
		StartTimeSeconds: startTime,
	}, nil
}

// Values returns a copy of the stored sample values.
func (b SampleBuffer) Values() []float64 {
	copied := make([]float64, len(b.Samples))
	copy(copied, b.Samples)
	return copied
}

// SampleCount returns the number of stored samples.
func (b SampleBuffer) SampleCount() int {
	return len(b.Samples)
}

// SamplePeriodSeconds returns the spacing between adjacent samples.
func (b SampleBuffer) SamplePeriodSeconds() float64 {
	return 1.0 / b.SampleRateHz
}

// DurationSeconds returns the covered duration of this half-open sample buffer.
func (b SampleBuffer) DurationSeconds() float64 {
	return float64(b.SampleCount()) / b.SampleRateHz
}

// TimeAt returns the time associated with a stored sample index.
func (b SampleBuffer) TimeAt(index int) (float64, error) {
	if index < 0 || index >= b.SampleCount() {
		return 0, fmt.Errorf("index must be in [0, %d), got %d", b.SampleCount(), index)
	}
	return TimeAtSample(index, b.SampleRateHz, b.StartTimeSeconds)
}

// UniformSampler evaluates a signal at evenly spaced times.
type UniformSampler struct {
	SampleRateHz float64
}

// NewUniformSampler creates a sampler with the given samples-per-second rate.
func NewUniformSampler(sampleRateHz float64) (UniformSampler, error) {
	sampleRate, err := positiveFloat("sample_rate_hz", sampleRateHz)
	if err != nil {
		return UniformSampler{}, err
	}
	return UniformSampler{SampleRateHz: sampleRate}, nil
}

// Sample samples signal over [0, durationSeconds).
func (s UniformSampler) Sample(
	signal ContinuousSignal,
	durationSeconds float64,
) (SampleBuffer, error) {
	return s.SampleFrom(signal, durationSeconds, 0.0)
}

// SampleFrom samples signal over [startTimeSeconds, endTimeSeconds).
func (s UniformSampler) SampleFrom(
	signal ContinuousSignal,
	durationSeconds float64,
	startTimeSeconds float64,
) (SampleBuffer, error) {
	count, err := SampleCountForDuration(durationSeconds, s.SampleRateHz)
	if err != nil {
		return SampleBuffer{}, err
	}
	return s.SampleCount(signal, count, startTimeSeconds)
}

// SampleCount samples exactly sampleCount values from signal.
func (s UniformSampler) SampleCount(
	signal ContinuousSignal,
	sampleCount int,
	startTimeSeconds float64,
) (SampleBuffer, error) {
	values, err := s.SampleValues(signal, sampleCount, startTimeSeconds)
	if err != nil {
		return SampleBuffer{}, err
	}
	return NewSampleBuffer(values, s.SampleRateHz, startTimeSeconds)
}

// SampleValues returns the same values that SampleCount would store in a
// SampleBuffer, without attaching metadata.
func (s UniformSampler) SampleValues(
	signal ContinuousSignal,
	sampleCount int,
	startTimeSeconds float64,
) ([]float64, error) {
	if sampleCount < 0 {
		return nil, fmt.Errorf("sample_count must be >= 0, got %d", sampleCount)
	}
	startTime, err := finiteFloat("start_time_seconds", startTimeSeconds)
	if err != nil {
		return nil, err
	}

	values := make([]float64, sampleCount)
	for index := 0; index < sampleCount; index++ {
		time, err := TimeAtSample(index, s.SampleRateHz, startTime)
		if err != nil {
			return nil, err
		}
		value, err := signal.ValueAt(time)
		if err != nil {
			return nil, err
		}
		values[index] = value
	}
	return values, nil
}

// NyquistFrequency returns this sampler's Nyquist frequency.
func (s UniformSampler) NyquistFrequency() float64 {
	return s.SampleRateHz / 2.0
}
