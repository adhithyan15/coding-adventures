//! Exact arithmetic, independent endpoint, work and publication contracts.
use super::*;
use diagram_ir::{IshikawaCause,IshikawaConfig,IshikawaDiagram,IshikawaLook};

fn diagram(count:usize,look:IshikawaLook)->IshikawaDiagram {
    IshikawaDiagram {effect:"E".into(),causes:(0..count).map(|i|IshikawaCause {
        id:format!("n{i}"),label:format!("A{i}"),parent_id:None,depth:1,
    }).collect(),config:IshikawaConfig {look,..Default::default()}}
}
fn rectangle()->Vec<Point> {
    [(0.0,0.0),(10.0,0.0),(10.0,10.0),(0.0,10.0),(0.0,0.0)]
        .into_iter().map(|(x,y)|Point{x,y}).collect()
}
fn layout(d:&IshikawaDiagram,width:f64,limits:Limits)->Result<LayoutedIshikawaDiagram,IshikawaLayoutError> {
    super::super::try_layout_ishikawa_with_limits(d,width,limits)
}

#[test]
fn shared_collision_pair_count_has_exact_overflow_and_nearest_production_bounds() {
    for pair in [(0,0),(0,1),(1,0),(1,1)] {assert_eq!(pair_work(pair.0,pair.1,0),Ok(0));}
    assert_eq!(pair_work(2,1,1),Ok(1));
    assert_eq!(pair_work(2,2,1),Err(IshikawaLayoutError::CollisionWork));
    assert_eq!(pair_work(531,4062,8_388_608),Ok(8_388_606));
    assert_eq!(pair_work(1705,3725,8_388_608),Err(IshikawaLayoutError::CollisionWork));
    assert_eq!(pair_work(usize::MAX,usize::MAX,usize::MAX),Err(IshikawaLayoutError::CollisionWork));
    assert_eq!(pair_work(50_000,50_000,8_388_608),Err(IshikawaLayoutError::CollisionWork));
}

#[test]
fn public_both_looks_precharge_shared_pairs_and_never_clip_before_admission() {
    for look in [IshikawaLook::Classic,IshikawaLook::HandDrawn] {
        let limits=Limits{collision_pairs:1,..Limits::default()};
        reset_stats();assert!(layout(&diagram(3,look),640.0,limits).is_ok());
        assert_eq!(stats().collision_pairs,1);
        reset_stats();assert_eq!(layout(&diagram(4,look),640.0,limits),Err(IshikawaLayoutError::CollisionWork));
        assert_eq!(stats().collision_pairs,0);assert_eq!(stats().clip_lines,0);assert_eq!(stats().outline_builds,0);
    }
}

#[test]
fn signed_grid_validates_used_offsets_and_never_requires_an_unused_next() {
    let limit=(1_u64<<50) as f64;
    assert_eq!(grid(-limit,-limit+10.0),Err(IshikawaLayoutError::DerivedGeometry));
    let positive=grid(limit-10.0,limit).unwrap();
    assert_eq!((positive.first,positive.lines),(225_179_981_368_522,3));
    assert_eq!((positive.first+2) as f64*5.0,limit-4.0);
    assert_eq!(grid(-1.0,0.0).unwrap().first,-1);
    for (low,high) in [(f64::NAN,1.0),(0.0,f64::INFINITY),(2.0,1.0),(1e308,1e308)] {
        assert_eq!(grid(low,high),Err(IshikawaLayoutError::DerivedGeometry));
    }
}

#[test]
fn rectangle_clipping_has_independent_exact_counts_and_analytic_endpoints() {
    let polygon=rectangle();reset_stats();
    let result=finish_hatches(&[&polygon],Limits::default()).unwrap();
    assert_eq!(result.len(),1);assert_eq!(result[0].len(),2);
    let nx=41.0_f64.to_radians().sin();let ny=41.0_f64.to_radians().cos();
    let expected=[((0.0,5.0/ny),(5.0/nx,0.0)),(((10.0-10.0*ny)/nx,10.0),(10.0,(10.0-10.0*nx)/ny))];
    for (segment,(from,to)) in result[0].iter().zip(expected) {
        for (actual,want) in [(segment.from.x,from.0),(segment.from.y,from.1),(segment.to.x,to.0),(segment.to.y,to.1)] {
            assert!((actual-want).abs()<1e-12,"independent line/rectangle intersection: {actual} != {want}");
        }
    }
    assert_eq!(stats().clip_lines,6);assert_eq!(stats().edge_visits,24);
}

#[test]
fn hatch_line_work_actual_segment_limits_are_independent_and_aggregate() {
    let polygon=rectangle();
    for (limits,error) in [
        (Limits{lines:2,..Limits::default()},IshikawaLayoutError::HatchLines),
        (Limits{edge_visits:23,..Limits::default()},IshikawaLayoutError::HatchWork),
        (Limits{segments:1,..Limits::default()},IshikawaLayoutError::HatchSegments),
    ] {assert_eq!(finish_hatches(&[&polygon],limits),Err(error));}
    assert!(finish_hatches(&[&polygon],Limits{lines:3,edge_visits:24,segments:2,..Limits::default()}).is_ok());
    assert_eq!(finish_hatches(&[&polygon,&polygon],Limits{lines:5,..Limits::default()}),Err(IshikawaLayoutError::HatchLines));
    assert_eq!(finish_hatches(&[&polygon,&polygon],Limits{edge_visits:47,..Limits::default()}),Err(IshikawaLayoutError::HatchWork));
    assert_eq!(finish_hatches(&[&polygon,&polygon],Limits{segments:3,..Limits::default()}),Err(IshikawaLayoutError::HatchSegments));
    assert!(finish_hatches(&[&polygon,&polygon],Limits{lines:6,edge_visits:48,segments:4,..Limits::default()}).is_ok());
}

#[test]
fn all_outline_numeric_preflight_precedes_any_clipping_or_budget_shortcut() {
    let polygon=rectangle();let bad=[Point{x:f64::NAN,y:0.0}];reset_stats();
    assert_eq!(finish_hatches(&[&polygon,&bad],Limits{lines:0,..Limits::default()}),Err(IshikawaLayoutError::DerivedGeometry));
    assert_eq!(stats().clip_lines,0);
    assert_eq!(finish_hatches(&[&[]],Limits::default()).unwrap(),vec![vec![]]);
    assert_eq!(finish_hatches(&[&[Point{x:f64::NAN,y:0.0}]],Limits::default()),Err(IshikawaLayoutError::DerivedGeometry));
}

#[test]
fn cap_font_cross_side_separation_and_single_label_hatching_remain_supported() {
    for look in [IshikawaLook::Classic,IshikawaLook::HandDrawn] {
        let mut one=diagram(1,look);one.causes[0].label="A".into();one.config.font_size=65536.0;
        reset_stats();let result=layout(&one,576.0,Limits::default()).unwrap();
        assert_eq!(result.config.font_size,65536.0);assert_eq!(stats().outline_builds,2);
        assert_eq!(result.bones[0].label_position.y,180.0-57344.0-2.0);
        if look==IshikawaLook::HandDrawn {assert!(!result.bones[0].hatches.is_empty());assert_eq!(stats().hatch_passes,4);}
        else {assert!(result.bones[0].hatches.is_empty());assert_eq!(stats().clip_lines,0);}
        let mut two=diagram(2,look);two.config.font_size=65536.0;
        let result=layout(&two,576.0,Limits::default()).unwrap();
        let above=&result.bones[0];let below=&result.bones[1];
        assert!((below.label_position.y-below.label_height/2.0)-(above.label_position.y+above.label_height/2.0)>=4.0);
    }
}

#[test]
fn huge_classic_precision_failure_refuses_instead_of_claiming_overlap_clearance() {
    let mut d=diagram(1,IshikawaLook::Classic);
    for id in ["b","c"] {d.causes.push(IshikawaCause{id:id.into(),label:id.into(),parent_id:Some("n0".into()),depth:2});}
    reset_stats();assert_eq!(layout(&d,1e20,Limits::default()),Err(IshikawaLayoutError::DerivedGeometry));
    assert_eq!(stats().collision_pairs,0);assert_eq!(stats().clip_lines,0);
    assert!(layout(&d,640.0,Limits::default()).is_ok());
    assert!(layout(&diagram(0,IshikawaLook::Classic),1e12,Limits{lines:0,edge_visits:0,segments:0,..Limits::default()}).is_ok());
}

#[test]
fn new_allocation_failures_and_error_priority_publish_no_layout() {
    let limits=Limits{collision_pairs:0,..Limits::default()};let _failure=AllocationFailure::after(0);
    assert_eq!(layout(&diagram(4,IshikawaLook::HandDrawn),f64::NAN,limits),Err(IshikawaLayoutError::DerivedGeometry));
    assert_eq!(layout(&diagram(4,IshikawaLook::HandDrawn),640.0,limits),Err(IshikawaLayoutError::CollisionWork));
    assert_eq!(layout(&diagram(1,IshikawaLook::HandDrawn),640.0,Limits::default()),Err(IshikawaLayoutError::Allocation));
}

#[test]
fn invalid_font_fallback_preserves_the_complete_normalized_layout() {
    for look in [IshikawaLook::Classic,IshikawaLook::HandDrawn] {
        let mut d=diagram(2,look);let expected=layout(&d,640.0,Limits::default()).unwrap();
        for bad in [f64::NAN,f64::INFINITY,1e12,1e307,0.0,65537.0] {
            d.config.font_size=bad;assert_eq!(layout(&d,640.0,Limits::default()).unwrap(),expected);
        }
    }
}


#[test]
fn public_projection_domain_error_precedes_outline_descriptor_allocation() {
    // Initial scalars/extents remain finite and represented; the final fish-head
    // projection is beyond the hatch offset domain. No descriptor allocation
    // may conceal that numeric error, even on the zero-bone public route.
    let _failure=AllocationFailure::after(0);
    assert_eq!(layout(&diagram(0,IshikawaLook::HandDrawn),2e15,Limits::default()),Err(IshikawaLayoutError::DerivedGeometry));
}
