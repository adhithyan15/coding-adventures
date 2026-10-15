// Independent authored geometry keeps the palette oracle separate from placement.
// Tables include indices five/eight and wraparound; overrides retain global order.
#[test]
fn venn_palette_typed_complete_scenes_empty_short_default_and_long_cycles() {
    use std::cell::RefCell;
    struct RecordingResolver(RefCell<Vec<FontQuery>>);
    impl FontResolver for RecordingResolver {
        type Handle=FakeHandle;
        fn resolve(&self,q:&FontQuery)->Result<FakeHandle,FontResolutionError> {
            self.0.borrow_mut().push(q.clone());Ok(FakeHandle)
        }
    }
    let default_colors=["#60a5fa","#f59e0b","#34d399","#f472b6","#a78bfa","#22d3ee","#fb7185","#a3e635"];
    let default_rgb=["96,165,250","245,158,11","52,211,153","244,114,182","167,139,250","34,211,238","251,113,133","163,230,53","96,165,250","245,158,11","52,211,153"];
    let numbered=["#010101","#020202","#030303","#040404","#050505","#060606","#070707","#080808","#090909","#0a0a0a"];
    let cases:Vec<(Vec<String>,Vec<&str>)>=vec![
        (vec![],vec!["96,165,250";11]),
        (numbered[..1].iter().map(|s|s.to_string()).collect(),vec!["1,1,1";11]),
        (numbered[..2].iter().map(|s|s.to_string()).collect(),vec!["1,1,1","2,2,2","1,1,1","2,2,2","1,1,1","2,2,2","1,1,1","2,2,2","1,1,1","2,2,2","1,1,1"]),
        (numbered[..8].iter().map(|s|s.to_string()).collect(),vec!["1,1,1","2,2,2","3,3,3","4,4,4","5,5,5","6,6,6","7,7,7","8,8,8","1,1,1","2,2,2","3,3,3"]),
        (numbered.iter().map(|s|s.to_string()).collect(),vec!["1,1,1","2,2,2","3,3,3","4,4,4","5,5,5","6,6,6","7,7,7","8,8,8","9,9,9","10,10,10","1,1,1"]),
        (default_colors.iter().map(|s|s.to_string()).collect(),default_rgb.to_vec()),
    ];
    assert_eq!(diagram_ir::VennConfig::default().theme_colors,default_colors.map(str::to_string).to_vec());
    for (palette,expected_rgb) in cases { for authored in [false,true] { for count in [0,11] {
        let config=diagram_ir::VennConfig {width:400.0,height:300.0,theme_colors:palette.clone(),..Default::default()};
        let mut circles=Vec::new();let mut instructions=Vec::new();
        for (i,rgb) in expected_rgb.iter().take(count).enumerate() {
            let override_fill=authored && (i==1 || i==8);
            let (cx,cy,radius)=(20.0+i as f64*25.0,100.0,7.0);
            circles.push(diagram_ir::LayoutedVennCircle {id:format!("set{i}"),label:format!("L{i}"),cx,cy,radius,
                style:diagram_ir::VennStyle {fill:override_fill.then(||"#ffffff".into()),fill_opacity:override_fill.then_some(0.5),..Default::default()}});
            instructions.push(PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx,cy,rx:radius,ry:radius,
                fill:Some(if override_fill {"rgba(255,255,255,0.5)".into()} else {format!("rgba({rgb},0.38)")}),
                stroke:Some("#334155".into()),stroke_width:Some(2.0),stroke_dash:None,stroke_dash_offset:None}));
        }
        let mut metadata=HashMap::from([("venn.width".into(),"400".into()),("venn.height".into(),"300".into()),
            ("venn.padding".into(),"8".into()),("venn.useMaxWidth".into(),"true".into()),("venn.useDebugLayout".into(),"false".into()),
            ("venn.fontFamily".into(),"sans-serif".into()),("venn.titleTextColor".into(),"#0f172a".into()),("venn.setTextColor".into(),"#0f172a".into())]);
        for (i,color) in palette.iter().enumerate() {metadata.insert(format!("venn.themeColor{}",i+1),color.clone());}
        assert_eq!(metadata.len(),8+palette.len());
        let expected=PaintScene {width:400.0,height:300.0,background:"rgb(255,255,255)".into(),instructions,id:None,metadata:Some(metadata)};
        let layout=LayoutedVennDiagram {width:400.0,height:300.0,title:None,config,circles,labels:vec![]};
        let resolver=RecordingResolver(RefCell::new(Vec::new()));
        let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
        let options=DiagramToPaintOptions {background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,
            label_font:defaults.label_font,title_font:defaults.title_font,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        assert_eq!(diagram_to_paint_venn(&layout,&options),expected,"P={} authored={authored} count={count}",palette.len());
        assert!(resolver.0.borrow().is_empty(),"complete empty font-query trace");
    }}}
}
