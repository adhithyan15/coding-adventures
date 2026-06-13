//! Text writers for LEF 5.8 and DEF 5.8.
//!
//! Each function returns a `String`; callers write to disk with `std::fs::write`.

use crate::models::{CellLef, Def, LayerDef, PinDef, SiteDef, TechLef, ViaDef};

// ---------------------------------------------------------------------------
// LEF writers
// ---------------------------------------------------------------------------

/// Emit a TechLef as a LEF 5.8 string.
pub fn write_tech_lef_str(tech: &TechLef) -> String {
    let mut out = String::new();
    out.push_str(&format!("VERSION {} ;\n", tech.version));
    out.push_str("BUSBITCHARS \"[]\" ;\n");
    out.push_str("DIVIDERCHAR \"/\" ;\n");
    out.push_str(&format!(
        "UNITS\n  DATABASE MICRONS {} ;\nEND UNITS\n\n",
        tech.units_microns
    ));
    for layer in &tech.layers {
        out.push_str(&layer_to_lef(layer));
        out.push('\n');
    }
    for via in &tech.vias {
        out.push_str(&via_to_lef(via));
        out.push('\n');
    }
    for site in &tech.sites {
        out.push_str(&site_to_lef(site));
        out.push('\n');
    }
    out.push_str("END LIBRARY\n");
    out
}

fn layer_to_lef(l: &LayerDef) -> String {
    let mut s = format!("LAYER {}\n  TYPE {} ;\n", l.name, l.r#type);
    if let Some(dir) = &l.direction {
        s.push_str(&format!("  DIRECTION {dir} ;\n"));
    }
    if l.pitch != 0.0 { s.push_str(&format!("  PITCH {} ;\n", l.pitch)); }
    if l.width != 0.0 { s.push_str(&format!("  WIDTH {} ;\n", l.width)); }
    if l.spacing != 0.0 { s.push_str(&format!("  SPACING {} ;\n", l.spacing)); }
    s.push_str(&format!("END {}\n", l.name));
    s
}

fn via_to_lef(v: &ViaDef) -> String {
    let default = if v.is_default { " DEFAULT" } else { "" };
    let mut s = format!("VIA {}{default}\n", v.name);
    for vl in &v.layers {
        let r = &vl.rect;
        s.push_str(&format!(
            "  LAYER {} ;\n    RECT {} {} {} {} ;\n",
            vl.layer, r.x1, r.y1, r.x2, r.y2
        ));
    }
    s.push_str(&format!("END {}\n", v.name));
    s
}

fn site_to_lef(site: &SiteDef) -> String {
    format!(
        "SITE {}\n  CLASS {} ;\n  SIZE {} BY {} ;\nEND {}\n",
        site.name, site.class, site.width, site.height, site.name
    )
}

/// Emit a list of CellLef entries as a cells LEF string.
pub fn write_cells_lef_str(cells: &[CellLef]) -> String {
    let mut out = String::new();
    for cell in cells {
        out.push_str(&cell_to_lef(cell));
        out.push('\n');
    }
    out
}

fn cell_to_lef(cell: &CellLef) -> String {
    let mut s = format!("MACRO {}\n  CLASS {} ;\n  ORIGIN 0 0 ;\n", cell.name, cell.class);
    if let Some(f) = &cell.foreign {
        s.push_str(&format!("  FOREIGN {f} ;\n"));
    }
    s.push_str(&format!("  SIZE {} BY {} ;\n", cell.width, cell.height));
    if !cell.site.is_empty() {
        s.push_str(&format!("  SITE {} ;\n", cell.site));
    }
    for pin in &cell.pins {
        s.push_str(&pin_to_lef(pin));
    }
    if !cell.obs.is_empty() {
        s.push_str("  OBS\n");
        for (layer, rect) in &cell.obs {
            s.push_str(&format!(
                "    LAYER {layer} ;\n    RECT {} {} {} {} ;\n",
                rect.x1, rect.y1, rect.x2, rect.y2
            ));
        }
        s.push_str("  END\n");
    }
    s.push_str(&format!("END {}\n", cell.name));
    s
}

fn pin_to_lef(pin: &PinDef) -> String {
    let mut s = format!(
        "  PIN {}\n    DIRECTION {} ;\n    USE {} ;\n    PORT\n",
        pin.name,
        pin.direction.as_lef_str(),
        pin.use_.as_lef_str()
    );
    for port in &pin.ports {
        let r = &port.rect;
        s.push_str(&format!(
            "      LAYER {} ;\n      RECT {} {} {} {} ;\n",
            port.layer, r.x1, r.y1, r.x2, r.y2
        ));
    }
    s.push_str(&format!("    END\n  END {}\n", pin.name));
    s
}

// ---------------------------------------------------------------------------
// DEF writer
// ---------------------------------------------------------------------------

/// Emit a Def as a DEF 5.8 string.
pub fn write_def_str(def: &Def) -> String {
    let mut out = String::new();
    out.push_str(&format!("VERSION {} ;\n", def.version));
    out.push_str("DIVIDERCHAR \"/\" ;\n");
    out.push_str("BUSBITCHARS \"[]\" ;\n");
    out.push_str(&format!("DESIGN {} ;\n", def.design));
    out.push_str(&format!(
        "UNITS DISTANCE MICRONS {} ;\n\n",
        def.units_microns
    ));

    if let Some(d) = &def.die_area {
        out.push_str(&format!(
            "DIEAREA ( {} {} ) ( {} {} ) ;\n\n",
            d.x1, d.y1, d.x2, d.y2
        ));
    }

    for row in &def.rows {
        out.push_str(&format!(
            "ROW {} {} {} {} {} DO {} BY {} STEP {} {} ;\n",
            row.name, row.site, row.origin_x, row.origin_y,
            row.orientation, row.num_x, row.num_y,
            row.step_x, row.step_y
        ));
    }
    if !def.rows.is_empty() { out.push('\n'); }

    if !def.components.is_empty() {
        out.push_str(&format!("COMPONENTS {} ;\n", def.components.len()));
        for c in &def.components {
            let mut line = format!("  - {} {}", c.name, c.cell_type);
            if c.placed {
                if let (Some(lx), Some(ly)) = (c.location_x, c.location_y) {
                    line.push_str(&format!(" + PLACED ( {} {} ) {}", lx, ly, c.orientation));
                }
            }
            line.push_str(" ;");
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str("END COMPONENTS\n\n");
    }

    if !def.pins.is_empty() {
        out.push_str(&format!("PINS {} ;\n", def.pins.len()));
        for p in &def.pins {
            let mut line = format!(
                "  - {} + NET {} + DIRECTION {} + USE {}",
                p.name, p.net,
                p.direction.as_lef_str(),
                p.use_.as_lef_str()
            );
            if let (Some(layer), Some(rect)) = (&p.layer, &p.rect) {
                line.push_str(&format!(
                    " + LAYER {} ( {} {} ) ( {} {} )",
                    layer, rect.x1, rect.y1, rect.x2, rect.y2
                ));
            }
            line.push_str(" ;");
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str("END PINS\n\n");
    }

    if !def.nets.is_empty() {
        out.push_str(&format!("NETS {} ;\n", def.nets.len()));
        for n in &def.nets {
            let conns: String = n.connections
                .iter()
                .map(|(comp, pin)| format!("( {comp} {pin} )"))
                .collect::<Vec<_>>()
                .join(" ");
            let mut line = format!("  - {} {} + USE SIGNAL", n.name, conns);
            if !n.routed_segments.is_empty() {
                line.push_str("\n    ROUTED");
                for seg in &n.routed_segments {
                    let pts: String = seg.points
                        .iter()
                        .map(|(x, y)| format!("( {x} {y} )"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    line.push_str(&format!(" {} {}", seg.layer, pts));
                }
            }
            line.push_str(" ;");
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str("END NETS\n\n");
    }

    out.push_str("END DESIGN\n");
    out
}
