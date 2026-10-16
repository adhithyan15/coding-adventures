// Frozen compatibility formulas from incoming6e54; seed/path identities are
// checked independently of final layout/collision scheduling. The legacy float
// offset loop is bounded here to256 iterations over DECLARED ordinary fixtures.
mod ishikawa_handdrawn_contract {
use super::*;
use diagram_ir::LayoutedIshikawaHatch;
fn reference_line_path(from: &Point, to: &Point, hand_drawn: bool, seed: i64, index: usize) -> Vec<Point> {
    if !hand_drawn { return vec![from.clone(), to.clone()]; }
    let dx = to.x - from.x; let dy = to.y - from.y; let length = dx.hypot(dy).max(1.0);
    let perpendicular = (-dy / length, dx / length);
    let mut state = (seed as u64).wrapping_add((index as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15));
    let mut offset = || { state ^= state << 13; state ^= state >> 7; state ^= state << 17; (state as f64 / u64::MAX as f64 - 0.5) * 6.0 };
    let mut points = vec![from.clone()];
    for ratio in [0.25, 0.5, 0.75] {
        let jitter = offset();
        points.push(Point { x: from.x + dx * ratio + perpendicular.0 * jitter,
            y: from.y + dy * ratio + perpendicular.1 * jitter });
    }
    points.push(to.clone()); points
}

fn reference_arrowhead(path: &[Point]) -> Vec<Point> {
    let [.., before, tip] = path else { return Vec::new(); };
    let dx = tip.x - before.x; let dy = tip.y - before.y; let length = dx.hypot(dy).max(1.0);
    let unit = (dx / length, dy / length); let perpendicular = (-unit.1, unit.0);
    let base = Point { x: tip.x - unit.0 * 12.0, y: tip.y - unit.1 * 12.0 };
    vec![tip.clone(), Point { x: base.x + perpendicular.0 * 5.0, y: base.y + perpendicular.1 * 5.0 },
        Point { x: base.x - perpendicular.0 * 5.0, y: base.y - perpendicular.1 * 5.0 }]
}

fn reference_box_outline(x: f64, y: f64, width: f64, height: f64, hand_drawn: bool, seed: i64, index: usize) -> Vec<Point> {
    let corners = [Point { x, y }, Point { x: x + width, y }, Point { x: x + width, y: y + height },
        Point { x, y: y + height }, Point { x, y }];
    let mut outline = vec![corners[0].clone()];
    for side in 0..4 {
        let points = reference_line_path(&corners[side], &corners[side + 1], hand_drawn, seed, index * 4 + side);
        outline.extend(points.into_iter().skip(1));
    }
    outline
}

fn reference_effect_outline(x: f64, y: f64, width: f64, height: f64, hand_drawn: bool, seed: i64, index: usize) -> Vec<Point> {
    let mut shape = vec![Point { x, y }, Point { x, y: y + height }];
    for step in 1..=12 {
        let t = step as f64 / 12.0; let one_minus_t = 1.0 - t;
        shape.push(Point { x: one_minus_t * one_minus_t * x + 2.0 * one_minus_t * t * (x + width) + t * t * x,
            y: one_minus_t * one_minus_t * (y + height) + 2.0 * one_minus_t * t * (y + height / 2.0) + t * t * y });
    }
    if !hand_drawn { return shape; }
    let mut outline = vec![shape[0].clone()];
    for side in 0..shape.len() - 1 {
        let points = reference_line_path(&shape[side], &shape[side + 1], true, seed, index * 16 + side);
        outline.extend(points.into_iter().skip(1));
    }
    outline
}

fn reference_hatches(outline: &[Point]) -> Vec<LayoutedIshikawaHatch> {
    let angle = -41.0_f64.to_radians(); let direction = (angle.cos(), angle.sin());
    let normal = (-direction.1, direction.0); let project = |point: &Point| point.x * normal.0 + point.y * normal.1;
    let minimum = outline.iter().map(&project).fold(f64::INFINITY, f64::min);
    let maximum = outline.iter().map(&project).fold(f64::NEG_INFINITY, f64::max);
    let mut hatches = Vec::new(); let mut offset = (minimum / 5.0).floor() * 5.0;
    for _ in 0..256 {
        if offset > maximum { return hatches; }
        let mut intersections = Vec::new();
        for edge in outline.windows(2) {
            let from = project(&edge[0]) - offset; let to = project(&edge[1]) - offset;
            if from * to <= 0.0 && (from - to).abs() > 1e-9 {
                let ratio = from / (from - to);
                let point = Point { x: edge[0].x + (edge[1].x - edge[0].x) * ratio,
                    y: edge[0].y + (edge[1].y - edge[0].y) * ratio };
                intersections.push((point.x * direction.0 + point.y * direction.1, point));
            }
        }
        intersections.sort_by(|left, right| left.0.total_cmp(&right.0));
        intersections.dedup_by(|left, right| (left.0 - right.0).abs() < 1e-7);
        for pair in intersections.as_chunks::<2>().0 {
            hatches.push(LayoutedIshikawaHatch { from: pair[0].1.clone(), to: pair[1].1.clone() });
        }
        offset += 5.0;
    }
    panic!("declared ordinary fixture exceeded its 256-line reference bound");
}

// These complete fixtures compare the checked two-pass implementation with the
// bounded pre-repair one-pass compatibility math above, fed declared coordinates.
// No expected value reads an actual layout or calls a production layout helper.
struct Case { name:&'static str, body:&'static str, width:f64, height:f64, padding:f64,
    spine:((f64,f64),(f64,f64)), effect:(f64,f64), effect_glyph:(f64,f64), bones:Vec<Bone> }
struct Bone { label:&'static str,parent:Option<usize>,depth:usize,from:(f64,f64),to:(f64,f64),
    center:(f64,f64),width:f64,glyph:(f64,f64) }
fn point(p:(f64,f64))->Point {Point{x:p.0,y:p.1}}
fn expected_layout(case:&Case,config:diagram_ir::IshikawaConfig)->LayoutedIshikawaDiagram {
    let seed=config.hand_drawn_seed;
    let effect_outline=reference_effect_outline(case.effect.0,case.effect.1,144.0,68.0,true,seed,1_000);
    let effect_hatches=reference_hatches(&effect_outline);
    LayoutedIshikawaDiagram {width:case.width,height:case.height,effect:"E".into(),config,
        effect_x:case.effect.0,effect_y:case.effect.1,effect_width:144.0,effect_height:68.0,
        spine_from:point(case.spine.0),spine_to:point(case.spine.1),
        spine_path:reference_line_path(&point(case.spine.0),&point(case.spine.1),true,seed,0),effect_outline,effect_hatches,
        bones:case.bones.iter().enumerate().map(|(i,b)|{
            let path=reference_line_path(&point(b.from),&point(b.to),true,seed,i+1);
            let arrowhead=reference_arrowhead(&path);
            let label_outline=reference_box_outline(b.center.0-b.width/2.0,b.center.1-14.0,b.width,28.0,true,seed,2_000+i);
            let hatches=reference_hatches(&label_outline);
            diagram_ir::LayoutedIshikawaBone {from:point(b.from),to:point(b.to),path,arrowhead,
                label:b.label.into(),label_position:point(b.center),label_width:b.width,label_height:28.0,
                depth:b.depth,label_outline,hatches}
        }).collect()}
}
fn path(points:&[Point],stroke:&str,width:f64)->PaintInstruction {
    PaintInstruction::Path(PaintPath {base:PaintBase::default(),commands:points.iter().enumerate().map(|(i,p)|
        if i==0 {PathCommand::MoveTo{x:p.x,y:p.y}}else{PathCommand::LineTo{x:p.x,y:p.y}}).collect(),
        fill:Some("none".into()),fill_rule:None,stroke:Some(stroke.into()),stroke_width:Some(width),
        stroke_cap:Some(StrokeCap::Round),stroke_join:Some(StrokeJoin::Round),stroke_dash:None,stroke_dash_offset:None})
}
fn glyphs(label:&str,position:(f64,f64),size:f64)->PaintInstruction {
    PaintInstruction::GlyphRun(PaintGlyphRun{base:PaintBase::default(),glyphs:label.chars().enumerate().map(|(i,c)|
        GlyphPosition{glyph_id:c as u32,x:position.0+i as f64*(size/2.0),y:position.1}).collect(),
        font_ref:"fake:test".into(),font_size:size,fill:Some("rgb(17, 34, 51)".into())})
}
fn expected_scene(case:&Case,layout:&LayoutedIshikawaDiagram,family:&str,seed:i64)->PaintScene {
    let mut instructions=vec![path(&layout.spine_path,"#123456",3.0)];
    for b in &layout.bones {
        instructions.push(path(&b.path,"#123456",if b.depth==1 {2.5}else{1.5}));
        instructions.push(ishikawa_fixture_closed(&b.arrowhead,"#123456","#123456",1.0));
        for h in &b.hatches {instructions.push(path(&[h.from.clone(),h.to.clone()],"#abcdef",2.5));}
        instructions.push(ishikawa_fixture_closed(&b.label_outline,"none","#123456",1.0));
    }
    for h in &layout.effect_hatches {instructions.push(path(&[h.from.clone(),h.to.clone()],"#abcdef",2.5));}
    instructions.push(ishikawa_fixture_closed(&layout.effect_outline,"none","#123456",2.0));
    for b in &case.bones {instructions.push(glyphs(b.label,b.glyph,16.0));}
    instructions.push(glyphs("E",case.effect_glyph,6.0));
    PaintScene {width:case.width,height:case.height,background:"rgb(255,255,255)".into(),id:None,
        metadata:Some(HashMap::from([("ishikawa.diagramPadding".into(),case.padding.to_string()),
            ("ishikawa.useMaxWidth".into(),"true".into()),("ishikawa.fontSize".into(),"16".into()),
            ("ishikawa.look".into(),"handDrawn".into()),("ishikawa.handDrawnSeed".into(),seed.to_string()),
            ("ishikawa.fontFamily".into(),family.into()),("ishikawa.lineColor".into(),"#123456".into()),
            ("ishikawa.fillColor".into(),"#abcdef".into()),("ishikawa.textColor".into(),"#112233".into())])),instructions}
}
#[test]
fn bounded_legacy_clip_reference_matches_independent_analytic_rectangle() {
    let hatches=reference_hatches(&[point((0.0,0.0)),point((10.0,0.0)),point((10.0,10.0)),point((0.0,10.0)),point((0.0,0.0))]);
    let angle=41.0_f64.to_radians();let (s,c)=(angle.sin(),angle.cos());
    let expected=[((0.0,5.0/c),(5.0/s,0.0)),(((10.0-10.0*c)/s,10.0),(10.0,(10.0-10.0*s)/c))];
    assert_eq!(hatches.len(),2);
    for (actual,(from,to)) in hatches.iter().zip(expected) {
        for (observed,wanted) in [(actual.from.x,from.0),(actual.from.y,from.1),(actual.to.x,to.0),(actual.to.y,to.1)] {
            assert!((observed-wanted).abs()<1e-12,"analytic endpoint {observed} != {wanted}");
        }
    }
}
#[test]
fn complete_seeded_sources_keep_final_hatches_scenes_and_font_queries() {
    let cases=[
        Case{name:"single",body:"ishikawa-beta\nE\n  A\n",width:576.0,height:360.0,padding:80.0,
            spine:((96.0,180.0),(352.0,180.0)),effect:(352.0,146.0),effect_glyph:(380.5,158.8),
            bones:vec![Bone{label:"A",parent:None,depth:1,from:(162.56,108.0),to:(224.0,180.0),
                center:(193.28,102.0),width:72.0,glyph:(189.28,102.8)}]},
        Case{name:"nested-three",body:"ishikawa-beta\nE\n  A\n    BBBBBBBB\n      CCCCCCCCCCCCCCCCCCCCCCC\n",
            width:586.0,height:470.0,padding:85.0,spine:((101.0,235.0),(357.0,235.0)),effect:(357.0,201.0),effect_glyph:(385.5,213.8),
            bones:vec![Bone{label:"A",parent:None,depth:1,from:(167.56,127.0),to:(229.0,235.0),center:(198.28,139.0),width:72.0,glyph:(194.28,139.8)},
                Bone{label:"BBBBBBBB",parent:Some(1),depth:2,from:(122.56,102.25),to:(167.56,127.0),center:(122.56,107.0),width:76.0,glyph:(90.56,107.8)},
                Bone{label:"CCCCCCCCCCCCCCCCCCCCCCC",parent:Some(2),depth:3,from:(84.56,81.35),to:(122.56,102.25),center:(84.56,75.0),width:180.0,glyph:(-7.439999999999998,75.8)}]},
    ];
    for case in cases {for seed in [0,17,-7] {for family in ["Georgia","CourierNew"] {for directive in [false,true] {
        let prefix=if directive {format!("%%{{init: {{\"look\":\"handDrawn\",\"handDrawnSeed\":{seed},\"ishikawa\":{{\"diagramPadding\":{},\"useMaxWidth\":true}},\"themeVariables\":{{\"fontFamily\":\"{family}\",\"lineColor\":\"#123456\",\"mainBkg\":\"#abcdef\",\"textColor\":\"#112233\"}}}}}}%%\n",case.padding)}
            else {format!("---\nconfig:\n  look: handDrawn\n  handDrawnSeed: {seed}\n  ishikawa:\n    diagramPadding: {}\n    useMaxWidth: true\nthemeVariables:\n  fontFamily: {family}\n  lineColor: '#123456'\n  mainBkg: '#abcdef'\n  textColor: '#112233'\n---\n",case.padding)};
        let source=prefix+case.body;
        let config=diagram_ir::IshikawaConfig{font_size:16.0,look:diagram_ir::IshikawaLook::HandDrawn,
            hand_drawn_seed:seed,diagram_padding:case.padding,use_max_width:true,font_family:family.into(),
            line_color:"#123456".into(),fill_color:"#abcdef".into(),text_color:"#112233".into()};
        let expected_ir=diagram_ir::IshikawaDiagram{effect:"E".into(),config:config.clone(),
            causes:case.bones.iter().enumerate().map(|(i,b)|diagram_ir::IshikawaCause{id:format!("cause-{}",i+1),
                label:b.label.into(),parent_id:b.parent.map(|id|format!("cause-{id}")),depth:b.depth}).collect()};
        let parsed=mermaid_parser::parse_ishikawa(&source).unwrap();
        assert_eq!(parsed,expected_ir,"whole seeded source IR {source}");
        let expected_layout=expected_layout(&case,config);
        let layout=diagram_layout_geometric::try_layout_ishikawa(&parsed,case.width).unwrap();
        assert_eq!(layout,expected_layout,"whole seeded final layout {} seed{seed}",case.name);
        let expected=expected_scene(&case,&expected_layout,family,seed);
        let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
        let options=DiagramToPaintOptions{background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,
            label_font:FontSpec{family:"Ignored Label".into(),size:4.0,weight:600,italic:true,stretch:FontStretch::Condensed,..defaults.label_font},
            title_font:FontSpec{family:"Ignored Title".into(),size:6.0,weight:800,italic:true,stretch:FontStretch::Expanded,..defaults.title_font},
            shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        assert_eq!(diagram_to_paint_ishikawa(&layout,&options),expected,"whole seeded scene {} seed{seed}",case.name);
        assert_eq!(*resolver.0.borrow(),vec![FontQuery::named(family).with_weight(text_interfaces::FontWeight(600))
            .with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Condensed),
            FontQuery::named(family).with_weight(text_interfaces::FontWeight(800)).with_style(text_interfaces::FontStyle::Italic)
                .with_stretch(text_interfaces::FontStretch::Expanded)],"whole ordered seeded font queries");
        eprintln!("ishikawa-handdrawn COMPLETE case={} seed={seed} family={family} directive={directive} wholeIR/layout/scene/query",case.name);
    }}}}
}

}
