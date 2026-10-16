// Expected geometry uses fixture-declared centers/radii, never layout output.
// Plain links still carry complete label geometry in whole-layout equality.
fn wardley_fixture_link(from: diagram_ir::Point, to: diagram_ir::Point,
    source_radius: f64, target_radius: f64) -> diagram_ir::LayoutedWardleyLink {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let distance = dx.hypot(dy);
    let (ux, uy) = if distance > f64::EPSILON { (dx / distance, dy / distance) } else { (0.0, 0.0) };
    let angle = dy.atan2(dx).to_degrees();
    diagram_ir::LayoutedWardleyLink {
        from: diagram_ir::Point { x: from.x + ux * source_radius, y: from.y + uy * source_radius },
        to: diagram_ir::Point { x: to.x - ux * target_radius, y: to.y - uy * target_radius },
        dashed: false, label: None, flow: None,
        label_position: diagram_ir::Point { x: (from.x + to.x) / 2.0 + uy * 8.0,
            y: (from.y + to.y) / 2.0 - ux * 8.0 },
        label_angle: if (-90.0..=90.0).contains(&angle) { angle } else { angle + 180.0 },
    }
}
