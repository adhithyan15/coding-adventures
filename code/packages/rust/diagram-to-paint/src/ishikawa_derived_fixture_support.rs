// Test-only geometry from declared fixture coordinates, never returned layout.
// The fish head is a twelve-sample quadratic arc; arrows have length12/halfwidth5.
fn ishikawa_fixture_effect(x:f64,y:f64)->Vec<Point> {
    let mut points=vec![Point{x,y},Point{x,y:y+68.0}];
    for index in 1..=12 {
        let t=index as f64/12.0;let u=1.0-t;
        points.push(Point{x:u*u*x+2.0*u*t*(x+144.0)+t*t*x,
            y:u*u*(y+68.0)+2.0*u*t*(y+34.0)+t*t*y});
    }
    points
}
fn ishikawa_fixture_bone(ends:(Point,Point),label:&str,center:Point,width:f64,depth:usize)->diagram_ir::LayoutedIshikawaBone {
    let (from,to)=ends;let (dx,dy)=(to.x-from.x,to.y-from.y);let length=dx.hypot(dy).max(1.0);
    let (ux,uy)=(dx/length,dy/length);let (bx,by)=(to.x-ux*12.0,to.y-uy*12.0);
    let arrowhead=vec![to.clone(),Point{x:bx-uy*5.0,y:by+ux*5.0},Point{x:bx+uy*5.0,y:by-ux*5.0}];
    let (x,y)=(center.x-width/2.0,center.y-14.0);
    let label_outline=vec![Point{x,y},Point{x:x+width,y},Point{x:x+width,y:y+28.0},Point{x,y:y+28.0},Point{x,y}];
    diagram_ir::LayoutedIshikawaBone{path:vec![from.clone(),to.clone()],from,to,label:label.into(),
        label_position:center,label_width:width,label_height:28.0,depth,arrowhead,label_outline}
}
fn ishikawa_fixture_closed(points:&[Point],fill:&str,stroke:&str,width:f64)->PaintInstruction {
    let mut commands:Vec<_>=points.iter().enumerate().map(|(i,p)|if i==0 {PathCommand::MoveTo{x:p.x,y:p.y}}
        else {PathCommand::LineTo{x:p.x,y:p.y}}).collect();commands.push(PathCommand::Close);
    PaintInstruction::Path(PaintPath{base:PaintBase::default(),commands,fill:Some(fill.into()),fill_rule:None,
        stroke:Some(stroke.into()),stroke_width:Some(width),stroke_cap:Some(StrokeCap::Round),stroke_join:Some(StrokeJoin::Round),
        stroke_dash:None,stroke_dash_offset:None})
}
