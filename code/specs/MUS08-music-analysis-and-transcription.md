# MUS08: Music Understanding, Analysis, and Sample-to-Score Transcription

## 1. Overview

Specifications `MUS00` through `MUS07` define the forward synthesis stack:

```text
note name -> frequency (MUS00)
frequency -> sound wave (MUS01)
text score -> multi-track events (MUS03, MUS04)
instrument profile -> shaped harmonics & envelope (MUS05)
drum hit -> excitation & resonance (MUS07)
```

In every forward stage, structured symbolic information descends toward raw numerical
PCM audio samples.

`MUS08` defines the reverse direction — the **music understanding and transcription** layer:

```text
PCM audio samples -> spectral & wavelet analysis (DSP01-06)
                  -> onset detection & pitch tracking (MUS08)
                  -> note segmentation & quantization (MUS08)
                  -> PortableScore (MUS04) & InstrumentProfile (MUS05)
```

The goals of `MUS08` are:

1. **Invert Note-to-Frequency**: Convert any positive continuous frequency in Hertz
   back into its nearest equal-tempered note name, MIDI semitone number, and cents
   deviation relative to standard pitch reference ($A4 = 440$ Hz).
2. **Audio Feature Extraction**: Extract canonical time-domain and frequency-domain
   features from waveforms or Short-Time Fourier Transform spectra (spectral centroid,
   spectral flux, spectral rolloff, zero-crossing rate, RMS energy).
3. **Robust Fundamental Pitch Detection**: Estimate fundamental frequency ($f_0$)
   iteratively using both time-domain autocorrelation/difference (YIN algorithm)
   and frequency-domain harmonic compression (Harmonic Product Spectrum).
4. **Multi-Resolution Onset Detection**: Detect note onset events accurately across
   both sharp transients and soft acoustic attacks by fusing spectral flux with
   discrete wavelet transform (DWT) detail energy.
5. **Harmonic Profile & Timbre Extraction**: Analyze an acoustic or synthesized note
   sample to extract its fundamental amplitude, harmonic partial overtones, and
   four-stage ADSR amplitude envelope, producing a `MUS05` `InstrumentProfile`
   ready for re-synthesis in `musical-instruments` or `music-machine`.
6. **Sample-to-Score Transcription**: Invert the rendering pipeline by segmenting
   recorded PCM audio into discrete notes, estimating their pitches, snapping their
   timings to a rhythmic grid at a specified tempo, and generating a validated
   `MUS04` `PortableScore`.
7. **Round-Trip Verifiability**: Establish formal testable parity such that a score
   rendered via `render_portable_score_to_pcm` can be transcribed back via
   `transcribe_pcm_to_portable_score` with equivalent note sequences, pitches,
   and metric structure.

---

## 2. Beginner Mental Model: The Inverse Audio Stack

When a musician plays a song on a piano or flute:

1. The instrument vibrates, generating continuous sound pressure waves in the air.
2. An audio interface records these vibrations as thousands of numbers per second (PCM samples).

To a human listener, that stream of numbers sounds like "a flute playing C4 for one beat,
then E4 for one beat, then G4 for two beats". But to a computer program, the recording
is just a list of numbers: `[-0.012, 0.045, 0.128, ...]`.

The transcription task answers three fundamental questions:

| Question | Analysis Problem | Primary Algorithm / Feature |
|---|---|---|
| **When did notes start?** | Onset Detection | Spectral flux + Wavelet detail energy |
| **What pitches were played?** | Pitch Estimation | YIN algorithm & Harmonic Product Spectrum |
| **What was the sound color?** | Timbre Extraction | Harmonic partial series + ADSR envelope |
| **How does it fit musical time?** | Score Transcription | Grid quantization + PortableScore generation |

---

## 3. Equal-Tempered Pitch Inversion

In `MUS00`, a note string (such as `A4` or `C#5`) maps to a frequency $f$:

$$f = 440.0 \cdot 2^{(m - 69) / 12}$$

where $m$ is the standard MIDI note number ($A4 = 69$, $C4 = 60$).

`MUS08` provides the inverse mapping `frequency_to_note(hz, a4_hz=440.0)`:

1. **Exact Semitone Index**:
   $$m_{\text{exact}} = 69.0 + 12.0 \cdot \log_2\left(\frac{f}{f_{A4}}\right)$$
2. **Nearest Integer MIDI Number**:
   $$m_{\text{nearest}} = \lfloor m_{\text{exact}} + 0.5 \rfloor$$
3. **Cents Deviation**:
   $$\text{cents} = 100.0 \cdot (m_{\text{exact}} - m_{\text{nearest}})$$
   where $\text{cents} \in [-50.0, +50.0]$ indicates microtonal sharpness or flatness.
4. **Canonical Note Name**:
   Given chromatic pitch classes `("C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B")`:
   $$\text{pitch\_class} = \text{PITCH\_CLASSES}[m_{\text{nearest}} \pmod{12}]$$
   $$\text{octave} = \lfloor m_{\text{nearest}} / 12 \rfloor - 1$$
   $$\text{note\_name} = \text{pitch\_class} + \text{str}(\text{octave})$$

---

## 4. Spectral & Time-Domain Audio Features

Feature extraction transforms raw audio buffers into compact descriptive vectors:

### 4.1 Spectral Centroid
The spectral centroid represents the "center of mass" of the frequency spectrum,
strongly correlated with perceived auditory brightness:

$$\text{Centroid} = \frac{\sum_{k=0}^{K-1} f_k \cdot |X[k]|}{\sum_{k=0}^{K-1} |X[k]|}$$

where $f_k = k \cdot \frac{f_s}{2(K - 1)}$ is the frequency of bin $k$, and $|X[k]|$ is the
spectral magnitude. If total spectral energy is zero, the centroid is defined as $0.0$.

### 4.2 Spectral Flux
Spectral flux measures the rate of local spectral variation between consecutive frames,
providing a strong indicator of transient note attacks:

$$H(X_m, X_{m-1}) = \sum_{k=0}^{K-1} \max\left(0, |X_m[k]| - |X_{m-1}[k]|\right)$$

Half-wave rectification captures positive energy increases characteristic of note onsets,
suppressing energy decreases during note decay.

### 4.3 Spectral Rolloff
The spectral rolloff is the frequency $f_{k^*}$ below which a specified fraction
(default $85\%$) of the total spectral energy is contained:

$$\sum_{k=0}^{k^*} |X[k]| \ge \gamma \sum_{k=0}^{K-1} |X[k]|$$

### 4.4 Zero-Crossing Rate (ZCR)
The zero-crossing rate measures the proportion of sign transitions in a time-domain frame:

$$\text{ZCR} = \frac{1}{N - 1} \sum_{n=1}^{N-1} \mathbb{I}(\operatorname{sgn}(x[n]) \ne \operatorname{sgn}(x[n-1]))$$

High ZCR characterizes unpitched percussion, noise, and friction transients; low ZCR
characterizes smooth pitched tones.

### 4.5 Root-Mean-Square Energy (RMS)
RMS energy measures physical signal power:

$$\text{RMS} = \sqrt{\frac{1}{N} \sum_{n=0}^{N-1} x[n]^2}$$

---

## 5. Fundamental Pitch Detection

`MUS08` specifies two complementary pitch estimation methods:

### 5.1 YIN Algorithm
The YIN algorithm operates in the time domain, computing a normalized squared difference:

1. **Difference Function**:
   $$d(\tau) = \sum_{j=0}^{W - 1} (x[j] - x[j + \tau])^2$$
2. **Cumulative Mean Normalized Difference**:
   $$d'(\tau) = \begin{cases} 1 & \tau = 0 \\ \frac{d(\tau)}{\frac{1}{\tau} \sum_{j=1}^{\tau} d(j)} & \tau > 0 \end{cases}$$
3. **Absolute Thresholding**: Select the smallest lag $\tau$ within $[\tau_{\min}, \tau_{\max}]$
   where $d'(\tau) < \theta$ (typically $\theta = 0.15$), followed by local minimum descent.
4. **Parabolic Interpolation**: Refine the discrete lag estimate $\tau$ to sub-sample precision
   $\tau^* = \tau + \Delta\tau$ using quadratic peak interpolation.
5. **Pitch Output**:
   $$f_0 = \frac{f_s}{\tau^*}$$

### 5.2 Harmonic Product Spectrum (HPS)
The Harmonic Product Spectrum operates in the frequency domain, reinforcing the fundamental
by multiplying downsampled spectral magnitudes across harmonic multiples $h = 1 \dots H$:

$$P[k] = \prod_{h=1}^H |X[h \cdot k]|$$

Logarithmic accumulation $\sum_{h=1}^H \ln(|X[h \cdot k]| + \epsilon)$ prevents numeric underflow
and identifies the fundamental bin $k^*$ with maximal harmonic support.

---

## 6. Onset Detection & Adaptive Peak Picking

Accurate transcription requires detecting note beginnings without spurious triggers.
`MUS08` combines frequency-domain and time-scale representations:

1. **Spectral Flux Novelty**: Computes positive frame-to-frame magnitude flux from STFT.
2. **Wavelet Detail Energy**: Decomposes the signal via discrete wavelet transform (DWT,
   Daubechies or Haar) and extracts detail coefficients $cD$, which concentrate impulsive
   attack energy.
3. **Feature Fusion**:
   $$O[m] = 0.5 \cdot \widetilde{SF}[m] + 0.5 \cdot \widetilde{WE}[m]$$
   where each component is normalized to $[0, 1]$.
4. **Adaptive Thresholding & Peak Picking**: A moving local mean $\mu[m]$ and standard
   deviation $\sigma[m]$ define an adaptive threshold:
   $$\Theta[m] = \mu[m] + \delta \cdot \sigma[m]$$
   A frame $m$ is accepted as an onset if $O[m]$ is a local maximum exceeding $\Theta[m]$,
   subject to a refractory lockout duration (default 50 ms) preventing multi-triggering.

---

## 7. Harmonic Profile & Timbre Extraction

`extract_harmonic_profile` examines a single note sample and produces a `MUS05` `InstrumentProfile`:

1. **Fundamental Frequency ($f_0$)**: Detected via YIN or provided explicitly.
2. **Harmonic Partials**: Evaluates FFT peak magnitudes around integer multiples $h \cdot f_0$
   ($h = 1 \dots \text{max\_partials} \le 64$), normalized relative to fundamental $A_1$:
   $$a_h = \frac{A_h}{A_1}$$
   yielding a tuple of `HarmonicPartial(frequency_multiplier=h, amplitude=a_h, phase_cycles=0.0)`.
3. **ADSR Loudness Envelope**: Extracts attack duration $A$, decay duration $D$, sustain level $S$,
   and release duration $R$ from the smoothed frame energy envelope, returning an `ADSREnvelope`.
4. **Synthesis Profile**: Encapsulates the partials and envelope into an `InstrumentProfile`
   with `synthesis_kind="additive"`, enabling seamless resynthesis in `musical-instruments`.

---

## 8. Sample-to-Score Transcription

`transcribe_pcm_to_portable_score` converts audio samples into a `MUS04` `PortableScore`:

```text
PCM Samples -> Onset Detection -> Note Boundaries -> Segment Pitch & Duration -> Quantization -> PortableScore
```

1. **Segmentation**: Detected onsets partition the audio into note intervals $[t_i, t_{i+1})$.
2. **Pitch Identification**: Stable central portions of each segment are evaluated with `yin_pitch`
   and converted to equal-tempered notes with `frequency_to_note`.
3. **Rhythmic Quantization**:
   - Start ticks: $k_{\text{start}} = \operatorname{round}\left(t_i \cdot \frac{\text{BPM} \cdot \text{PPQ}}{60}\right)$
     snapped to the nearest grid increment (e.g. 16th note $= \text{PPQ} / 4$).
   - Duration ticks: $k_{\text{dur}} = \max\left(\text{grid\_step}, \operatorname{round}\left(\Delta t \cdot \frac{\text{BPM} \cdot \text{PPQ}}{60}\right)\right)$.
4. **Score Assembly**: Emits `PortableScoreEvent` items inside a multi-track score model conforming
   to `music-machine-score/v2`.

---

## 9. Security, Resource Bounds, and VER15 Assurance

The implementation satisfies strict VER15 security controls:

- **Pre-flight Bounds (CWE-400)**:
  - `MAX_SAMPLES = 16_777_216` (approx. 6.3 minutes at 44.1 kHz)
  - `MAX_FFT_LEN = 1_048_576`
  - `MAX_FRAMES = 1_048_576`
  - `MAX_PARTIALS = 64`
  - `MAX_ONSETS = 65_536`
  - `MAX_BPM = 1000.0`
  - `MAX_SAMPLE_RATE_HZ = 192_000.0`
  Every buffer and loop count is validated against these limits **before** allocating memory.
- **Fail-Closed Numeric Validation (CWE-20)**: All input floats are checked with `math.isfinite`.
  `NaN`, `+Inf`, and `-Inf` fail immediately with `InvalidInputError`. Non-positive sample rates,
  frequencies, and tempos fail closed.
- **Strictly Iterative Loops (CWE-674)**: All pitch tracking, dynamic programming, peak picking,
  and segmentation algorithms are strictly iterative with constant stack depth ($O(1)$ stack frames).
- **Zero Regex (CWE-1333)**: Note name formatting, token parsing, and identifier validation use
  pure string slicing and table lookups with zero regular expression engines.
- **Capability Conformance**: Declares pure computation (`[]` capabilities) in
  `required_capabilities.json`, enforced by static AST analysis.
