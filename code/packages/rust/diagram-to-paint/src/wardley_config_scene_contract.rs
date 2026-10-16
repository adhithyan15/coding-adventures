// Literal causal drawing tables specified before observing Wardley output.
mod wardley_config_scene_contract {
use super::*;
struct WardleyShaper;
impl TextShaper for WardleyShaper {
    type Handle=FakeHandle;
    fn shape(&self,text:&str,_:&FakeHandle,size:f32,opts:&ShapeOptions)->Result<ShapedText,ShapingError> {
        if opts.direction!=Direction::Ltr {return Err(ShapingError::UnsupportedDirection(opts.direction));}
        let advance=size/4.0;
        Ok(ShapedText::single(ShapedRun {glyphs:text.chars().enumerate().map(|(i,c)|Glyph {
            glyph_id:c as u32,cluster:i as u32,x_advance:advance,y_advance:0.0,x_offset:0.0,y_offset:0.0}).collect(),
            x_advance_total:text.chars().count() as f32*advance,font_ref:"fake:test".into()}))
    }
    fn font_ref(&self,_:&FakeHandle)->String {"fake:test".into()}
}
struct Case {name:&'static str,source:&'static str,config:diagram_ir::WardleyConfig,populated:bool,
w:f64,h:f64,lo:f64,hi_x:f64,hi_y:f64,mid:f64,stages:[f64;4],stage_glyph_x:[f64;4],stage_y:f64,
stage_font:f64,label_font:f64,axis_font:f64,title_font:f64,title_x:f64,title_y:f64,
node_x:[f64;2],node_y:[f64;2],axis_x:f64,axis_y:f64,anchor_radius:f64,grid:Vec<(f64,f64)>}
fn p(x:f64,y:f64)->Point {Point{x,y}}
fn path(points:Vec<Point>,stroke:&str,width:f64,dashed:bool)->PaintInstruction {
 PaintInstruction::Path(PaintPath{base:PaintBase::default(),commands:points.into_iter().enumerate().map(|(i,p)|
 if i==0 {PathCommand::MoveTo{x:p.x,y:p.y}}else{PathCommand::LineTo{x:p.x,y:p.y}}).collect(),
 fill:Some("none".into()),fill_rule:None,stroke:Some(stroke.into()),stroke_width:Some(width),
 stroke_cap:Some(StrokeCap::Round),stroke_join:Some(StrokeJoin::Round),stroke_dash:dashed.then(||vec![2.0,6.0]),stroke_dash_offset:None})
}
fn glyphs(text:&str,x:f64,y:f64,size:f64,color:&str)->PaintInstruction {
 let advance=(size as f32/4.0) as f64;
 PaintInstruction::GlyphRun(PaintGlyphRun{base:PaintBase::default(),glyphs:text.chars().enumerate().map(|(i,c)|
 GlyphPosition{glyph_id:c as u32,x:x+i as f64*advance,y}).collect(),font_ref:"fake:test".into(),font_size:size as f32 as f64,fill:Some(color.into())})
}
fn ellipse(x:f64,y:f64,radius:f64,_anchor:bool)->PaintInstruction {PaintInstruction::Ellipse(PaintEllipse{
base:PaintBase::default(),cx:x,cy:y,rx:radius,ry:radius,fill:Some("#ffffff".into()),
stroke:Some("#000000".into()),stroke_width:Some(1.0),stroke_dash:None,stroke_dash_offset:None})}
#[test]
fn wardley_whole_source_ir_layout_scene_and_cached_queries() {
 let cases=vec![
Case{name:"empty-default",source:"wardley-beta\n",config:diagram_ir::WardleyConfig{width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},populated:false,w:900.0,h:600.0,lo:48.0,hi_x:852.0,hi_y:552.0,mid:450.0,stage_y:568.0,stage_font:10.0,label_font:10.0,axis_font:12.0,title_font:12.600000000000001,title_x:448.4249999523163,title_y:15.080000000000002,axis_x:15.0,axis_y:57.6,anchor_radius:8.100000000000001,stages:[48.0,316.0,584.0,852.0],stage_glyph_x:[39.25,301.0,575.25,840.75],node_x:[129.75,933.75],node_y:[40.0,544.0],grid:vec![]},
Case{name:"populated-default",source:"wardley-beta\ntitle T\nanchor A [1,0]\ncomponent B [0,1]\nA -> B\nevolve A 0.5\n",config:diagram_ir::WardleyConfig{width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},populated:true,w:900.0,h:600.0,lo:48.0,hi_x:852.0,hi_y:552.0,mid:450.0,stage_y:568.0,stage_font:10.0,label_font:10.0,axis_font:12.0,title_font:12.600000000000001,title_x:448.4249999523163,title_y:15.080000000000002,axis_x:15.0,axis_y:57.6,anchor_radius:8.100000000000001,stages:[48.0,316.0,584.0,852.0],stage_glyph_x:[39.25,301.0,575.25,840.75],node_x:[129.75,933.75],node_y:[40.0,544.0],grid:vec![]},
Case{name:"custom-grid-body-precedence",source:"%%{init: {\"wardley-beta\":{\"width\":700.0,\"height\":500.0,\"padding\":40.0,\"nodeRadius\":7.0,\"nodeLabelOffset\":12.0,\"axisFontSize\":20.0,\"labelFontSize\":12.0,\"showGrid\":true,\"useMaxWidth\":false}}}%%\nwardley-beta\ntitle T\nanchor A [1,0]\ncomponent B [0,1]\nA -> B\nevolve A 0.5\nsize [620,400]\n",config:diagram_ir::WardleyConfig{width:620.0,height:400.0,padding:40.0,node_radius:7.0,node_label_offset:12.0,axis_font_size:20.0,label_font_size:12.0,show_grid:true,use_max_width:false,theme:diagram_ir::WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},populated:true,w:620.0,h:400.0,lo:40.0,hi_x:580.0,hi_y:360.0,mid:310.0,stage_y:382.4,stage_font:18.0,label_font:12.0,axis_font:20.0,title_font:21.0,title_x:307.375,title_y:21.8,axis_x:5.0,axis_y:56.0,anchor_radius:9.450000000000001,stages:[40.0,220.0,400.0,580.0],stage_glyph_x:[31.25,193.0,384.25,552.75],node_x:[125.5,665.5],node_y:[33.6,353.6],grid:vec![(148.0,296.0),(256.0,232.0),(364.0,168.0),(472.0,104.0)]},
Case{name:"minimum-positive-fonts",source:"%%{init: {\"wardley-beta\":{\"width\":420.0,\"height\":300.0,\"padding\":0.0,\"nodeRadius\":1.0,\"nodeLabelOffset\":0.0,\"axisFontSize\":1.0,\"labelFontSize\":1.0,\"showGrid\":false,\"useMaxWidth\":false}}}%%\nwardley-beta\ntitle T\nanchor A [1,0]\ncomponent B [0,1]\nA -> B\nevolve A 0.5\n",config:diagram_ir::WardleyConfig{width:420.0,height:300.0,padding:0.0,node_radius:1.0,node_label_offset:0.0,axis_font_size:1.0,label_font_size:1.0,show_grid:false,use_max_width:false,theme:diagram_ir::WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},populated:true,w:420.0,h:300.0,lo:0.0,hi_x:420.0,hi_y:300.0,mid:210.0,stage_y:308.8,stage_font:1.0,label_font:1.0,axis_font:1.0,title_font:1.05,title_x:209.86875000596046,title_y:5.84,axis_x:28.75,axis_y:0.8,anchor_radius:1.35,stages:[0.0,140.0,280.0,420.0],stage_glyph_x:[46.125,138.5,279.125,371.875],node_x:[74.875,494.875],node_y:[-15.2,284.8],grid:vec![]},
 ];
 let stage_names=["Genesis","Custom Built","Product","Commodity"];
 for family in ["Georgia","CourierNew"] {for c in &cases {
    let expected_ir=diagram_ir::WardleyDiagram {title:c.populated.then(||"T".into()),config:c.config.clone(),
      stages:stage_names.map(String::from).to_vec(),nodes:if c.populated {vec![
        diagram_ir::WardleyNode{id:"wardley-1".into(),label:"A".into(),visibility:1.0,evolution:0.0,anchor:true},
        diagram_ir::WardleyNode{id:"wardley-2".into(),label:"B".into(),visibility:0.0,evolution:1.0,anchor:false}]}else{vec![]},
      links:if c.populated {vec![diagram_ir::WardleyLink{source:"A".into(),target:"B".into()}]}else{vec![]},
      evolves:if c.populated {vec![diagram_ir::WardleyEvolution{component:"A".into(),target:0.5}]}else{vec![]},notes:vec![],annotations_box:None,annotations:vec![]};
    let parsed=mermaid_parser::parse_wardley(c.source).unwrap();assert_eq!(parsed,expected_ir,"wholeIR {} {family}",c.name);
    let expected_layout=LayoutedWardleyDiagram{width:c.w,height:c.h,title:c.populated.then(||"T".into()),config:c.config.clone(),
      stages:stage_names.map(String::from).to_vec(),nodes:if c.populated {vec![
        diagram_ir::LayoutedWardleyNode{id:"wardley-1".into(),label:"A".into(),position:p(c.lo,c.lo),anchor:true},
        diagram_ir::LayoutedWardleyNode{id:"wardley-2".into(),label:"B".into(),position:p(c.hi_x,c.hi_y),anchor:false}]}else{vec![]},
      links:if c.populated {vec![diagram_ir::LayoutedWardleyLink{from:p(c.lo,c.lo),to:p(c.hi_x,c.hi_y)}]}else{vec![]},
      evolves:if c.populated {vec![diagram_ir::LayoutedWardleyEvolution{from:p(c.lo,c.lo),to:p(c.mid,c.lo)}]}else{vec![]},notes:vec![],annotations_box:None,annotations:vec![]};
    let layout=diagram_layout_geometric::try_layout_wardley(&parsed).unwrap();assert_eq!(layout,expected_layout,"wholeLayout {} {family}",c.name);
    let mut instructions=vec![path(vec![p(c.lo,c.lo),p(c.lo,c.hi_y),p(c.hi_x,c.hi_y)],"#000000",1.0,false)];
    for (x,y) in &c.grid {instructions.push(path(vec![p(*x,c.lo),p(*x,c.hi_y)],"rgba(100, 100, 100, 0.2)",1.0,true));
        instructions.push(path(vec![p(c.lo,*y),p(c.hi_x,*y)],"rgba(100, 100, 100, 0.2)",1.0,true));}
    for x in &c.stages[1..] {instructions.push(path(vec![p(*x,c.lo),p(*x,c.hi_y)],"#000000",1.0,false));}
    if c.populated {instructions.push(path(vec![p(c.lo,c.lo),p(c.hi_x,c.hi_y)],"#000000",1.0,false));
        instructions.push(path(vec![p(c.lo,c.lo),p(c.mid,c.lo)],"#dc3545",1.0,false));
        instructions.push(ellipse(c.lo,c.lo,c.anchor_radius,true));instructions.push(ellipse(c.hi_x,c.hi_y,c.config.node_radius,false));
        instructions.push(glyphs("T",c.title_x,c.title_y,c.title_font,"rgb(34, 34, 34)"));}
    for (i,name) in stage_names.iter().enumerate() {instructions.push(glyphs(name,c.stage_glyph_x[i],c.stage_y,c.stage_font,"rgb(34, 34, 34)"));}
    if c.populated {instructions.push(glyphs("A",c.node_x[0],c.node_y[0],c.label_font,"rgb(34, 34, 34)"));
        instructions.push(glyphs("B",c.node_x[1],c.node_y[1],c.label_font,"rgb(34, 34, 34)"));}
    instructions.push(glyphs("Visibility",c.axis_x,c.axis_y,c.axis_font,"rgb(34, 34, 34)"));
    let expected=PaintScene{width:c.w,height:c.h,background:"#ffffff".into(),id:None,metadata:Some(HashMap::from([
        ("wardley.padding".into(),c.config.padding.to_string()),("wardley.nodeRadius".into(),c.config.node_radius.to_string()),
        ("wardley.nodeLabelOffset".into(),c.config.node_label_offset.to_string()),("wardley.axisFontSize".into(),c.config.axis_font_size.to_string()),
        ("wardley.labelFontSize".into(),c.config.label_font_size.to_string()),("wardley.showGrid".into(),c.config.show_grid.to_string()),
        ("wardley.useMaxWidth".into(),c.config.use_max_width.to_string()),
        ("wardley.backgroundColor".into(),c.config.theme.background_color.clone()),
        ("wardley.axisColor".into(),c.config.theme.axis_color.clone()),
        ("wardley.axisTextColor".into(),c.config.theme.axis_text_color.clone()),
        ("wardley.gridColor".into(),c.config.theme.grid_color.clone()),
        ("wardley.componentFill".into(),c.config.theme.component_fill.clone()),
        ("wardley.componentStroke".into(),c.config.theme.component_stroke.clone()),
        ("wardley.componentLabelColor".into(),c.config.theme.component_label_color.clone()),
        ("wardley.linkStroke".into(),c.config.theme.link_stroke.clone()),
        ("wardley.evolutionStroke".into(),c.config.theme.evolution_stroke.clone()),
        ("wardley.noteCount".into(),"0".into())])),instructions};
    let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
    let options=DiagramToPaintOptions {background:Color{r:3,g:5,b:7,a:255},device_pixel_ratio:1.0,
      label_font:FontSpec{family:family.into(),size:4.0,weight:600,italic:true,stretch:FontStretch::Condensed,..defaults.label_font},
      title_font:FontSpec{family:family.into(),size:6.0,weight:800,italic:true,stretch:FontStretch::Expanded,..defaults.title_font},
      shaper:&WardleyShaper,metrics:&FakeMetrics,resolver:&resolver};
    assert_eq!(diagram_to_paint_wardley(&layout,&options),expected,"wholeScene {} {family}",c.name);
    let mut queries=vec![];
    if c.populated {queries.push(FontQuery::named(family).with_weight(text_interfaces::FontWeight(800)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Expanded));}
    queries.push(FontQuery::named(family).with_weight(text_interfaces::FontWeight(600)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Condensed));
    assert_eq!(*resolver.0.borrow(),queries,"wholeOrderedCachedQueries {} {family}",c.name);
    eprintln!("WARDLEY-COMPLETE case={} family={family} wholeIR/layout/scene/query",c.name);
 }}
}
}
