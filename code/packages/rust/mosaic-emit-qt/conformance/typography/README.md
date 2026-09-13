# Qt typography conformance

Generate the authored fixture, then load the emitted QML through Qt Quick Test:

```powershell
cargo run --manifest-path code/packages/rust/mosaic-compile/Cargo.toml -- pkg code/packages/rust/mosaic-emit-qt/conformance/typography --backend qt --output C:/temp/mosaic-qt-font
Copy-Item code/packages/rust/mosaic-emit-qt/conformance/typography/tst_typography.qml C:/temp/mosaic-qt-font/qt/
qmltestrunner -platform offscreen -input C:/temp/mosaic-qt-font/qt -o C:/temp/mosaic-qt-font/result.txt,txt
Get-Content C:/temp/mosaic-qt-font/result.txt
```

The test checks native font properties for text, buttons, text fields and
inherited table text/editors. Sizes 13/19.5/26 become 13/20/26 integer pixels.
NaN, infinity, zero, negative, overflowing and sub-pixel values restore the
13px authored fallback. A separate unstyled text checks restoration of the
actual platform default, without assuming that default's size. The fixture
uses Qt 6 and needs no application runtime or native table model; compiler
unit tests separately exercise canonical TableView delegate propagation.

Inspect the test report: some Windows Qt builds do not propagate test failures
as the shell process exit code. This is typography acceptance, not VisiCalc
visual or complete native workflow acceptance.
