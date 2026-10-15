// Literal source/geometry tables, specified before importing label-box code.
// Fake font4 has advance2/ascent3.2, keeping the long-label case on one line.
// These are complete drawing/transport oracles, not a general font-fit promise.
mod ishikawa_box_labels_contract {
    use super::*;

    struct Bone {
        label: &'static str,
        from: (f64, f64),
        to: (f64, f64),
        center: (f64, f64),
        width: f64,
        rect: (f64, f64),
        glyph: (f64, f64),
        depth: usize,
    }
    struct Case {
        name: &'static str,
        source: String,
        causes: Vec<(&'static str, Option<usize>, usize)>,
        width: f64,
        height: f64,
        padding: f64,
        spine: ((f64, f64), (f64, f64)),
        effect: (f64, f64),
        effect_glyph: (f64, f64),
        bones: Vec<Bone>,
    }
    fn point(pair: (f64, f64)) -> Point { Point { x: pair.0, y: pair.1 } }
    fn line(from: (f64, f64), to: (f64, f64), width: f64) -> PaintInstruction {
        PaintInstruction::Path(PaintPath { base: PaintBase::default(),
            commands: vec![PathCommand::MoveTo{x:from.0,y:from.1},PathCommand::LineTo{x:to.0,y:to.1}],
            fill:Some("none".into()),fill_rule:None,stroke:Some("#123456".into()),stroke_width:Some(width),
            stroke_cap:Some(StrokeCap::Round),stroke_join:Some(StrokeJoin::Round),stroke_dash:None,stroke_dash_offset:None })
    }
    fn rectangle(x:f64,y:f64,width:f64,height:f64,stroke:f64,radius:f64) -> PaintInstruction {
        PaintInstruction::Rect(PaintRect {base:PaintBase::default(),x,y,width,height,
            fill:Some("#abcdef".into()),stroke:Some("#123456".into()),stroke_width:Some(stroke),
            corner_radius:Some(radius),stroke_dash:None,stroke_dash_offset:None})
    }
    fn glyphs(label:&str, position:(f64,f64), size:f64) -> PaintInstruction {
        PaintInstruction::GlyphRun(PaintGlyphRun {base:PaintBase::default(),
            glyphs:label.chars().enumerate().map(|(index,c)|GlyphPosition {glyph_id:c as u32,
                x:position.0+index as f64*2.0,y:position.1}).collect(),
            font_ref:"fake:test".into(),font_size:size,fill:Some("rgb(17, 34, 51)".into())})
    }
    fn single(name:&'static str,label:&'static str,width:f64,glyph_x:f64) -> Case {
        Case {name,source:format!("ishikawa-beta\nE\n  {label}\n"),causes:vec![(label,None,1)],
            width:576.0,height:360.0,padding:80.0,spine:((96.0,180.0),(346.0,180.0)),
            effect:(346.0,151.0),effect_glyph:(419.5,163.8),bones:vec![Bone {
                label,from:(161.0,108.0),to:(221.0,180.0),center:(191.0,102.0),width,
                rect:(191.0-width/2.0,88.0),glyph:(glyph_x,93.2),depth:1}]}
    }
    #[test]
    fn complete_source_boxes_keep_scalar_widths_parity_depth_style_and_query_order() {
        let cases=vec![
            Case {name:"empty",source:"ishikawa-beta\nE\n".into(),causes:vec![],width:576.0,height:360.0,
                padding:80.0,spine:((96.0,180.0),(346.0,180.0)),effect:(346.0,151.0),effect_glyph:(419.5,163.8),bones:vec![]},
            single("short","A",72.0,190.0),
            single("interior","ABCDEFGH",76.0,183.0),
            single("clamped","CCCCCCCCCCCCCCCCCCCCCCC",180.0,168.0),
            single("unicode-scalars","界界界界界界界界",76.0,183.0),
            Case {name:"root-parities",source:"ishikawa-beta\nE\n  A\n  界界界界界界界界\n".into(),
                causes:vec![("A",None,1),("界界界界界界界界",None,1)],width:601.0,height:360.0,padding:30.0,
                spine:((46.0,180.0),(421.0,180.0)),effect:(421.0,151.0),effect_glyph:(494.5,163.8),
                bones:vec![Bone {label:"A",from:(111.0,72.0),to:(171.0,180.0),center:(141.0,84.0),width:72.0,
                    rect:(105.0,70.0),glyph:(140.0,75.2),depth:1},
                    Bone {label:"界界界界界界界界",from:(236.0,288.0),to:(296.0,180.0),center:(266.0,276.0),width:76.0,
                    rect:(228.0,262.0),glyph:(258.0,267.2),depth:1}]},
            Case {name:"nested-three",source:"ishikawa-beta\nE\n  A\n    BBBBBBBB\n      CCCCCCCCCCCCCCCCCCCCCCC\n".into(),
                causes:vec![("A",None,1),("BBBBBBBB",Some(1),2),("CCCCCCCCCCCCCCCCCCCCCCC",Some(2),3)],
                width:586.0,height:470.0,padding:85.0,spine:((101.0,235.0),(351.0,235.0)),
                effect:(351.0,206.0),effect_glyph:(424.5,218.8),bones:vec![
                    Bone {label:"A",from:(166.0,127.0),to:(226.0,235.0),center:(196.0,139.0),width:72.0,
                        rect:(160.0,125.0),glyph:(195.0,130.2),depth:1},
                    Bone {label:"BBBBBBBB",from:(121.0,102.25),to:(166.0,127.0),center:(121.0,120.25),width:76.0,
                        rect:(83.0,106.25),glyph:(113.0,111.45),depth:2},
                    Bone {label:"CCCCCCCCCCCCCCCCCCCCCCC",from:(83.0,81.35),to:(121.0,102.25),center:(83.0,99.35),width:180.0,
                        rect:(-7.0,85.35),glyph:(60.0,90.55),depth:3}]},
            Case {name:"finite-padding-clamp",source:"ishikawa-beta\nE\n  A\n".into(),causes:vec![("A",None,1)],
                width:576.0,height:360.0,padding:1_000_000_000.0,spine:((106.0,180.0),(336.0,180.0)),
                effect:(336.0,151.0),effect_glyph:(409.5,163.8),bones:vec![
                    Bone {label:"A",from:(165.8,115.2),to:(221.0,180.0),center:(193.4,105.6),width:72.0,
                        rect:(157.4,91.6),glyph:(192.4,96.8),depth:1}]},
        ];
        for family in ["Georgia","CourierNew"] { for case in &cases {
            let prefix=format!("---\nconfig:\n  ishikawa:\n    diagramPadding: {}\n    useMaxWidth: true\nthemeVariables:\n  fontFamily: {family}\n  lineColor: '#123456'\n  mainBkg: '#abcdef'\n  textColor: '#112233'\n---\n",case.padding);
            let source=prefix+&case.source;
            let config=diagram_ir::IshikawaConfig {diagram_padding:case.padding,use_max_width:true,
                font_family:family.into(),line_color:"#123456".into(),fill_color:"#abcdef".into(),text_color:"#112233".into()};
            let expected_ir=diagram_ir::IshikawaDiagram {effect:"E".into(),config:config.clone(),
                causes:case.causes.iter().enumerate().map(|(index,(label,parent,depth))|diagram_ir::IshikawaCause {
                    id:format!("cause-{}",index+1),label:(*label).into(),parent_id:parent.map(|id|format!("cause-{id}")),depth:*depth}).collect()};
            let ir=mermaid_parser::parse_ishikawa(&source).unwrap();assert_eq!(ir,expected_ir,"whole source IR {} {family}",case.name);
            let expected_layout=LayoutedIshikawaDiagram {width:case.width,height:case.height,effect:"E".into(),config,
                effect_x:case.effect.0,effect_y:case.effect.1,effect_width:150.0,effect_height:58.0,
                spine_from:point(case.spine.0),spine_to:point(case.spine.1),
                bones:case.bones.iter().map(|bone|diagram_ir::LayoutedIshikawaBone {from:point(bone.from),to:point(bone.to),
                    label:bone.label.into(),label_position:point(bone.center),label_width:bone.width,label_height:28.0,depth:bone.depth}).collect()};
            let layout=diagram_layout_geometric::layout_ishikawa(&ir,case.width);assert_eq!(layout,expected_layout,"whole layout {} {family}",case.name);
            let mut instructions=vec![line(case.spine.0,case.spine.1,3.0)];
            for bone in &case.bones {instructions.push(line(bone.from,bone.to,if bone.depth==1 {2.5}else{1.5}));
                instructions.push(rectangle(bone.rect.0,bone.rect.1,bone.width,28.0,1.0,4.0));}
            instructions.push(rectangle(case.effect.0,case.effect.1,150.0,58.0,2.0,8.0));
            for bone in &case.bones {instructions.push(glyphs(bone.label,bone.glyph,4.0));}
            instructions.push(glyphs("E",case.effect_glyph,6.0));
            let expected=PaintScene {width:case.width,height:case.height,background:"rgb(255,255,255)".into(),id:None,
                metadata:Some(HashMap::from([("ishikawa.diagramPadding".into(),case.padding.to_string()),("ishikawa.useMaxWidth".into(),"true".into()),
                    ("ishikawa.fontFamily".into(),family.into()),("ishikawa.lineColor".into(),"#123456".into()),
                    ("ishikawa.fillColor".into(),"#abcdef".into()),("ishikawa.textColor".into(),"#112233".into())])),instructions};
            let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
            let options=DiagramToPaintOptions {background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,
                label_font:FontSpec {family:"Ignored Label".into(),size:4.0,weight:600,italic:true,stretch:FontStretch::Condensed,..defaults.label_font},
                title_font:FontSpec {family:"Ignored Title".into(),size:6.0,weight:800,italic:true,stretch:FontStretch::Expanded,..defaults.title_font},
                shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
            assert_eq!(diagram_to_paint_ishikawa(&layout,&options),expected,"whole scene {} {family}",case.name);
            let mut queries=vec![];
            if !case.bones.is_empty() {queries.push(FontQuery::named(family).with_weight(text_interfaces::FontWeight(600)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Condensed));}
            queries.push(FontQuery::named(family).with_weight(text_interfaces::FontWeight(800)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Expanded));
            assert_eq!(*resolver.0.borrow(),queries,"whole ordered cached queries {} {family}",case.name);
            eprintln!("ishikawa-box COMPLETE case={} family={family} wholeIR/layout/scene/query",case.name);
        }}
    }

    #[test]
    fn cause_depth_controls_height_without_a_new_source_admission_limit() {
        for (depth,height) in [(0,360.0),(1,360.0),(2,400.0),(3,470.0),(7,750.0)] {
            let diagram=diagram_ir::IshikawaDiagram {effect:"E".into(),config:diagram_ir::IshikawaConfig::default(),
                causes:(1..=depth).map(|index|diagram_ir::IshikawaCause {id:format!("c{index}"),label:"A".into(),
                    parent_id:if index==1 {None}else{Some(format!("c{}",index-1))},depth:index}).collect()};
            let layout=diagram_layout_geometric::layout_ishikawa(&diagram,560.0);
            assert_eq!((layout.width,layout.height),(560.0,height),"complete canvas dimensions depth{depth}");
            assert_eq!(layout.bones.len(),depth,"authored chain not skipped");
            eprintln!("ishikawa-box HEIGHT depth={depth} width=560 expected={height}");
        }
    }
}
