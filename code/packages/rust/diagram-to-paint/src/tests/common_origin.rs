//! Independent legacy predicate, directed-occurrence and work controls for the
//! reviewed integer common-origin cohort. The pre-repair work failures are real
//! executed evidence; a mathematical design review alone is not acceptance.
use super::*;

type Signature = Vec<(usize, usize, u64, u64, u64)>;
fn signature(values: Vec<(f64, Point, usize, usize)>) -> Signature {
    values.into_iter().map(|(t, p, edge, segment)|
        (edge, segment, t.to_bits(), p.x.to_bits(), p.y.to_bits())).collect()
}
fn bits(a: &Point, b: &Point) -> [u64; 4] {
    [a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits()]
}
fn transformed(mut a: Point, mut b: Point, axes: bool, reverse: bool) -> (Point, Point) {
    if axes { std::mem::swap(&mut a.x, &mut a.y); std::mem::swap(&mut b.x, &mut b.y); }
    if reverse { std::mem::swap(&mut a, &mut b); }
    (a, b)
}

#[test]
fn outside_right_empty_scene_and_work() {
    let mut failures = Vec::new();
    for count in [32usize, 64] {
        for axes in [false, true] {
            for reverse in [false, true] {
                for mode in [SwimlaneLineHops::Arc, SwimlaneLineHops::Gap] {
                    let parsed = mermaid_parser::parse_swimlane("swimlane-beta LR\nsubgraph L\na[A]\nb[B]\nend\na --> b").unwrap();
                    let mut layout = diagram_layout_hierarchy::layout_swimlane(&parsed);
                    let template = layout.edges[0].clone();
                    layout.width = layout.width.max((6 * count) as f64);
                    layout.height = layout.height.max((6 * count) as f64);
                    layout.edges = (0..count).map(|i| {
                        let mut edge = template.clone();
                        edge.id = Some(format!("outside-right-{i}"));
                        edge.curve = Some(SwimlaneEdgeCurve::Linear);
                        (edge.from, edge.to) = transformed(Point { x: 0.0, y: i as f64 },
                            Point { x: (4 * count + i) as f64, y: (i + 1) as f64 }, axes, reverse);
                        edge
                    }).collect();
                    let original = layout.clone();
                    for (i, edge) in layout.edges.iter().enumerate() {
                        assert_eq!(edge.id.as_deref(), Some(format!("outside-right-{i}").as_str()));
                        for earlier in &layout.edges[..i] {
                            assert!(swimlane_line_crossing(&edge.from, &edge.to, &earlier.from, &earlier.to).is_none());
                        }
                    }
                    layout.line_hops = Some(mode);
                    let enabled_input = layout.clone();
                    SWIMLANE_LINE_CROSSING_VISITS.with(|n| n.set(0));
                    SWIMLANE_HOP_INDEX_WORK.with(|n| n.set(0));
                    let mut enabled = diagram_to_paint_swimlane(&layout, &make_opts(&FakeShaper, &FakeMetrics, &FakeResolver));
                    let predicates = SWIMLANE_LINE_CROSSING_VISITS.with(std::cell::Cell::get);
                    let work = SWIMLANE_HOP_INDEX_WORK.with(std::cell::Cell::get);
                    assert_eq!(layout, enabled_input);
                    assert_eq!(enabled.metadata.as_mut().unwrap().remove("swimlane.lineHops").as_deref(),
                        Some(match mode { SwimlaneLineHops::Arc => "arc", SwimlaneLineHops::Gap => "gap" }));
                    layout.line_hops = None;
                    assert_eq!(layout, original, "every original raw endpoint/ID/style/occurrence survives");
                    let disabled = diagram_to_paint_swimlane(&layout, &make_opts(&FakeShaper, &FakeMetrics, &FakeResolver));
                    assert_eq!(enabled, disabled, "the entire scene, including labels/markers/metadata, survives");
                    let levels = (usize::BITS - count.leading_zeros()) as usize + 1;
                    eprintln!("outside-right count={count} axes={axes} reverse={reverse} mode={mode:?} predicates={predicates} index_work={work}");
                    if predicates > 2 * count * levels || work > 64 * count * levels {
                        failures.push((count, axes, reverse, mode, predicates, work));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "outside-right work failures {}/16: {failures:?}", failures.len());
}

#[test]
fn independent_occurrence_order_and_directed_bits_match_legacy() {
    // The oracle owns an independently grouped directed-bit inventory, not the
    // production tree, slots or extrema. Group first-seen order and duplicate
    // insertion order deliberately differ from globally sorted occurrence IDs.
    for axes in [false, true] {
        for permutation in 0..3 {
            let mut index = SwimlaneHopCrossings::default();
            let mut groups: Vec<(Point, Point, Vec<(usize, usize)>)> = Vec::new();
            for step in 0..48usize {
                let i = match permutation { 0 => step, 1 => 47-step, _ => step*17%48 };
                let start = (i % 13) as i64 - 6;
                let mut end = ((i * 19) % 61) as i64 - 30;
                if end == start { end += 1; } // observations are oblique
                let (a, b) = transformed(Point { x: if i%5==0 { -0.0 } else { 0.0 }, y: start as f64 },
                    Point { x: (100+i) as f64, y: end as f64 }, axes, i%3==0);
                let expected = groups.iter().filter_map(|(s,e,occurrences)|
                    swimlane_line_crossing(&a,&b,s,e).map(|(t,p)|
                        occurrences.iter().map(move |&(edge,segment)| (t,p.clone(),edge,segment))))
                    .flatten().collect();
                assert_eq!(signature(index.prior_crossings(&a,&b)), signature(expected), "step={step} permutation={permutation} axes={axes}");
                for occurrence in [(200-step,step%3),(400+step,7)] {
                    index.insert_geometry(occurrence.0,occurrence.1,&a,&b);
                    if let Some((_,_,entries)) = groups.iter_mut().find(|(s,e,_)| bits(s,e)==bits(&a,&b)) {
                        entries.push(occurrence);
                    } else { groups.push((a.clone(),b.clone(),vec![occurrence])); }
                }
                // Horizontal (or exchanged vertical) query exercises slope0,
                // with its direction reversed independently of observations.
                let (q0,q1) = transformed(Point{x:0.0,y:1.0},Point{x:200.0,y:1.0},axes,step%2==0);
                let expected=groups.iter().filter_map(|(s,e,occ)| swimlane_line_crossing(&q0,&q1,s,e)
                    .map(|(t,p)|occ.iter().map(move |&(edge,seg)|(t,p.clone(),edge,seg)))).flatten().collect();
                assert_eq!(signature(index.prior_crossings(&q0,&q1)),signature(expected));
            }
            // Uncertified and different-left geometries must retain the full
            // legacy result rather than silently selecting a partial cohort.
            for (a,b) in [(Point{x:0.5,y:-4.0},Point{x:82.0,y:40.0}),
                          (Point{x:1.0,y:31.0},Point{x:165.0,y:-37.0})] {
                let (a,b)=transformed(a,b,axes,false);
                index.insert_geometry(999,2,&a,&b); groups.push((a,b,vec![(999,2)]));
                let (q0,q1)=transformed(Point{x:0.0,y:8.0},Point{x:160.0,y:-8.0},axes,true);
                let expected=groups.iter().filter_map(|(s,e,occ)|swimlane_line_crossing(&q0,&q1,s,e)
                    .map(|(t,p)|occ.iter().map(move |&(edge,seg)|(t,p.clone(),edge,seg)))).flatten().collect();
                assert_eq!(signature(index.prior_crossings(&q0,&q1)),signature(expected));
            }
        }
    }
}

#[test]
fn each_endpoint_bound_is_independently_needed() {
    for query_short in [false,true] {
        for negate in [false,true] {
            for axes in [false,true] {
                for query_reverse in [false,true] {
                    let sign=if negate {-1.0}else{1.0};
                    let dx=if query_short {2.0}else{100.0};
                    let mut index=SwimlaneHopCrossings::default();
                    let inputs=if query_short {[(0.0,40.0,60.0),(-1.0,42.0,62.0)]}
                        else {[(0.0,2.0,4.0),(-1.0,3.0,5.0)]};
                    let (q0,q1)=transformed(Point{x:0.0,y:sign*10.0},Point{x:dx,y:sign*(10.0+dx)},axes,query_reverse);
                    for (i,(start,right,end)) in inputs.into_iter().enumerate() {
                        let (a,b)=transformed(Point{x:0.0,y:sign*start},Point{x:right,y:sign*end},axes,i==0);
                        assert!(swimlane_line_crossing(&q0,&q1,&a,&b).is_none());
                        index.insert_geometry(i,0,&a,&b);
                    }
                    SWIMLANE_LINE_CROSSING_VISITS.with(|n|n.set(0));
                    assert!(index.prior_crossings(&q0,&q1).is_empty());
                    let calls=SWIMLANE_LINE_CROSSING_VISITS.with(std::cell::Cell::get);
                    assert_eq!(calls,0,"query_short={query_short} negate={negate} axes={axes} qreverse={query_reverse}");
                }
            }
        }
    }
}

#[test]
fn original_epsilon_interval_is_pinned_by_integer_golden_queries() {
    for (cross_x,prior_dx,accepted) in [(1i64,500_000i64,true),(0,500_000,false),
                                       (999_998,2_000_000,true),(999_999,2_000_000,false)] {
        for axes in [false,true] {
            for prior_reverse in [false,true] {
                // Query orientation is fixed: reversing a lower endpoint case
                // intentionally changes the asymmetric inclusive interval.
                let(q0,q1)=transformed(Point{x:0.0,y:0.0},Point{x:1_000_000.0,y:0.0},axes,false);
                let(a,b)=transformed(Point{x:0.0,y:cross_x as f64},Point{x:prior_dx as f64,y:(cross_x-prior_dx) as f64},axes,prior_reverse);
                let golden=swimlane_line_crossing(&q0,&q1,&a,&b);
                assert_eq!(golden.is_some(),accepted,"cross_x={cross_x} prior_reverse={prior_reverse}");
                if let Some((t,_))=&golden {assert_eq!(t.to_bits(),(cross_x as f64/1_000_000.0).to_bits());}
                let mut index=SwimlaneHopCrossings::default();
                index.insert_geometry(8,3,&a,&b);
                let(f0,f1)=transformed(Point{x:0.0,y:-10.0},Point{x:600_003.0,y:-20.0},axes,false);
                index.insert_geometry(9,4,&f0,&f1);
                let expected=golden.map(|(t,p)|vec![(t,p,8,3)]).unwrap_or_default();
                assert_eq!(signature(index.prior_crossings(&q0,&q1)),signature(expected));
            }
        }
    }
}
