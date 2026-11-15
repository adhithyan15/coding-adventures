# @coding-adventures/music-machine-web

Browser-safe deterministic sheet music parser, multi-track synthesizer, and Web Audio playback.

## Features

- **Text Score Parser**: Beginner-friendly sheet music parser (MUS01/MUS03) with linear-time directive and token parsing.
- **Portable Score Parser & Formatter**: Canonical `music-machine-score/v2` multi-track format (MUS04).
- **Deterministic JSON**: Bounded-depth JSON parser and deterministic sorted-key JSON serializer.
- **Multi-Track Synthesizer**: Polyphonic note synthesis with additive overtones, ADSR loudness envelopes, and General MIDI mapping.
- **Percussion Kit & Hi-Hat Choking**: Physical acoustic drums with hi-hat choke dynamics (MUS07).
- **Web Audio Sink**: Direct playback through browser `AudioContext` and gain routing.
- **WAV Generation**: Deterministic 16-bit mono RIFF/WAVE export.
- **VER15 Hardened**: Comprehensive proof against CWE-20, CWE-190, CWE-400, CWE-674, CWE-1321, and CWE-1333.
