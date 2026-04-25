use diagram_ir::{
    Compartment, CompartmentKind, LayoutedCompartment, LayoutedStructuralDiagram,
    LayoutedStructuralNode, LayoutedStructuralRelationship, Point, RelKind, StructuralDiagram,
    StructuralNode,
};

pub const VERSION: &str = "0.1.0";

const MIN_NODE_W: f64 = 160.0;
const HEADER_H: f64 = 40.0;
const ROW_H: f64 = 20.0;
const COMPARTMENT_PAD: f64 = 8.0;
const COL_GAP: f64 = 80.0;
const ROW_GAP: f64 = 60.0;
const COLS_PER_ROW: usize = 3;
const MARGIN: f64 = 32.0;

pub fn layout_structural_diagram(diagram: &StructuralDiagram) -> LayoutedStructuralDiagram {
    let nodes = assign_geometry(&diagram.nodes);
    let relationships = diagram.relationships.iter().map(|rel| {
        let from_node = nodes.iter().find(|n| n.id == rel.from);
        let to_node = nodes.iter().find(|n| n.id == rel.to);
        let (p1, p2) = match (from_node, to_node) {
            (Some(f), Some(t)) => closest_side_points(f, t),
            _ => (Point { x: 0.0, y: 0.0 }, Point { x: 0.0, y: 0.0 }),
        };
        LayoutedStructuralRelationship {
            from_id: rel.from.clone(), to_id: rel.to.clone(), kind: rel.kind.clone(),
            points: vec![p1, p2], from_mult: rel.from_mult.clone(), to_mult: rel.to_mult.clone(),
            label: rel.label.as_ref().map(|lbl: &String| {
                let mid = match (from_node, to_node) {
                    (Some(f), Some(t)) => Point { x: (ncx(f)+ncx(t))/2.0, y: (ncy(f)+ncy(t))/2.0 },
                    _ => Point { x: 0.0, y: 0.0 },
                };
                (mid, lbl.clone())
            }),
        }
    }).collect();
    let (tw, th) = canvas_size(&nodes);
    LayoutedStructuralDiagram { width: tw, height: th, nodes, relationships }
}

fn node_w(node: &StructuralNode) -> f64 {
    let hl = node.label.len() + node.stereotype.as_ref().map(|s| s.len()+4).unwrap_or(0);
    let ml = node.compartments.iter().flat_map(|c| c.entries.iter()).map(|e| e.len()).max().unwrap_or(0);
    (hl.max(ml) as f64 * 7.5 + 24.0).max(MIN_NODE_W)
}
fn node_h(node: &StructuralNode) -> f64 {
    let mut h = HEADER_H;
    for c in &node.compartments { match c.kind { CompartmentKind::Header => {}, _ => { h += COMPARTMENT_PAD*2.0 + c.entries.len().max(1) as f64 * ROW_H; } } }
    h
}
fn assign_geometry(nodes: &[StructuralNode]) -> Vec<LayoutedStructuralNode> {
    let mut result = Vec::new();
    let mut x = MARGIN; let mut y = MARGIN; let mut row_h = 0.0_f64;
    for (i, node) in nodes.iter().enumerate() {
        let w = node_w(node); let h = node_h(node);
        if i > 0 && i % COLS_PER_ROW == 0 { x = MARGIN; y += row_h + ROW_GAP; row_h = 0.0; }
        let comps = build_compartments(&node.compartments);
        result.push(LayoutedStructuralNode { id:node.id.clone(), x, y, width:w, height:h, header:node.label.clone(), stereotype:node.stereotype.clone(), compartments:comps });
        x += w + COL_GAP; row_h = row_h.max(h);
    }
    result
}
fn build_compartments(comps: &[Compartment]) -> Vec<LayoutedCompartment> {
    let mut result = Vec::new(); let mut y_off = HEADER_H;
    for c in comps { match c.kind { CompartmentKind::Header => continue, _ => {
        let h = COMPARTMENT_PAD*2.0 + c.entries.len().max(1) as f64 * ROW_H;
        result.push(LayoutedCompartment { y_offset: y_off, height: h, rows: c.entries.clone() });
        y_off += h;
    }}}
    result
}
fn closest_side_points(a: &LayoutedStructuralNode, b: &LayoutedStructuralNode) -> (Point, Point) {
    let dx = ncx(b) - ncx(a); let dy = ncy(b) - ncy(a);
    if dx.abs() > dy.abs() {
        if dx >= 0.0 { (Point{x:a.x+a.width,y:ncy(a)}, Point{x:b.x,y:ncy(b)}) }
        else { (Point{x:a.x,y:ncy(a)}, Point{x:b.x+b.width,y:ncy(b)}) }
    } else {
        if dy >= 0.0 { (Point{x:ncx(a),y:a.y+a.height}, Point{x:ncx(b),y:b.y}) }
        else { (Point{x:ncx(a),y:a.y}, Point{x:ncx(b),y:b.y+b.height}) }
    }
}
fn ncx(n: &LayoutedStructuralNode) -> f64 { n.x + n.width/2.0 }
fn ncy(n: &LayoutedStructuralNode) -> f64 { n.y + n.height/2.0 }
fn canvas_size(nodes: &[LayoutedStructuralNode]) -> (f64, f64) {
    let w = nodes.iter().map(|n| n.x+n.width).fold(0.0_f64, f64::max) + MARGIN;
    let h = nodes.iter().map(|n| n.y+n.height).fold(0.0_f64, f64::max) + MARGIN;
    (w.max(100.0), h.max(100.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{Compartment,CompartmentKind,RelKind,StructuralDiagram,StructuralKind,StructuralNode,StructuralNodeKind,StructuralRelationship};
    fn make_diagram() -> StructuralDiagram {
        StructuralDiagram { kind:StructuralKind::Class, title:None,
            nodes: vec![
                StructuralNode { id:"Animal".into(), label:"Animal".into(), stereotype:None, node_kind:StructuralNodeKind::Abstract,
                    compartments:vec![Compartment{kind:CompartmentKind::Fields,entries:vec!["+name: String".into()]},
                                       Compartment{kind:CompartmentKind::Methods,entries:vec!["+sound() String".into()]}] },
                StructuralNode { id:"Dog".into(), label:"Dog".into(), stereotype:None, node_kind:StructuralNodeKind::Class,
                    compartments:vec![Compartment{kind:CompartmentKind::Methods,entries:vec!["+sound() String".into()]}] },
            ],
            relationships:vec![StructuralRelationship{from:"Dog".into(),to:"Animal".into(),kind:RelKind::Inheritance,from_mult:None,to_mult:None,label:None}] }
    }
    #[test] fn version_exists() { assert_eq!(VERSION, "0.1.0"); }
    #[test] fn two_nodes_laid_out() { assert_eq!(layout_structural_diagram(&make_diagram()).nodes.len(), 2); }
    #[test] fn nodes_have_positive_dimensions() {
        for n in &layout_structural_diagram(&make_diagram()).nodes { assert!(n.width >= MIN_NODE_W); assert!(n.height >= HEADER_H); }
    }
    #[test] fn relationship_has_two_points() {
        let out = layout_structural_diagram(&make_diagram());
        assert_eq!(out.relationships[0].points.len(), 2);
        assert_eq!(out.relationships[0].kind, RelKind::Inheritance);
    }
    #[test] fn canvas_size_positive() { let out = layout_structural_diagram(&make_diagram()); assert!(out.width > 0.0); assert!(out.height > 0.0); }
    #[test] fn compartments_have_correct_rows() {
        let out = layout_structural_diagram(&make_diagram());
        let animal = out.nodes.iter().find(|n| n.id == "Animal").unwrap();
        assert_eq!(animal.compartments[0].rows, vec!["+name: String"]);
    }
}
