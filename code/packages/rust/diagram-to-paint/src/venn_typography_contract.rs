// The font tables are independent of production. Fake metrics have ascent0.8em
// and each one-character glyph advances0.5em, hence centered x is center-size/4.
// Keep complete geometry, style, glyph order and resolver identities in every cell.
#[test]
fn venn_typography_sources_have_complete_scenes_and_ordered_queries() {
    let palette=["#112233","#445566","#34d399","#f472b6","#a78bfa","#22d3ee","#fb7185","#a3e635"].map(str::to_string).to_vec();
    for (width,set_size,text_size,title_size,cx_a,cx_b,label_a,label_b,tcx_a,tcx_b,tlabel_a,tlabel_b) in [
        (400.0,12.0,10.0,8.0,240.0,160.0,276.25,123.75,234.56,165.44,265.88,134.12),
        (800.0,24.0,20.0,16.0,440.0,360.0,476.25,323.75,434.56,365.44,465.88,334.12),
        (1600.0,48.0,40.0,32.0,840.0,760.0,876.25,723.75,834.56,765.44,865.88,734.12)] {
        for family in ["Georgia","CourierNew"] {for titled in [false,true] {for authored in [false,true] {
            let source=format!("---\nconfig:\n  venn:\n    width: {width}\n    height: 300\n    padding: 25\nthemeVariables:\n  fontFamily: {family}\n  venn1: '#112233'\n  venn2: '#445566'\n---\nvenn-beta\n{}set a[\"A\"]: 1\nset b[\"B\"]: 1\nunion a,b[\"U\"]: 0.5\ntext n[\"N\"]\n{}",if titled {"title T\n"} else {""},if authored {"style a color:#ff0000\nstyle n color:#00ff00"} else {""});
            let parsed=mermaid_parser::parse_venn(&source).unwrap();
            let layout=diagram_layout_geometric::try_layout_venn(&parsed).unwrap();
            let (left,right,left_label,right_label,cy,radius)=if titled {(tcx_a,tcx_b,tlabel_a,tlabel_b,167.0,54.0)} else {(cx_a,cx_b,label_a,label_b,150.0,62.5)};
            let ellipse=|cx,rgb:&str,stroke:&str|PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx,cy,rx:radius,ry:radius,fill:Some(format!("rgba({rgb},0.1)")),stroke:Some(stroke.into()),stroke_width:Some(width/320.0),stroke_dash:None,stroke_dash_offset:None});
            let mut instructions=vec![ellipse(left,"17,34,51","#112233"),ellipse(right,"68,85,102","#445566")];
            let glyph=|c:char,cx,top,size,fill:&str|PaintInstruction::GlyphRun(PaintGlyphRun {base:PaintBase::default(),glyphs:vec![GlyphPosition {glyph_id:c as u32,x:cx-size/4.0,y:top+size*0.8}],font_ref:"fake:test".into(),font_size:size,fill:Some(fill.into())});
            if titled {instructions.push(glyph('T',width/2.0,5.0,title_size,"rgb(15, 23, 42)"));}
            instructions.push(glyph('A',left_label,cy-12.0,set_size,if authored {"rgb(255, 0, 0)"} else {"rgb(15, 23, 42)"}));
            instructions.push(glyph('B',right_label,cy-12.0,set_size,"rgb(15, 23, 42)"));
            instructions.push(glyph('U',width/2.0,cy-12.0,set_size,"rgb(15, 23, 42)"));
            let normal_y=match (width,titled) { (400.0,false)=>163.125,(800.0,false)=>170.625,(1600.0,false)=>185.625,(400.0,true)=>f64::from_bits(0x40666b851eb851eb),(800.0,true)=>f64::from_bits(0x40675b851eb851eb),(1600.0,true)=>f64::from_bits(0x40693b851eb851ec),_=>panic!("undeclared old source fixture") };
            instructions.push(glyph('N',width/2.0,normal_y-12.0,text_size,if authored {"rgb(0, 255, 0)"} else {"rgb(15, 23, 42)"}));
            let expected=PaintScene {width,height:300.0,background:"rgb(255,255,255)".into(),instructions,id:None,metadata:Some(venn_debug_metadata(width,300.0,25.0,false,family,&palette))};
            let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));
            let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
            let options=DiagramToPaintOptions {background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,
                label_font:FontSpec {family:"Ignored Caller".into(),weight:600,italic:true,stretch:FontStretch::Condensed,..defaults.label_font},
                title_font:FontSpec {family:"Ignored Title".into(),weight:800,italic:true,stretch:FontStretch::Expanded,..defaults.title_font},shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
            assert_eq!(diagram_to_paint_venn(&layout,&options),expected,"complete typography width={width} family={family} title={titled} authored={authored}");
            let mut queries=vec![];
            if titled {queries.push(FontQuery::named(family).with_weight(text_interfaces::FontWeight(800)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Expanded));}
            queries.push(FontQuery::named(family).with_weight(text_interfaces::FontWeight(600)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Condensed));
            assert_eq!(*resolver.0.borrow(),queries,"complete ordered typography resolver trace");
        }}}
    }
}
