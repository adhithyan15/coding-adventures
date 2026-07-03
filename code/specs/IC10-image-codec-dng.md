# IC10 — DNG Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container), IC09 (image-codec-tiff)  
**Implements**: Adobe Digital Negative (DNG) 1.6

---

## 1. Overview

DNG (Digital Negative) is an open, publicly documented RAW image format created
by Adobe in 2004 and specified in the *DNG Specification* (current: 1.6, 2021).
DNG is a strict superset of TIFF 6.0: every DNG file is a valid TIFF file,
extended with private tags in the Adobe namespace (tag IDs 50700–51100+).

**Why DNG?**

- Open spec → no reverse engineering required
- Many cameras can output DNG natively (Google Pixel, Leica, Pentax, Hasselblad)
- Adobe Lightroom, ACR, and darktable can all convert proprietary RAWs to DNG
- DNG embeds its own colour calibration data, making correct colour much easier

**Key properties**:

- **Container**: TIFF 6.0 with DNG private tags
- **Bayer data**: PhotometricInterpretation = 32803 (CFA) or 34892 (LinearRaw)
- **Compression**: uncompressed (1), lossless JPEG (7), lossy JPEG (65536–65540)
- **Colour calibration**: `ColorMatrix1`/`ColorMatrix2`, `ForwardMatrix1/2`,
  `AsShotNeutral` (white balance), `CalibrationIlluminant1/2`
- **Preview images**: one or more lower-resolution IFDs (NewSubfileType != 0)
- **Black/white levels**: `BlackLevel`, `WhiteLevel` tags
- **Linearisation LUT**: optional `LinearizationTable` for non-linear sensors
- **Active area**: `ActiveArea` tag clips sensor edges (optical black rows/cols)

---

## 2. DNG Private Tags

All DNG-specific tags are in the range 50700–51100+. The most important ones:

| Tag   | Name                   | Type     | Description                                    |
|-------|------------------------|----------|------------------------------------------------|
| 50706 | DNGVersion             | BYTE[4]  | e.g., [1,6,0,0] for DNG 1.6                   |
| 50707 | DNGBackwardVersion     | BYTE[4]  | Minimum DNG version required to read           |
| 50708 | UniqueCameraModel      | ASCII    | Unique camera model string                     |
| 50710 | CFAPlaneColor          | BYTE     | Maps CFA plane index to colour (0=R,1=G,2=B)  |
| 50711 | CFALayout              | SHORT    | 1=rectangular (standard), 2–6 = non-rect       |
| 50712 | LinearizationTable     | SHORT[]  | Input→linear LUT (if sensor is not linear)     |
| 50713 | BlackLevelRepeatDim    | SHORT[2] | [rows, cols] of BlackLevel pattern             |
| 50714 | BlackLevel             | RATIONAL | Black level per CFA plane (may be per-pattern) |
| 50717 | WhiteLevel             | SHORT/LONG | Saturation point                             |
| 50718 | DefaultScale           | RATIONAL[2] | [H, V] scale to apply before output         |
| 50719 | DefaultCropOrigin      | RATIONAL[2] | [col, row] of top-left of crop             |
| 50720 | DefaultCropSize        | RATIONAL[2] | [width, height] of default crop            |
| 50721 | ColorMatrix1           | SRATIONAL[9]| 3×3 matrix: XYZ(D50) → camera raw (ill.1)  |
| 50722 | ColorMatrix2           | SRATIONAL[9]| 3×3 matrix: XYZ(D50) → camera raw (ill.2)  |
| 50723 | CameraCalibration1     | SRATIONAL[9]| Per-camera calibration correction           |
| 50724 | CameraCalibration2     | SRATIONAL[9]| Per-camera calibration correction (ill.2)   |
| 50728 | AsShotNeutral          | RATIONAL[3] | [R, G, B] white balance (as shot)          |
| 50729 | AsShotWhiteXY          | RATIONAL[2] | White point as XY chromaticity (alt. to above)|
| 50730 | BaselineExposure       | SRATIONAL| EV offset for tone mapping                      |
| 50778 | CalibrationIlluminant1 | SHORT    | Standard illuminant for matrix1 (17=D65, 21=D50)|
| 50779 | CalibrationIlluminant2 | SHORT    | Standard illuminant for matrix2                |
| 50829 | ActiveArea             | LONG[4]  | [top, left, bottom, right] sensor active region |
| 50830 | MaskedAreas            | LONG[]   | Optical black regions (4-tuple per region)     |
| 50831 | AsShotICCProfile       | UNDEFINED | ICC profile embedded                          |
| 50879 | ForwardMatrix1         | SRATIONAL[9]| 3×3: camera raw → XYZ(D50) for ill.1       |
| 50880 | ForwardMatrix2         | SRATIONAL[9]| 3×3: camera raw → XYZ(D50) for ill.2       |
| 51008 | OpcodeList1            | UNDEFINED | Pre-demosaic opcodes (WarpRectilinear, etc.)   |
| 51009 | OpcodeList2            | UNDEFINED | Post-demosaic opcodes                          |
| 51022 | OpcodeList3            | UNDEFINED | Post-linearisation opcodes                     |
| 51125 | DefaultUserCrop        | RATIONAL[4]| Default user crop in normalized coordinates  |

---

## 3. IFD Structure in DNG Files

A typical DNG file contains:

```
IFD0 (main image — full resolution RAW):
  NewSubfileType = 0x00000000 (full image)
  PhotometricInterpretation = 32803 (CFA) or 34892 (LinearRaw)
  Compression = 1 (uncompressed) or 7 (lossless JPEG)
  BitsPerSample = 12 or 14
  SamplesPerPixel = 1
  … DNG tags above …

IFD1 (embedded JPEG thumbnail / preview):
  NewSubfileType = 0x00000001 (reduced image)
  PhotometricInterpretation = 2 (RGB) or 6 (YCbCr)
  Compression = 6 (old-JPEG) or 7 (new-JPEG)
  … regular TIFF/JPEG tags …

Sub-IFD chain (pointed to by SubIFDs tag 330 in IFD0):
  May contain additional preview resolutions
```

**Selection rule**: The decoder selects the IFD with `NewSubfileType == 0`
as the RAW image. If multiple IFDs have NewSubfileType == 0, choose the one
with the largest width.

---

## 4. Colour Processing Pipeline

DNG embeds all data needed for correct colour reconstruction:

```
1. Read 12/14/16-bit sensor values from CFA strips/tiles
2. Apply LinearizationTable (if present): value = lut[value]
3. Subtract BlackLevel per CFA plane
4. Clip to [0, WhiteLevel - BlackLevel]
5. Normalize to [0.0, 1.0]
6. Bilinear Bayer demosaicing → linear camera RGB per pixel
7. Apply white balance: R *= 1/AsShotNeutral[0], G *= 1/AsShotNeutral[1],
                        B *= 1/AsShotNeutral[2]
   (AsShotNeutral is [neutral_R, neutral_G, neutral_B] normalised so that G≈1)
8. Apply ForwardMatrix (preferred, if present):
   [X,Y,Z] = ForwardMatrix × [R,G,B]
   Then apply Bradford adaptation from D50 to D65 (sRGB white point):
   [R_lin, G_lin, B_lin] = M_D50_to_sRGB × [X,Y,Z]
   OR apply ColorMatrix inverse (if ForwardMatrix absent):
   camera_to_xyz = inv(ColorMatrix)
   [X,Y,Z] = camera_to_xyz × [R,G,B]
9. Apply sRGB tone curve: y = 12.92*x (x≤0.0031308), else 1.055*x^(1/2.4)-0.055
10. Clip to [0.0, 1.0] and scale to u8 (0–255)
11. Set alpha = 255
```

### 4.1 White Balance from AsShotNeutral

`AsShotNeutral` = [R_n, G_n, B_n] means "the sensor read these values for
a neutral grey under the shot illuminant." White balance multipliers are:

```rust
let wb = [1.0 / as_shot_neutral[0],
          1.0 / as_shot_neutral[1],
          1.0 / as_shot_neutral[2]];
```

Normalize so that the green channel multiplier is 1.0:

```rust
let g = wb[1];
wb = [wb[0]/g, 1.0, wb[2]/g];
```

### 4.2 Matrix Interpolation Between Illuminants

DNG allows two calibration illuminants. If both are present, interpolate
using a simple daylight factor based on the white point's correlated colour
temperature (CCT). For v0.1, use only `ColorMatrix1` / `ForwardMatrix1`
(no CCT interpolation required).

---

## 5. Active Area and Crop

```
ActiveArea = [top, left, bottom, right]   // sensor-level coordinates
DefaultCropOrigin = [cropLeft, cropTop]   // within ActiveArea
DefaultCropSize   = [cropWidth, cropHeight]

Final image = sensor[ActiveArea.top .. ActiveArea.bottom,
                     ActiveArea.left .. ActiveArea.right]
            cropped to DefaultCropOrigin + DefaultCropSize
```

For v0.1, `ActiveArea` is respected; `DefaultCrop*` is ignored (output the
full active area).

---

## 6. API

```rust
/// Decode a DNG file to RGBA8 PixelContainer.
/// Uses DNG colour calibration tags automatically.
pub fn decode_dng(bytes: &[u8]) -> Result<PixelContainer, String>;

/// Decode with explicit options (override colour calibration).
pub fn decode_dng_with_opts(bytes: &[u8], opts: &DngDecodeOptions)
    -> Result<PixelContainer, String>;

/// Encode a PixelContainer as a minimal DNG file (uncompressed CFA).
/// Note: encodes as a synthetic LinearRaw (identity colour matrix).
pub fn encode_dng(pixels: &PixelContainer) -> Vec<u8>;

pub struct DngCodec;
impl paint_instructions::ImageCodec for DngCodec {
    fn mime_type(&self) -> &'static str { "image/x-adobe-dng" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

pub const VERSION: &str = "0.1.0";

pub struct DngDecodeOptions {
    /// Override ForwardMatrix (camera RGB → XYZ D50).
    pub forward_matrix: Option<[[f64; 3]; 3]>,
    /// Override white balance multipliers [R, G, B].
    pub wb_override: Option<[f64; 3]>,
    /// Whether to apply DefaultCrop. Default: false (output full ActiveArea).
    pub apply_crop: bool,
}
```

---

## 7. Crate Layout

```
image-codec-dng/
  Cargo.toml        (deps: pixel-container, paint-instructions, image-codec-tiff)
  BUILD             (cargo test -p image-codec-dng -- --nocapture)
  README.md
  CHANGELOG.md
  src/
    lib.rs          (pub API, DngCodec, VERSION)
    tags.rs         (DNG tag constants + reading helpers)
    color.rs        (DNG colour pipeline: WB, matrix, gamma)
    crop.rs         (ActiveArea + DefaultCrop handling)
    encoder.rs      (synthetic DNG writer for round-trip tests)
    decoder.rs      (decode_dng: find raw IFD, build TiffDecodeOptions)
```

---

## 8. Test Strategy (≥95% coverage target)

| Category                              | Tests |
|---------------------------------------|-------|
| Round-trip (synthetic DNG, solid colour) | 2  |
| DNG tag parsing (AsShotNeutral, WL, BL)  | 3  |
| Colour pipeline (WB application)         | 2  |
| ForwardMatrix → sRGB conversion          | 1  |
| ColorMatrix1 fallback (no ForwardMatrix) | 1  |
| ActiveArea crop                          | 1  |
| Thumbnail IFD skipped correctly          | 1  |
| LinearizationTable applied              | 1  |
| Error: no raw IFD found                  | 1  |
| Error: truncated DNG                     | 1  |
| MIME type + codec trait                  | 1  |
| **Total**                               | **15**|

---

## 9. Security Constraints

Inherits IC09 (TIFF) constraints. Additional:
- `LinearizationTable` max length: 65536 entries
- `OpcodeList*` tags: parsed for size only; opcodes not executed in v0.1
- ForwardMatrix / ColorMatrix values: reject if denominator == 0

---

## 10. References

- Adobe DNG Specification 1.6.0.0 — https://helpx.adobe.com/camera-raw/using/adobe-dng-converter.html
- DNG SDK — https://github.com/aamini/dng-sdk (Apache 2.0 reference)
- Colour-science Python library (colour.io) for matrix math reference
