# PHY04: Discrete Waveform

## Overview

A discrete waveform is a time series of samples taken from an analog waveform.

This is the natural bridge between:

- analog signals
- DSP intuition
- DAC and ADC behavior
- numerical simulation

## Model

A discrete waveform is defined by:

- an array of samples
- a sample rate

From those, we derive:

- sample period
- duration
- zero-order-hold reconstruction

## Why this matters

DACs do not usually output a perfectly smooth analog curve directly. They
produce discrete levels that are held for finite time intervals. That stepped
output is then smoothed by downstream filtering.
