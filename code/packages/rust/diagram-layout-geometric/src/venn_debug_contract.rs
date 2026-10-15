//! Independent debug geometry: structural identity, authored order and real work.
use super::*;
use diagram_ir::{VennConfig, VennRegion, VennStyle, VennText};

fn text(sets: &[&str], id: usize) -> VennText {
    VennText { sets: sets.iter().map(|s| s.to_string()).collect(), id: format!("t{id}"), label: None, style: VennStyle::default() }
}
fn circle(id: &str, cx: f64, cy: f64, radius: f64) -> LayoutedVennCircle {
    LayoutedVennCircle { id: id.into(), label: id.into(), cx, cy, radius, style: VennStyle::default() }
}
fn diagram(texts: Vec<VennText>) -> VennDiagram {
    VennDiagram { title: None, config: VennConfig { width: 1600.0, use_debug_layout: true, ..Default::default() }, regions: vec![], texts }
}
fn debug_circle(cx: f64, cy: f64, radius: f64) -> LayoutedVennDebugCircle { LayoutedVennDebugCircle { cx, cy, radius } }
fn cell(x: f64, y: f64, width: f64, height: f64) -> LayoutedVennDebugCell { LayoutedVennDebugCell { x, y, width, height } }

#[test]
fn venn_debug_structural_groups_do_not_alias_nul_or_reorder_authored_vectors() {
    let circles = vec![circle("A\0B", 10.0, 20.0, 8.0), circle("A", 30.0, 20.0, 8.0), circle("B", 50.0, 20.0, 8.0)];
    let centers = circles.iter().map(|c| (c.id.as_str(), (c.cx, c.cy))).collect();
    let d = diagram(vec![text(&["A\0B"], 0), text(&["A", "B"], 1)]);
    assert_eq!(venn_debug_geometry(&d, &circles, &centers), (
        vec![debug_circle(10.0, 20.0, 8.0), debug_circle(40.0, 20.0, 4.8)],
        vec![cell(-30.0, 20.0, 80.0, 60.0), cell(0.0, 20.0, 80.0, 60.0)]));
    // Ordered vectors, duplicate members and partially unknown vectors retain
    // their authored identities; entirely unknown and empty groups emit nothing.
    let d = diagram(vec![text(&["B", "A"], 0), text(&["A", "B"], 1), text(&["A", "A"], 2),
        text(&["unknown"], 3), text(&[], 4), text(&["A", "unknown"], 5)]);
    assert_eq!(venn_debug_geometry(&d, &circles, &centers), (
        vec![debug_circle(40.0, 20.0, 4.8), debug_circle(40.0, 20.0, 4.8), debug_circle(30.0, 20.0, 8.0), debug_circle(30.0, 20.0, 8.0)],
        vec![cell(0.0, 20.0, 80.0, 60.0), cell(0.0, 20.0, 80.0, 60.0), cell(-10.0, 20.0, 80.0, 60.0), cell(-10.0, 20.0, 80.0, 60.0)]));
}

#[test]
fn venn_debug_grid_transitions_have_complete_independent_geometry() {
    // At width1920 scale1.2, the minimum text box is96 by72. Radius20
    // contributes no larger box. The table fixes ceil-grid transitions1..5.
    let cases = [vec![cell(52.0,100.0,96.0,72.0)],
        vec![cell(52.0,100.0,48.0,72.0),cell(100.0,100.0,48.0,72.0)],
        vec![cell(52.0,64.0,48.0,36.0),cell(100.0,64.0,48.0,36.0),cell(52.0,100.0,48.0,36.0)],
        vec![cell(52.0,64.0,48.0,36.0),cell(100.0,64.0,48.0,36.0),cell(52.0,100.0,48.0,36.0),cell(100.0,100.0,48.0,36.0)],
        vec![cell(52.0,64.0,32.0,36.0),cell(84.0,64.0,32.0,36.0),cell(116.0,64.0,32.0,36.0),cell(52.0,100.0,32.0,36.0),cell(84.0,100.0,32.0,36.0)]];
    let circles = vec![circle("A",100.0,100.0,20.0)]; let centers = HashMap::from([("A",(100.0,100.0))]);
    for (index, expected) in cases.into_iter().enumerate() { for labeled in [false,true] {
        let mut d = diagram((0..=index).map(|i|text(&["A"],i)).collect()); d.config.width=1920.0;
        d.regions.push(VennRegion { sets:vec!["A".into()],size:1.0,label:labeled.then(||"L".into()),style:VennStyle::default() });
        let mut gold=expected.clone();if labeled {for c in &mut gold {c.y+=5.0;}}
        assert_eq!(venn_debug_geometry(&d,&circles,&centers),(vec![debug_circle(100.0,100.0,20.0)],gold),"count={} labeled={labeled}",index+1);
    }}
}

#[test]
fn venn_debug_duplicate_lookup_and_nonempty_label_or_preserve_existing_policy() {
    // Centers retain the last duplicate ID while radius lookup retains first.
    let circles=vec![circle("A",0.0,0.0,8.0),circle("A",10.0,0.0,100.0)];
    let centers=HashMap::from([("A",(10.0,0.0))]);let d=diagram(vec![text(&["A"],0)]);
    assert_eq!(venn_debug_geometry(&d,&circles,&centers),(vec![debug_circle(10.0,0.0,4.8)],vec![cell(-30.0,0.0,80.0,60.0)]));
    let circles=vec![circle("A",10.0,20.0,8.0),circle("A",10.0,20.0,100.0)];
    let centers=HashMap::from([("A",(10.0,20.0))]);let mut d=diagram(vec![text(&["A"],0)]);
    for label in [None,Some(String::new()),Some("later".into())] {d.regions.push(VennRegion {sets:vec!["A".into()],size:1.0,label,style:VennStyle::default()});}
    assert_eq!(venn_debug_geometry(&d,&circles,&centers),(vec![debug_circle(10.0,20.0,8.0)],vec![cell(-30.0,22.0,80.0,60.0)]));
}

#[test]
fn venn_debug_disabled_and_empty_inputs_emit_exact_empty_vectors() {
    for enabled in [false,true] {
        let mut d=diagram(vec![]);d.config.use_debug_layout=enabled;
        let got=try_layout_venn(&d).unwrap();assert_eq!((got.debug_circles,got.debug_cells),(vec![],vec![]));
        d.config.use_debug_layout=false;d.texts.push(text(&["A"],0));
        d.regions.push(VennRegion {sets:vec!["A".into()],size:1.0,label:None,style:VennStyle::default()});
        let got=try_layout_venn(&d).unwrap();assert_eq!((got.debug_circles,got.debug_cells),(vec![],vec![]));
    }
}

#[test]
fn venn_debug_work_tracks_actual_visits_with_linear_input_and_output() {
    for count in [8,32,128] {
        let ids:Vec<_>=(0..count).map(|i|format!("s{i}")).collect();
        let circles:Vec<_>=ids.iter().map(|id|circle(id,100.0,100.0,20.0)).collect();
        let centers=circles.iter().map(|c|(c.id.as_str(),(c.cx,c.cy))).collect();
        let mut d=diagram(ids.iter().enumerate().map(|(i,id)|text(&[id],i)).collect());
        d.regions=ids.iter().map(|id|VennRegion {sets:vec![id.clone()],size:1.0,label:None,style:VennStyle::default()}).collect();
        VENN_DEBUG_VISITS.with(|v|v.set(0));let (areas,cells)=venn_debug_geometry(&d,&circles,&centers);
        assert_eq!(areas,vec![debug_circle(100.0,100.0,20.0);count]);assert_eq!(cells,vec![cell(60.0,100.0,80.0,60.0);count]);
        let visits=VENN_DEBUG_VISITS.with(std::cell::Cell::get);
        assert!(visits>0 && visits<=16*count,"{count} linear-output groups made {visits} real visits");
    }
}

fn forged_layout() -> LayoutedVennDiagram {
    LayoutedVennDiagram {width:400.0,height:300.0,title:None,config:VennConfig::default(),circles:vec![],labels:vec![],debug_circles:vec![debug_circle(1.0,2.0,3.0)],debug_cells:vec![cell(1.0,2.0,3.0,4.0)]}
}
#[test]
fn venn_debug_checked_result_rejects_every_invalid_new_field() {
    for field in 0..7 {for value in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0] {
        if value==-1.0 && matches!(field,0|1|3|4) {continue;}
        let mut l=forged_layout();match field {0=>l.debug_circles[0].cx=value,1=>l.debug_circles[0].cy=value,2=>l.debug_circles[0].radius=value,
            3=>l.debug_cells[0].x=value,4=>l.debug_cells[0].y=value,5=>l.debug_cells[0].width=value,_=>l.debug_cells[0].height=value}
        assert_eq!(validate_venn_geometry(&l),Err(VennLayoutError::DerivedGeometry),"field={field} value={value}");
    }}
}
#[test]
fn venn_debug_checked_result_keeps_finite_negative_positions_and_zero_sizes() {
    let mut l=forged_layout();l.debug_circles=vec![debug_circle(-1.0,-2.0,0.0)];l.debug_cells=vec![cell(-3.0,-4.0,0.0,0.0)];
    assert_eq!(validate_venn_geometry(&l),Ok(()));
}
