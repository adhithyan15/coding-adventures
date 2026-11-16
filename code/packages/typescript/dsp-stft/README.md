# @coding-adventures/dsp-stft

Short-Time Fourier Transform, spectrogram, and speech features (Mel filterbank, MFCC) for the DSP layer.

## Features

- Short-Time Fourier Transform (`stft`) with strict-mode framing and window functions.
- Inverse Short-Time Fourier Transform (`istft`) with overlap-add (OLA) reconstruction.
- Magnitude and power spectrograms (`spectrogram`, `powerSpectrogram`) and log spectrogram (`logSpectrogram`).
- Analysis window generators (`buildWindow`, `hannWindow`, `hammingWindow`, `blackmanWindow`, `rectangularWindow`).
- Mel scale utilities: `hzToMel` and `melToHz` (HTK convention).
- Mel filterbank (`melFilterbank`) with area-normalised triangular filters.
- Mel spectrogram (`melSpectrogram`) and Mel-Frequency Cepstral Coefficients (`mfcc`) via DCT-II.
- Hardened with VER15 CWE/CVE proofs (CWE-20, CWE-190, CWE-400, CWE-674, CWE-1333). Zero ambient capabilities.
