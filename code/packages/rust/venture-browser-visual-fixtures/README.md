# venture-browser-visual-fixtures

Reusable deterministic visual acceptance for Venture's shared browser
pipeline and generated hosts.

The crate owns one versioned Mosaic-era page and its resources, deterministic
text measurement/shaping services, element and link geometry capture, full
RGBA screenshots, and exact structural screenshots. Structural screenshots
mask platform-font glyph pixels while retaining layout-driven images,
fallbacks, decorations, clipping, backgrounds, and viewport translation. This
keeps the exact ratchet portable across CoreText, DirectWrite, and Cairo hosts.

Generate inspectable PNG artifacts with:

```sh
cargo run -p venture-browser-visual-fixtures --example capture -- target/venture-visuals
```

Host launch tests should serve `fixture_response` from their existing local
HTTP server and report the shared frame and scroll probes. They should not copy
the fixture HTML or invent toolkit-specific visual baselines.

`load_international_page` provides a second non-golden fixture for reusable
text-flow acceptance. It covers inherited RTL content, isolate and embedding
controls, CJK wrapping without spaces, dictionary-segmented Thai/Lao/Khmer,
combining and emoji graphemes, and script/symbol font-fallback runs.

`isolated_gpu_layer_scene` and `assert_isolated_gpu_layer_pixels` form a
backend-neutral GPU composition oracle. The fixture catches flattened child
opacity, reordered filters, and incorrect destination-aware blending without
depending on a host toolkit or platform screenshot baseline. Both Metal and
WGPU consume this oracle; WGPU runs it whenever an offscreen adapter is
available, covering Metal, Vulkan, DX12, and software implementations through
one executor contract.
