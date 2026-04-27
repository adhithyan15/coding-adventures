module CodingAdventures.ImagePointOps.Tests

open System
open Xunit
open CodingAdventures.PixelContainer
open CodingAdventures.ImagePointOps

// Helper: 1×1 image with a single pixel.
let private solid r g b a =
    let img = PixelContainer(1, 1)
    img.SetPixel(0, 0, r, g, b, a)
    img

let private px (img: PixelContainer) =
    let p = img.GetPixel(0, 0)
    (p.R, p.G, p.B, p.A)

let private approxEq delta (a: byte) (b: byte) =
    abs (int a - int b) <= delta

// ── dimensions ────────────────────────────────────────────────────────────

[<Fact>]
let ``Dimensions are preserved`` () =
    let img = PixelContainer(3, 5)
    let out = invert img
    Assert.Equal(3, out.Width)
    Assert.Equal(5, out.Height)

// ── invert ────────────────────────────────────────────────────────────────

[<Fact>]
let ``Invert flips RGB`` () =
    let out = invert (solid 10uy 100uy 200uy 255uy)
    let r, g, b, a = px out
    Assert.Equal(245uy, r); Assert.Equal(155uy, g)
    Assert.Equal(55uy, b); Assert.Equal(255uy, a)

[<Fact>]
let ``Invert preserves alpha`` () =
    let out = invert (solid 10uy 100uy 200uy 128uy)
    let _, _, _, a = px out
    Assert.Equal(128uy, a)

[<Fact>]
let ``Double invert is identity`` () =
    let img = solid 30uy 80uy 180uy 255uy
    let out = invert (invert img)
    Assert.Equal(px img, px out)

// ── threshold ─────────────────────────────────────────────────────────────

[<Fact>]
let ``Threshold above gives white`` () =
    let out = threshold (solid 200uy 200uy 200uy 255uy) 128uy
    let r, g, b, _ = px out
    Assert.Equal(255uy, r); Assert.Equal(255uy, g); Assert.Equal(255uy, b)

[<Fact>]
let ``Threshold below gives black`` () =
    let out = threshold (solid 50uy 50uy 50uy 255uy) 128uy
    let r, _, _, _ = px out
    Assert.Equal(0uy, r)

[<Fact>]
let ``ThresholdLuminance white stays white`` () =
    let out = thresholdLuminance (solid 255uy 255uy 255uy 255uy) 128uy
    let r, _, _, _ = px out
    Assert.Equal(255uy, r)

// ── posterize ─────────────────────────────────────────────────────────────

[<Fact>]
let ``Posterize 2 levels binarises`` () =
    let out = posterize (solid 50uy 50uy 50uy 255uy) 2
    let r, _, _, _ = px out
    Assert.True(r = 0uy || r = 255uy)

// ── swapRGBBGR ────────────────────────────────────────────────────────────

[<Fact>]
let ``SwapRGBBGR swaps R and B`` () =
    let out = swapRGBBGR (solid 255uy 0uy 0uy 255uy)
    let r, g, b, _ = px out
    Assert.Equal(0uy, r); Assert.Equal(0uy, g); Assert.Equal(255uy, b)

// ── extractChannel ────────────────────────────────────────────────────────

[<Fact>]
let ``ExtractChannel R zeroes G and B`` () =
    let out = extractChannel (solid 100uy 150uy 200uy 255uy) R
    let r, g, b, _ = px out
    Assert.Equal(100uy, r); Assert.Equal(0uy, g); Assert.Equal(0uy, b)

// ── brightness ────────────────────────────────────────────────────────────

[<Fact>]
let ``Brightness clamps high`` () =
    let out = brightness (solid 250uy 10uy 10uy 255uy) 20
    let r, g, _, _ = px out
    Assert.Equal(255uy, r); Assert.Equal(30uy, g)

[<Fact>]
let ``Brightness clamps low`` () =
    let out = brightness (solid 5uy 10uy 10uy 255uy) -20
    let r, _, _, _ = px out
    Assert.Equal(0uy, r)

// ── contrast ──────────────────────────────────────────────────────────────

[<Fact>]
let ``Contrast identity`` () =
    let img = solid 100uy 150uy 200uy 255uy
    let out = contrast img 1.0
    let r, g, b, _ = px out
    let ir, ig, ib, _ = px img
    Assert.True(approxEq 1 r ir)
    Assert.True(approxEq 1 g ig)
    Assert.True(approxEq 1 b ib)

// ── gamma ─────────────────────────────────────────────────────────────────

[<Fact>]
let ``Gamma identity`` () =
    let img = solid 100uy 150uy 200uy 255uy
    let out = gamma img 1.0
    let r, _, _, _ = px out
    let ir, _, _, _ = px img
    Assert.True(approxEq 1 r ir)

[<Fact>]
let ``Gamma brightens midtones`` () =
    let out = gamma (solid 128uy 128uy 128uy 255uy) 0.5
    let r, _, _, _ = px out
    Assert.True(r > 128uy)

// ── exposure ──────────────────────────────────────────────────────────────

[<Fact>]
let ``Exposure +1 brightens`` () =
    let img = solid 100uy 100uy 100uy 255uy
    let out = exposure img 1.0
    let r, _, _, _ = px out
    let ir, _, _, _ = px img
    Assert.True(r > ir)

// ── greyscale ─────────────────────────────────────────────────────────────

[<Fact>]
let ``Greyscale white stays white`` () =
    for m in [Rec709; BT601; Average] do
        let out = greyscale (solid 255uy 255uy 255uy 255uy) m
        let r, g, b, _ = px out
        Assert.Equal(255uy, r); Assert.Equal(255uy, g); Assert.Equal(255uy, b)

[<Fact>]
let ``Greyscale black stays black`` () =
    let out = greyscale (solid 0uy 0uy 0uy 255uy) Rec709
    let r, g, b, _ = px out
    Assert.Equal(0uy, r); Assert.Equal(0uy, g); Assert.Equal(0uy, b)

// ── sepia ─────────────────────────────────────────────────────────────────

[<Fact>]
let ``Sepia preserves alpha`` () =
    let out = sepia (solid 128uy 128uy 128uy 200uy)
    let _, _, _, a = px out
    Assert.Equal(200uy, a)

// ── colourMatrix ──────────────────────────────────────────────────────────

[<Fact>]
let ``ColourMatrix identity`` () =
    let img = solid 80uy 120uy 200uy 255uy
    let id = array2D [[1.0;0.0;0.0];[0.0;1.0;0.0];[0.0;0.0;1.0]]
    let out = colourMatrix img id
    let r, g, b, _ = px out
    let ir, ig, ib, _ = px img
    Assert.True(approxEq 1 r ir)
    Assert.True(approxEq 1 g ig)
    Assert.True(approxEq 1 b ib)

// ── saturate ──────────────────────────────────────────────────────────────

[<Fact>]
let ``Saturate 0 gives grey`` () =
    let out = saturate (solid 200uy 100uy 50uy 255uy) 0.0
    let r, g, b, _ = px out
    Assert.Equal(r, g); Assert.Equal(g, b)

// ── hueRotate ─────────────────────────────────────────────────────────────

[<Fact>]
let ``HueRotate 360 is identity`` () =
    let img = solid 200uy 80uy 40uy 255uy
    let out = hueRotate img 360.0
    let r, g, b, _ = px out
    let ir, ig, ib, _ = px img
    Assert.True(approxEq 2 r ir)
    Assert.True(approxEq 2 g ig)
    Assert.True(approxEq 2 b ib)

// ── colorspace ────────────────────────────────────────────────────────────

[<Fact>]
let ``sRGB linear roundtrip`` () =
    let img = solid 100uy 150uy 200uy 255uy
    let out = linearToSRGBImage (srgbToLinearImage img)
    let r, g, b, _ = px out
    let ir, ig, ib, _ = px img
    Assert.True(approxEq 2 r ir)
    Assert.True(approxEq 2 g ig)
    Assert.True(approxEq 2 b ib)

// ── LUTs ──────────────────────────────────────────────────────────────────

[<Fact>]
let ``ApplyLUT1D invert LUT`` () =
    let lut = Array.init 256 (fun i -> byte (255 - i))
    let out = applyLUT1DU8 (solid 100uy 0uy 200uy 255uy) lut lut lut
    let r, g, b, _ = px out
    Assert.Equal(155uy, r); Assert.Equal(255uy, g); Assert.Equal(55uy, b)

[<Fact>]
let ``BuildLUT1DU8 identity`` () =
    let lut = buildLUT1DU8 id
    for i in 0..255 do
        Assert.True(approxEq 1 lut[i] (byte i), $"index {i}: {lut[i]}")

[<Fact>]
let ``BuildGammaLUT gamma=1 identity`` () =
    let lut = buildGammaLUT 1.0
    for i in 0..255 do
        Assert.True(approxEq 1 lut[i] (byte i), $"index {i}: {lut[i]}")
