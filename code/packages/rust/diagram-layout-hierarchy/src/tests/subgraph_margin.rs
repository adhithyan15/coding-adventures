//! The imported title margins must remain inside the checked layout contract.
use super::*;

#[test]
fn imported_subgraph_margin_exact_typed_admission() {
    let mut failures=Vec::new();
    for direction in [DiagramDirection::Lr,DiagramDirection::Rl,DiagramDirection::Tb,DiagramDirection::Bt] {
        for titled in [false,true] {
            for field in ["subGraphTitleMargin.top","subGraphTitleMargin.bottom"] {
                for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0,65_536.000_001] {
                    let mut input=ordering_diagram(2,&[(0,1,1)]);
                    input.direction=direction.clone();input.title=titled.then(||"Title".into());
                    let slot=if field.ends_with(".top") {&mut input.flowchart_config.subgraph_title_margin_top}
                        else {&mut input.flowchart_config.subgraph_title_margin_bottom};
                    *slot=Some(bad);
                    let before=input.clone();
                    match std::panic::catch_unwind(||try_layout_swimlane(&input)) {
                        Ok(Err(SwimlaneLayoutError::UnsupportedSpacing {field:actual})) if actual==field=>{},
                        other=>{
                            let detail=match other {Ok(Ok(_))=>"accepted".to_owned(),Ok(Err(e))=>format!("wrong error: {e}"),Err(_)=>"panicked".to_owned()};
                            failures.push((direction.clone(),titled,field,bad.to_bits(),detail));
                        },
                    }
                    assert_eq!(input.flowchart_config.subgraph_title_margin_top.map(f64::to_bits),before.flowchart_config.subgraph_title_margin_top.map(f64::to_bits));
                    assert_eq!(input.flowchart_config.subgraph_title_margin_bottom.map(f64::to_bits),before.flowchart_config.subgraph_title_margin_bottom.map(f64::to_bits));
                    let mut a=input;let mut b=before;
                    a.flowchart_config.subgraph_title_margin_top=None;a.flowchart_config.subgraph_title_margin_bottom=None;
                    b.flowchart_config.subgraph_title_margin_top=None;b.flowchart_config.subgraph_title_margin_bottom=None;
                    assert_eq!(a,b);
                }
            }
        }
    }
    assert!(failures.is_empty(),"invalid margin admissions {}/80: {failures:?}",failures.len());
}

fn inside(outer:[f64;4],inner:[f64;4])->bool {
    outer.into_iter().chain(inner).all(f64::is_finite)
        && inner[2]>=0.0 && inner[3]>=0.0
        && inner[0]>=outer[0] && inner[1]>=outer[1]
        && inner[0]+inner[2]<=outer[0]+outer[2]
        && inner[1]+inner[3]<=outer[1]+outer[3]
}

#[test]
fn imported_subgraph_margin_nested_geometry_and_default_identity() {
    let mut failures=Vec::new();let mut cells=0;
    for direction in [DiagramDirection::Lr,DiagramDirection::Rl,DiagramDirection::Tb,DiagramDirection::Bt] {
        for ranked in [false,true] {
            for depth in 0..=2 {
                for empty in [false,true] {
                    let mut baseline=ordering_diagram(2,if ranked {&[(0,1,1)]} else {&[]});
                    baseline.direction=direction.clone();baseline.automatic_lane_ordering=false;
                    baseline.ignore_cross_lane_edges = !ranked;
                    for i in 0..depth {
                        baseline.groups.push(diagram_ir::SwimlaneGroup {
                            id:format!("group{i}"),label:format!("Group{i}"),lane_id:"lane0".into(),
                            parent_id:(i>0).then(||format!("group{}",i-1)),
                            node_ids:if !empty && i+1==depth {vec!["node0".into()]} else {vec![]},
                            classes:vec!["owned".into()],style:Default::default(),
                        });
                    }
                    for padding in [None,Some(100.0),Some(65_536.0)] {
                    for shape in [DiagramShape::Rect,DiagramShape::Summary,DiagramShape::ForkJoin,DiagramShape::LightningBolt] {
                    baseline.flowchart_config.padding=padding;baseline.nodes[0].shape=shape.clone();
                    for outer in [None,Some(0.0),Some(12.0),Some(65_536.0)] {
                    baseline.flowchart_config.diagram_padding=outer;
                    let golden=try_layout_swimlane(&baseline).unwrap();
                    for (top,bottom) in [(None,None),(Some(-0.0),Some(0.0)),(Some(0.0),Some(-0.0)),
                        (Some(14.0),Some(7.0)),(Some(100.0),Some(0.0)),(Some(0.0),Some(100.0)),(Some(65_536.0),Some(65_536.0))] {
                        cells+=1;let mut input=baseline.clone();
                        input.flowchart_config.subgraph_title_margin_top=top;
                        input.flowchart_config.subgraph_title_margin_bottom=bottom;
                        let before=input.clone();let actual=try_layout_swimlane(&input).unwrap();
                        assert_eq!(input,before);
                        assert_eq!(input.flowchart_config.subgraph_title_margin_top.map(f64::to_bits),top.map(f64::to_bits));
                        assert_eq!(input.flowchart_config.subgraph_title_margin_bottom.map(f64::to_bits),bottom.map(f64::to_bits));
                        let sum=top.unwrap_or(0.0)+bottom.unwrap_or(0.0);
                        if sum==0.0 {
                            let mut expected=golden.clone();expected.flowchart_config=input.flowchart_config.clone();
                            assert_eq!(actual,expected,"complete zero/default identity");
                            continue;
                        }
                        let canvas=[0.0,0.0,actual.width,actual.height];
                        let mut ok=true;
                        for lane in &actual.lanes {
                            let rect=[lane.x,lane.y,lane.width,lane.height];
                            ok &= inside(canvas,rect);
                            ok &= inside(rect,[lane.x+8.0,lane.y+7.0+top.unwrap_or(0.0),lane.width-16.0,26.0]);
                        }
                        for node in &actual.nodes {
                            let owner=input.nodes.iter().find(|n|n.id==node.id).unwrap().lane_id.as_deref().unwrap();
                            let lane=actual.lanes.iter().find(|l|l.id==owner).unwrap();
                            ok &= inside([lane.x,lane.y,lane.width,lane.height],[node.x,node.y,node.width,node.height]);
                        }
                        for group in &actual.groups {
                            let rect=[group.x,group.y,group.width,group.height];
                            let lane=actual.lanes.iter().find(|l|l.id==group.lane_id).unwrap();
                            ok &= inside([lane.x,lane.y,lane.width,lane.height],rect);
                            let title_end=group.y+4.0+top.unwrap_or(0.0)+22.0;
                            ok &= inside(rect,[group.x+8.0,group.y+4.0+top.unwrap_or(0.0),group.width-16.0,22.0]);
                            let source=input.groups.iter().find(|g|g.id==group.id).unwrap();
                            for id in &source.node_ids {
                                let n=actual.nodes.iter().find(|n|n.id==*id).unwrap();
                                ok &= inside(rect,[n.x,n.y,n.width,n.height]) && title_end+bottom.unwrap_or(0.0)<=n.y;
                            }
                            for child in actual.groups.iter().filter(|g|g.parent_id.as_deref()==Some(group.id.as_str())) {
                                ok &= inside(rect,[child.x,child.y,child.width,child.height]) && title_end+bottom.unwrap_or(0.0)<=child.y;
                            }
                        }
                        if !ok {failures.push((direction.clone(),ranked,depth,empty,padding,shape.clone(),outer,top,bottom));}
                    }
                    }
                    }
                    }
                }
            }
        }
    }
    assert_eq!(cells,16128);
    assert!(failures.is_empty(),"nested geometry failed {}/16128 cells; first20: {:?}",failures.len(),&failures[..failures.len().min(20)]);
}
