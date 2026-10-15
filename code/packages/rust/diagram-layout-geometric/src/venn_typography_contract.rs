// A new layout field must enter the same derived-result predicate as every
// prior coordinate/radius. These controls precede the predicate repair.
use super::*;
use diagram_ir::{VennConfig,VennRegion,VennStyle,VennText};
fn forged_layout() -> LayoutedVennDiagram {
    LayoutedVennDiagram {width:400.0,height:300.0,title:None,config:VennConfig::default(),circles:vec![],labels:vec![],debug_circles:vec![],debug_cells:vec![]}
}
fn text(sets:&[&str],id:usize) -> VennText {
    VennText {sets:sets.iter().map(|s|s.to_string()).collect(),id:format!("t{id}"),label:None,style:VennStyle::default()}
}
fn diagram(texts:Vec<VennText>) -> VennDiagram {
    VennDiagram {title:None,config:VennConfig::default(),regions:vec![],texts}
}
#[test]
fn venn_typography_forged_fonts_obey_the_derived_result_predicate() {
    for font_size in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0] {
        let mut layout=forged_layout();
        layout.labels.push(LayoutedVennLabel {text:"A".into(),x:1.0,y:2.0,font_size,style:VennStyle::default()});
        assert_eq!(validate_venn_geometry(&layout),Err(VennLayoutError::DerivedGeometry),"font={font_size}");
    }
    for font_size in [0.0,12.0] {
        let mut layout=forged_layout();
        layout.labels.push(LayoutedVennLabel {text:"A".into(),x:1.0,y:2.0,font_size,style:VennStyle::default()});
        assert_eq!(validate_venn_geometry(&layout),Ok(()));
    }
}
#[test]
fn venn_typography_existing_width_bounds_generate_finite_sizes_without_a_new_cap() {
    for (width,set_font,text_font) in [(400.0,12.0,10.0),(1600.0,48.0,40.0),(65536.0,1966.08,1638.4),(f64::from_bits(1),0.0,0.0)] {
        let mut d=diagram(vec![text(&["A"],0)]);d.config.width=width;
        d.regions.push(VennRegion {sets:vec!["A".into()],size:1.0,label:Some("A".into()),style:VennStyle::default()});
        let got=try_layout_venn(&d).unwrap();
        assert_eq!(got.labels.iter().map(|l|l.font_size).collect::<Vec<_>>(),vec![set_font,text_font]);
    }
    let mut d=diagram(vec![]);d.config.width=65537.0;
    assert_eq!(try_layout_venn(&d),Err(VennLayoutError::Width));
}
