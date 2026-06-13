# sky130-pdk

Metadata and teaching-subset cell list for the SkyWater Sky130 130 nm open-source PDK.

## What's included

- **Process metadata** — V_DD (1.8 V), gate-oxide thickness, V_t, μC_ox, metal layers, cell-row height.
- **Teaching cell subset** — 35 cells: INV×4, BUF×4, NAND2/3, NOR2/3, AND2, OR2, XOR2, XNOR2, MUX2, DFF, latches, conb, clkbuf, tap, decap, fill.
- **Layer/datatype map** — GDS layer numbers and datatypes (li1, met1-5, poly, diff, tap, etc.).
- **Loader** — `load_sky130(PdkProfile, root)`.

## Usage

```rust
use sky130_pdk::{load_sky130, PdkProfile};

// No install needed for the teaching profile.
let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();

let inv = pdk.get_cell("sky130_fd_sc_hd__inv_1").unwrap();
println!("INV drive: {}", inv.drive_strength);  // 1

let layer = pdk.get_layer("met1.drawing").unwrap();
println!("met1 GDS layer: {}", layer.layer_number);  // 68
println!("met1 GDS dt:    {}", layer.datatype);       // 20

let meta = &pdk.process;
println!("Vdd: {} V, feature: {} nm", meta.vdd_nominal, meta.feature_size_nm);
```

## Testing

```
cargo test -p sky130-pdk -- --nocapture
```

12 unit tests + 3 doc-tests covering: cell presence, process metadata, layer map, error handling.
