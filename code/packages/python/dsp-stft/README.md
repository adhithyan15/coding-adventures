# coding-adventures-dsp-stft

Pure Python Short-Time Fourier Transform (STFT), ISTFT with NOLA/COLA reconstruction, power and log spectrograms, mel filterbanks, and MFCCs.

## Features

- **Window Functions**: Rectangular, Hann, Hamming, and Blackman windows.
- **Forward STFT**: Sliding-window FFT with strict framing producing interleaved complex `[num_frames, n_fft // 2 + 1, 2]` spectra.
- **Inverse STFT (ISTFT)**: Overlap-add (OLA) reconstruction with COLA/NOLA normalisation guaranteeing exact recovery up to FP precision.
- **Spectrograms**: Power spectrogram $|STFT|^2$, magnitude spectrogram, and log-power spectrogram.
- **Mel Scale & Filterbank**: HTK-convention `hz_to_mel`, `mel_to_hz`, and triangular mel filterbanks.
- **Mel Spectrogram & MFCC**: Perceptual mel-power pooling and DCT-II cepstral compression.
- **VER15 Security Hardening**: Pre-flight bounds (`MAX_FFT_LEN`, `MAX_SAMPLES`, `MAX_FRAMES`, `MAX_MELS`), CWE-20 finite-float checks, CWE-674 iterative execution, CWE-1333 zero regex imports, and pure zero-capability sandbox compliance.
