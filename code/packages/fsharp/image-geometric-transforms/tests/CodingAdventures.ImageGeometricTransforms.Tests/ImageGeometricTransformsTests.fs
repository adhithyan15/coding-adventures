module CodingAdventures.ImageGeometricTransforms.Tests

open System
open Xunit
open CodingAdventures.PixelContainer
open CodingAdventures.ImageGeometricTransforms

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Create a 1×1 image containing a single pixel.
let private solid r g b a =
    let img = PixelContainer(1, 1)
    img.SetPixel(0, 0, r, g, b, a)
    img

/// Extract the single pixel from a 1×1 image as an (R, G, B, A) tuple.
let private px (img: PixelContainer) =
    let p = img.GetPixel(0, 0)
    (p.R, p.G, p.B, p.A)

/// True if two bytes differ by at most delta.  Used for lossy round-trips.
let private approxEq delta (a: byte) (b: byte) = abs (int a - int b) <= delta

/// Create a 2×2 image with distinct corner pixels for direction-sensitive tests.
///   TL = top-left  (0,0)   TR = top-right  (1,0)
///   BL = bot-left  (0,1)   BR = bot-right  (1,1)
let private corners tl tr bl br =
    let img = PixelContainer(2, 2)
    let set x y (r, g, b, a) = img.SetPixel(x, y, r, g, b, a)
    set 0 0 tl; set 1 0 tr
    set 0 1 bl; set 1 1 br
    img

let private getpx (img: PixelContainer) x y =
    let p = img.GetPixel(x, y)
    (p.R, p.G, p.B, p.A)

// ── flipHorizontal ────────────────────────────────────────────────────────────

[<Fact>]
let ``flipHorizontal preserves dimensions`` () =
    let img = PixelContainer(4, 7)
    let out = flipHorizontal img
    Assert.Equal(4, out.Width)
    Assert.Equal(7, out.Height)

[<Fact>]
let ``flipHorizontal swaps left and right columns`` () =
    let img = corners (255uy,0uy,0uy,255uy) (0uy,0uy,255uy,255uy) (0uy,255uy,0uy,255uy) (255uy,255uy,0uy,255uy)
    let out = flipHorizontal img
    // After flip: TL becomes TR and vice versa.
    Assert.Equal((0uy,0uy,255uy,255uy), getpx out 0 0)  // was TR
    Assert.Equal((255uy,0uy,0uy,255uy), getpx out 1 0)  // was TL

[<Fact>]
let ``flipHorizontal applied twice is identity`` () =
    let img = corners (100uy,0uy,0uy,255uy) (0uy,100uy,0uy,255uy) (0uy,0uy,100uy,255uy) (50uy,50uy,50uy,255uy)
    let out = flipHorizontal (flipHorizontal img)
    Assert.Equal(getpx img 0 0, getpx out 0 0)
    Assert.Equal(getpx img 1 0, getpx out 1 0)
    Assert.Equal(getpx img 0 1, getpx out 0 1)
    Assert.Equal(getpx img 1 1, getpx out 1 1)

// ── flipVertical ──────────────────────────────────────────────────────────────

[<Fact>]
let ``flipVertical swaps top and bottom rows`` () =
    let img = corners (255uy,0uy,0uy,255uy) (0uy,0uy,255uy,255uy) (0uy,255uy,0uy,255uy) (255uy,255uy,0uy,255uy)
    let out = flipVertical img
    // After flip: TL becomes BL and vice versa.
    Assert.Equal((0uy,255uy,0uy,255uy), getpx out 0 0)  // was BL
    Assert.Equal((255uy,0uy,0uy,255uy), getpx out 0 1)  // was TL

[<Fact>]
let ``flipVertical applied twice is identity`` () =
    let img = corners (200uy,0uy,0uy,255uy) (0uy,200uy,0uy,255uy) (0uy,0uy,200uy,255uy) (100uy,100uy,100uy,255uy)
    let out = flipVertical (flipVertical img)
    Assert.Equal(getpx img 0 0, getpx out 0 0)
    Assert.Equal(getpx img 1 1, getpx out 1 1)

// ── rotate90CW ────────────────────────────────────────────────────────────────

[<Fact>]
let ``rotate90CW swaps dimensions`` () =
    let img = PixelContainer(3, 7)
    let out = rotate90CW img
    Assert.Equal(7, out.Width)
    Assert.Equal(3, out.Height)

[<Fact>]
let ``rotate90CW top-left goes to top-right`` () =
    // TL pixel (0,0) in the source should appear at top-right of output (W'-1, 0).
    let img = corners (255uy,0uy,0uy,255uy) (0uy,0uy,255uy,255uy) (0uy,255uy,0uy,255uy) (255uy,255uy,0uy,255uy)
    let out = rotate90CW img
    // After 90° CW: top-left (0,0) → top-right of output (1,0)
    Assert.Equal((255uy,0uy,0uy,255uy), getpx out 1 0)

[<Fact>]
let ``rotate90CW four times is identity`` () =
    let img = corners (100uy,0uy,0uy,255uy) (0uy,100uy,0uy,255uy) (0uy,0uy,100uy,255uy) (50uy,50uy,50uy,255uy)
    let out = img |> rotate90CW |> rotate90CW |> rotate90CW |> rotate90CW
    Assert.Equal(getpx img 0 0, getpx out 0 0)
    Assert.Equal(getpx img 1 0, getpx out 1 0)
    Assert.Equal(getpx img 0 1, getpx out 0 1)
    Assert.Equal(getpx img 1 1, getpx out 1 1)

// ── rotate90CCW ───────────────────────────────────────────────────────────────

[<Fact>]
let ``rotate90CCW swaps dimensions`` () =
    let img = PixelContainer(5, 2)
    let out = rotate90CCW img
    Assert.Equal(2, out.Width)
    Assert.Equal(5, out.Height)

[<Fact>]
let ``rotate90CW then rotate90CCW is identity`` () =
    let img = corners (80uy,0uy,0uy,255uy) (0uy,80uy,0uy,255uy) (0uy,0uy,80uy,255uy) (40uy,40uy,40uy,255uy)
    let out = img |> rotate90CW |> rotate90CCW
    Assert.Equal(getpx img 0 0, getpx out 0 0)
    Assert.Equal(getpx img 1 1, getpx out 1 1)

// ── rotate180 ─────────────────────────────────────────────────────────────────

[<Fact>]
let ``rotate180 preserves dimensions`` () =
    let img = PixelContainer(3, 3)
    let out = rotate180 img
    Assert.Equal(3, out.Width)
    Assert.Equal(3, out.Height)

[<Fact>]
let ``rotate180 moves top-left to bottom-right`` () =
    let img = corners (255uy,0uy,0uy,255uy) (0uy,0uy,255uy,255uy) (0uy,255uy,0uy,255uy) (128uy,128uy,128uy,255uy)
    let out = rotate180 img
    Assert.Equal((255uy,0uy,0uy,255uy), getpx out 1 1)

[<Fact>]
let ``rotate180 applied twice is identity`` () =
    let img = corners (200uy,50uy,0uy,255uy) (0uy,200uy,50uy,255uy) (50uy,0uy,200uy,255uy) (100uy,100uy,100uy,255uy)
    let out = rotate180 (rotate180 img)
    Assert.Equal(getpx img 0 0, getpx out 0 0)
    Assert.Equal(getpx img 1 1, getpx out 1 1)

// ── crop ──────────────────────────────────────────────────────────────────────

[<Fact>]
let ``crop returns correct dimensions`` () =
    let img = PixelContainer(10, 10)
    let out = crop img 2 3 5 4
    Assert.Equal(5, out.Width)
    Assert.Equal(4, out.Height)

[<Fact>]
let ``crop returns correct pixel values`` () =
    // 3×3 image, crop out the centre pixel.
    let img = PixelContainer(3, 3)
    img.SetPixel(1, 1, 200uy, 100uy, 50uy, 255uy)
    let out = crop img 1 1 1 1
    Assert.Equal((200uy, 100uy, 50uy, 255uy), px out)

[<Fact>]
let ``crop at origin with full size is identity`` () =
    let img = corners (10uy,0uy,0uy,255uy) (0uy,10uy,0uy,255uy) (0uy,0uy,10uy,255uy) (5uy,5uy,5uy,255uy)
    let out = crop img 0 0 img.Width img.Height
    Assert.Equal(getpx img 0 0, getpx out 0 0)
    Assert.Equal(getpx img 1 1, getpx out 1 1)

// ── pad ───────────────────────────────────────────────────────────────────────

[<Fact>]
let ``pad produces correct output dimensions`` () =
    let img = PixelContainer(3, 4)
    let out = pad img 1 2 3 4 (0uy, 0uy, 0uy, 255uy)
    Assert.Equal(3 + 4 + 2, out.Width)   // left + src.Width + right
    Assert.Equal(4 + 1 + 3, out.Height)  // top + src.Height + bottom

[<Fact>]
let ``pad fills border with fill colour`` () =
    let img = PixelContainer(1, 1)
    img.SetPixel(0, 0, 100uy, 100uy, 100uy, 255uy)
    let fill = (255uy, 0uy, 0uy, 255uy)
    let out = pad img 1 1 1 1 fill
    // Top-left corner of output is in the border region.
    Assert.Equal(fill, getpx out 0 0)
    Assert.Equal(fill, getpx out 2 2)  // bottom-right corner

[<Fact>]
let ``pad preserves interior pixel`` () =
    let img = PixelContainer(1, 1)
    img.SetPixel(0, 0, 200uy, 150uy, 100uy, 255uy)
    let out = pad img 2 2 2 2 (0uy, 0uy, 0uy, 255uy)
    // The original pixel is now at (2, 2) in the output.
    Assert.Equal((200uy, 150uy, 100uy, 255uy), getpx out 2 2)

// ── scale ─────────────────────────────────────────────────────────────────────

[<Fact>]
let ``scale produces correct output dimensions`` () =
    let img = PixelContainer(10, 8)
    let out = scale img 20 16 Nearest
    Assert.Equal(20, out.Width)
    Assert.Equal(16, out.Height)

[<Fact>]
let ``scale 1x1 to 1x1 preserves pixel`` () =
    let img = solid 200uy 100uy 50uy 255uy
    let out = scale img 1 1 Nearest
    Assert.Equal(px img, px out)

[<Fact>]
let ``scale 2x nearest preserves corners`` () =
    // Scale up 2×, check that corners map to expected quadrants.
    let img = PixelContainer(2, 2)
    img.SetPixel(0, 0, 255uy, 0uy, 0uy, 255uy)
    img.SetPixel(1, 0, 0uy, 255uy, 0uy, 255uy)
    img.SetPixel(0, 1, 0uy, 0uy, 255uy, 255uy)
    img.SetPixel(1, 1, 128uy, 128uy, 128uy, 255uy)
    let out = scale img 4 4 Nearest
    // Top-left quadrant should be red.
    Assert.Equal((255uy, 0uy, 0uy, 255uy), getpx out 0 0)
    Assert.Equal((255uy, 0uy, 0uy, 255uy), getpx out 1 1)

// ── rotate (continuous) ───────────────────────────────────────────────────────

[<Fact>]
let ``rotate by 0.0 radians approximately preserves pixels`` () =
    let img = PixelContainer(5, 5)
    img.SetPixel(2, 2, 200uy, 100uy, 50uy, 255uy)
    let out = rotate img 0.0 Nearest Crop
    let pr, pg, pb, pa = getpx out 2 2
    let ir, ig, ib, ia = getpx img 2 2
    Assert.True(approxEq 2 pr ir)
    Assert.True(approxEq 2 pg ig)
    Assert.True(approxEq 2 pb ib)
    Assert.Equal(ia, pa)

[<Fact>]
let ``rotate Crop preserves dimensions`` () =
    let img = PixelContainer(6, 4)
    let out = rotate img 0.5 Nearest Crop
    Assert.Equal(6, out.Width)
    Assert.Equal(4, out.Height)

[<Fact>]
let ``rotate Fit expands canvas`` () =
    let img = PixelContainer(4, 4)
    let out = rotate img (Math.PI / 4.0) Nearest Fit
    // Rotating a 4×4 image 45° should produce a larger canvas.
    Assert.True(out.Width >= img.Width)
    Assert.True(out.Height >= img.Height)

// ── affine ────────────────────────────────────────────────────────────────────

[<Fact>]
let ``affine identity matrix preserves interior pixel`` () =
    // Identity matrix: u = 1*x + 0*y + 0,  v = 0*x + 1*y + 0
    let id = array2D [[1.0; 0.0; 0.0]; [0.0; 1.0; 0.0]]
    let img = PixelContainer(5, 5)
    img.SetPixel(2, 2, 200uy, 100uy, 50uy, 255uy)
    let out = affine img id 5 5 Nearest Replicate
    let pr, pg, pb, pa = getpx out 2 2
    let ir, ig, ib, ia = getpx img 2 2
    Assert.True(approxEq 2 pr ir)
    Assert.True(approxEq 2 pg ig)
    Assert.True(approxEq 2 pb ib)
    Assert.Equal(ia, pa)

[<Fact>]
let ``affine identity produces correct output dimensions`` () =
    let id = array2D [[1.0; 0.0; 0.0]; [0.0; 1.0; 0.0]]
    let img = PixelContainer(3, 7)
    let out = affine img id 3 7 Nearest Zero
    Assert.Equal(3, out.Width)
    Assert.Equal(7, out.Height)

// ── perspectiveWarp ───────────────────────────────────────────────────────────

[<Fact>]
let ``perspectiveWarp identity matrix preserves interior pixel`` () =
    // Identity homography: H = diag(1, 1, 1)
    let h = array2D [[1.0; 0.0; 0.0]; [0.0; 1.0; 0.0]; [0.0; 0.0; 1.0]]
    let img = PixelContainer(5, 5)
    img.SetPixel(2, 2, 180uy, 90uy, 30uy, 255uy)
    let out = perspectiveWarp img h 5 5 Nearest Replicate
    let pr, pg, pb, pa = getpx out 2 2
    let ir, ig, ib, ia = getpx img 2 2
    Assert.True(approxEq 2 pr ir)
    Assert.True(approxEq 2 pg ig)
    Assert.True(approxEq 2 pb ib)
    Assert.Equal(ia, pa)

[<Fact>]
let ``perspectiveWarp identity produces correct output dimensions`` () =
    let h = array2D [[1.0; 0.0; 0.0]; [0.0; 1.0; 0.0]; [0.0; 0.0; 1.0]]
    let img = PixelContainer(4, 6)
    let out = perspectiveWarp img h 8 12 Nearest Zero
    Assert.Equal(8, out.Width)
    Assert.Equal(12, out.Height)

// ── OutOfBounds modes ─────────────────────────────────────────────────────────

[<Fact>]
let ``affine Zero OOB gives transparent black outside source`` () =
    // Translate right by 100 pixels so the output sees only out-of-bounds source.
    let translate = array2D [[1.0; 0.0; 100.5]; [0.0; 1.0; 0.0]]
    let img = PixelContainer(5, 5)
    img.Fill(255uy, 255uy, 255uy, 255uy)
    let out = affine img translate 5 5 Nearest Zero
    // All output pixels map to source x ≥ 100 → out of bounds → transparent black.
    Assert.Equal((0uy, 0uy, 0uy, 0uy), getpx out 0 0)

[<Fact>]
let ``affine Replicate OOB clamps to edge`` () =
    let img = PixelContainer(3, 3)
    img.Fill(200uy, 100uy, 50uy, 255uy)
    // Translate so output maps to source x = -10 (far left of source) → Replicate clamps.
    let translate = array2D [[1.0; 0.0; -9.5]; [0.0; 1.0; 0.0]]
    let out = affine img translate 3 3 Nearest Replicate
    // Should replicate the left edge (still the fill colour in this case).
    let r, g, b, a = getpx out 0 1
    Assert.Equal(200uy, r)

[<Fact>]
let ``affine Wrap OOB tiles correctly`` () =
    // 2×1 image with left pixel red and right pixel blue.
    let img = PixelContainer(2, 1)
    img.SetPixel(0, 0, 255uy, 0uy, 0uy, 255uy)
    img.SetPixel(1, 0, 0uy, 0uy, 255uy, 255uy)
    // Translate right by 2 (one full tile) — should wrap back to same pixel.
    let translate = array2D [[1.0; 0.0; 2.0]; [0.0; 1.0; 0.0]]
    let out = affine img translate 2 1 Nearest Wrap
    let r, _, b, _ = getpx out 0 0
    // x=0 in output → source x=2 → wrap → source x=0 → red
    Assert.Equal(255uy, r)
    Assert.Equal(0uy, b)

[<Fact>]
let ``affine Reflect OOB mirrors at boundary`` () =
    // 2×1 image: pixel (0,0)=red, pixel (1,0)=blue.
    let img = PixelContainer(2, 1)
    img.SetPixel(0, 0, 255uy, 0uy, 0uy, 255uy)
    img.SetPixel(1, 0, 0uy, 0uy, 255uy, 255uy)
    // Output pixel (0,0): xf=0.5, u = 0.5 + (-2.0) = -1.5 → floor → -2.
    // Reflect: period=4, r=((-2%4)+4)%4=2, 2>=2 → period-1-r=1 → pixel (1,0) = blue.
    let translate = array2D [[1.0; 0.0; -2.0]; [0.0; 1.0; 0.5]]
    let out = affine img translate 1 1 Nearest Reflect
    let _, _, b, _ = getpx out 0 0
    Assert.Equal(255uy, b)

// ── nearest / bilinear quality checks ─────────────────────────────────────────

[<Fact>]
let ``Nearest scale is exact on solid colour`` () =
    let img = PixelContainer(2, 2)
    img.Fill(100uy, 150uy, 200uy, 255uy)
    let out = scale img 4 4 Nearest
    for y in 0..3 do
        for x in 0..3 do
            Assert.Equal((100uy, 150uy, 200uy, 255uy), getpx out x y)

[<Fact>]
let ``Bilinear midpoint of two same pixels is same pixel`` () =
    // 1×2 image where both pixels are identical — bilinear at any v should give the same colour.
    let img = PixelContainer(1, 2)
    img.SetPixel(0, 0, 120uy, 60uy, 30uy, 255uy)
    img.SetPixel(0, 1, 120uy, 60uy, 30uy, 255uy)
    let out = scale img 1 4 Bilinear
    for y in 0..3 do
        let r, g, b, _ = getpx out 0 y
        Assert.True(approxEq 2 r 120uy, $"r at y={y}: {r}")
        Assert.True(approxEq 2 g 60uy,  $"g at y={y}: {g}")
        Assert.True(approxEq 2 b 30uy,  $"b at y={y}: {b}")
