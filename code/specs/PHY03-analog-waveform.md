# PHY03: Analog Waveform

## Overview

The existing `wave` package models one specific analog waveform: a sinusoid.

For the electronics path, it helps to make that idea more explicit:

- `analog-waveform` is the general concept of a continuous-time signal
- `wave` can remain as the original introductory sinusoid package

This avoids a breaking rename while giving the repo a cleaner path toward:

- sampled signals
- DAC reconstruction
- circuit simulation
- transient analysis

## Core abstraction

An analog waveform is any function of time:

$$x(t)$$

Examples:

- constant DC voltage
- sinusoidal AC voltage
- exponentially decaying capacitor voltage
- piecewise-linear switching waveform

## Initial scope

The first package should include:

- `AnalogWaveform` interface with `sampleAt(t)`
- `ConstantWaveform`
- `SineWaveform`

## Why this matters

SPICE-style simulation is fundamentally about solving for node voltages and
branch currents as functions of time. So we want a clean home for
continuous-time signals before introducing sampled or discretized ones.
