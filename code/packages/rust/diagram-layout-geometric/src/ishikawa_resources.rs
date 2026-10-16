//! Checked work and publication boundary for Ishikawa geometry.
//!
//! Admission has four phases: finite initial geometry, a precharged collision
//! schedule, final seeded outlines, then one globally bounded hatch pipeline.
//! Every mutable object stays local until all phases succeed.
use super::{Point,LayoutedIshikawaBone,LayoutedIshikawaDiagram,LayoutedIshikawaHatch};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum IshikawaLayoutError {DerivedGeometry,CollisionWork,HatchLines,HatchWork,HatchSegments,Allocation}
#[derive(Clone,Copy)]
pub(super) struct Limits {pub collision_pairs:usize,pub lines:usize,pub edge_visits:usize,pub segments:usize}
impl Default for Limits {
    fn default()->Self {Self{collision_pairs:8_388_608,lines:262_144,edge_visits:8_388_608,segments:262_144}}
}
type Error=IshikawaLayoutError;
fn finite(value:f64)->Result<f64,Error> {if value.is_finite(){Ok(value)}else{Err(Error::DerivedGeometry)}}
fn points_finite(points:&[Point])->Result<(),Error> {
    for p in points {finite(p.x)?;finite(p.y)?;}
    for edge in points.windows(2) {finite(edge[1].x-edge[0].x)?;finite(edge[1].y-edge[0].y)?;}
    Ok(())
}
fn extents(x:f64,y:f64,width:f64,height:f64)->Result<(f64,f64,f64,f64),Error> {
    for value in [x,y,width,height] {finite(value)?;}
    let right=finite(x+width)?;let bottom=finite(y+height)?;
    if width<=0.0 || height<=0.0 || x>=right || y>=bottom {return Err(Error::DerivedGeometry);}
    Ok((x,right,y,bottom))
}
fn bone_extents(b:&LayoutedIshikawaBone)->Result<(f64,f64,f64,f64),Error> {
    extents(finite(b.label_position.x-b.label_width/2.0)?,finite(b.label_position.y-b.label_height/2.0)?,b.label_width,b.label_height)?;
    // Preserve the actual centered operations consumed by the renderer and the
    // incoming collision algorithm; (x-half)+width can round differently.
    let left=finite(b.label_position.x-b.label_width/2.0)?;let right=finite(b.label_position.x+b.label_width/2.0)?;
    let top=finite(b.label_position.y-b.label_height/2.0)?;let bottom=finite(b.label_position.y+b.label_height/2.0)?;
    if left>=right || top>=bottom {return Err(Error::DerivedGeometry);}
    Ok((left,right,top,bottom))
}
fn side_clear(b:&LayoutedIshikawaBone,spine:f64,above:bool)->Result<(),Error> {
    let (_,_,top,bottom)=bone_extents(b)?;
    let gap=finite(if above {spine-bottom}else{top-spine})?;
    if gap<2.0 {return Err(Error::DerivedGeometry);} Ok(())
}
fn triangular(n:usize)->Option<usize> {
    if n<2 {Some(0)}else if n.is_multiple_of(2) {(n/2).checked_mul(n-1)}else{n.checked_mul((n-1)/2)}
}
fn pair_work(above:usize,below:usize,limit:usize)->Result<usize,Error> {
    let count=triangular(above).and_then(|a|triangular(below).and_then(|b|a.checked_add(b))).ok_or(Error::CollisionWork)?;
    if count>limit {Err(Error::CollisionWork)}else{Ok(count)}
}
fn reserve<T>(values:&mut Vec<T>,capacity:usize)->Result<(),Error> {
    if capacity<=values.capacity() {return Ok(());}
    #[cfg(test)] allocation_probe()?;
    values.try_reserve_exact(capacity-values.len()).map_err(|_|Error::Allocation)
}

pub(super) fn finish(layout:&mut LayoutedIshikawaDiagram,limits:Limits)->Result<(),Error> {
    for value in [layout.width,layout.height,layout.spine_from.x,layout.spine_from.y,layout.spine_to.x,layout.spine_to.y] {finite(value)?;}
    if layout.width<=0.0 || layout.height<=0.0 {return Err(Error::DerivedGeometry);}
    extents(layout.effect_x,layout.effect_y,layout.effect_width,layout.effect_height)?;
    points_finite(&layout.spine_path)?;
    let spine=layout.spine_from.y;
    let mut above=0;
    for b in &layout.bones {
        bone_extents(b)?;finite((b.label_position.y-spine).abs())?;
        points_finite(&b.path)?;points_finite(&b.arrowhead)?;
        if b.label_position.y<spine {above+=1;}
    }
    pair_work(above,layout.bones.len()-above,limits.collision_pairs)?;
    let mut order=Vec::new();reserve(&mut order,layout.bones.len())?;
    for (i,b) in layout.bones.iter().enumerate() {order.push((b.label_position.y<spine,(b.label_position.y-spine).abs(),i));}
    order.sort_unstable_by(|a,b|b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut first=0;
    for position in 0..order.len() {
        let (above,_,index)=order[position];
        if position>0 && order[position-1].0!=above {first=position;}
        let half=layout.bones[index].label_height/2.0;
        let clamp=finite(if above {finite(spine-half)?-2.0}else{finite(spine+half)?+2.0})?;
        let current=layout.bones[index].label_position.y;
        layout.bones[index].label_position.y=if above {current.min(clamp)}else{current.max(clamp)};
        side_clear(&layout.bones[index],spine,above)?;
        for &(_,_,other_index) in &order[first..position] {
            #[cfg(test)] observe(|s|s.collision_pairs+=1);
            let (left,right,_,_)=bone_extents(&layout.bones[index])?;
            let (other_left,other_right,other_top,other_bottom)=bone_extents(&layout.bones[other_index])?;
            let horizontal_clear=finite(other_left-right)? >=4.0 || finite(left-other_right)? >=4.0;
            let horizontal_overlap=finite(right+4.0)?>other_left && finite(left-4.0)?<other_right;
            if !horizontal_overlap {if !horizontal_clear {return Err(Error::DerivedGeometry);}continue;}
            let target=layout.bones[index].label_position.y;
            let shifted=if above && finite(finite(target+half)?+4.0)?>other_top {
                target.min(finite(finite(other_top-half)?-4.0)?)
            }else if !above && finite(finite(target-half)?-4.0)?<other_bottom {
                target.max(finite(finite(other_bottom+half)?+4.0)?)
            }else{target};
            layout.bones[index].label_position.y=shifted;
            side_clear(&layout.bones[index],spine,above)?;
            let (_,_,top,bottom)=bone_extents(&layout.bones[index])?;
            let vertical_gap=finite(if above {other_top-bottom}else{top-other_bottom})?;
            if !horizontal_clear && vertical_gap<4.0 {return Err(Error::DerivedGeometry);}
        }
    }
    // Nothing was clipped before the last collision target was certified.
    let hand=layout.config.look==diagram_ir::IshikawaLook::HandDrawn;let seed=layout.config.hand_drawn_seed;
    layout.effect_outline=super::ishikawa_effect_outline(layout.effect_x,layout.effect_y,layout.effect_width,layout.effect_height,hand,seed,1_000);
    #[cfg(test)] observe(|s|s.outline_builds+=1);
    points_finite(&layout.effect_outline)?;
    for (i,b) in layout.bones.iter_mut().enumerate() {
        b.label_outline=super::ishikawa_box_outline(b.label_position.x-b.label_width/2.0,b.label_position.y-b.label_height/2.0,b.label_width,b.label_height,hand,seed,2_000+i);
        #[cfg(test)] observe(|s|s.outline_builds+=1);
        points_finite(&b.label_outline)?;
    }
    if hand {
        let mut outlines=Vec::new();reserve(&mut outlines,layout.bones.len()+1)?;
        outlines.push(layout.effect_outline.as_slice());for b in &layout.bones {outlines.push(b.label_outline.as_slice());}
        let result=finish_hatches(&outlines,limits)?;drop(outlines);
        let mut result=result.into_iter();layout.effect_hatches=result.next().expect("effect outline admitted");
        for b in &mut layout.bones {b.hatches=result.next().expect("label outline admitted");}
    }
    Ok(())
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct Grid {first:i64,lines:usize}
fn grid(minimum:f64,maximum:f64)->Result<Grid,Error> {
    finite(minimum)?;finite(maximum)?;if minimum>maximum {return Err(Error::DerivedGeometry);}
    let first=finite((minimum/5.0).floor())?;let last=finite((maximum/5.0).floor())?;
    let limit=(1_u64<<50) as f64;
    // These stricter generated-offset bounds imply exact safe i64 casts, integer
    // subtraction and exact 5*index arithmetic. An unused next index is irrelevant.
    if first*5.0 < -limit || last*5.0>limit {return Err(Error::DerivedGeometry);}
    let first=first as i64;let last=last as i64;
    let lines=last.checked_sub(first).and_then(|n|n.checked_add(1)).and_then(|n|usize::try_from(n).ok()).ok_or(Error::DerivedGeometry)?;
    Ok(Grid{first,lines})
}
fn direction()->(f64,f64) {let angle=-41.0_f64.to_radians();(angle.cos(),angle.sin())}
fn projection(p:&Point)->f64 {let d=direction();p.x*(-d.1)+p.y*d.0}
fn outline_grid(outline:&[Point])->Result<Grid,Error> {
    if outline.len()>53 {return Err(Error::DerivedGeometry);}
    points_finite(outline)?;
    if outline.is_empty() {return Ok(Grid{first:0,lines:0});}
    let mut minimum=f64::INFINITY;let mut maximum=f64::NEG_INFINITY;
    for p in outline {let value=finite(projection(p))?;minimum=minimum.min(value);maximum=maximum.max(value);}
    grid(minimum,maximum)
}
struct Intersection {key:f64,edge:usize,point:Point}
fn intersections(outline:&[Point],offset:f64,scratch:&mut Vec<Intersection>)->Result<(),Error> {
    scratch.clear();let d=direction();
    #[cfg(test)] observe(|s|s.clip_lines+=1);
    for (edge_index,edge) in outline.windows(2).enumerate() {
        #[cfg(test)] observe(|s|s.edge_visits+=1);
        let from=finite(projection(&edge[0])-offset)?;let to=finite(projection(&edge[1])-offset)?;
        let denominator=finite(from-to)?;
        if from*to<=0.0 && denominator.abs()>1e-9 {
            let ratio=finite(from/denominator)?;
            if !(0.0..=1.0).contains(&ratio) {return Err(Error::DerivedGeometry);}
            let point=Point{x:finite(edge[0].x+(edge[1].x-edge[0].x)*ratio)?,y:finite(edge[0].y+(edge[1].y-edge[0].y)*ratio)?};
            let key=finite(point.x*d.0+point.y*d.1)?;
            if scratch.len()==scratch.capacity() {return Err(Error::Allocation);}
            scratch.push(Intersection{key,edge:edge_index,point});
        }
    }
    // Edge index makes equal-key ordering identical to the old stable sort,
    // without hidden stable-sort scratch allocation on every logical line.
    scratch.sort_unstable_by(|a,b|a.key.total_cmp(&b.key).then(a.edge.cmp(&b.edge)));
    scratch.dedup_by(|a,b|(a.key-b.key).abs()<1e-7);Ok(())
}
fn finish_hatches(outlines:&[&[Point]],limits:Limits)->Result<Vec<Vec<LayoutedIshikawaHatch>>,Error> {
    // Numeric validity of *all* outlines wins over a budget error on an earlier
    // outline. This first pass allocates neither plans nor hatch storage.
    let mut lines=Some(0_usize);let mut work=Some(0_usize);let mut max_edges=0;
    for outline in outlines {
        let plan=outline_grid(outline)?;let edges=outline.len().saturating_sub(1);max_edges=max_edges.max(edges);
        lines=lines.and_then(|sum|sum.checked_add(plan.lines));
        work=work.and_then(|sum|plan.lines.checked_mul(edges).and_then(|n|n.checked_mul(2)).and_then(|n|sum.checked_add(n)));
    }
    if lines.is_none_or(|n|n>limits.lines) {return Err(Error::HatchLines);}
    if work.is_none_or(|n|n>limits.edge_visits) {return Err(Error::HatchWork);}
    let mut plans=Vec::new();reserve(&mut plans,outlines.len())?;
    let mut scratch=Vec::new();reserve(&mut scratch,max_edges)?;
    let mut total=0_usize;
    for outline in outlines {
        let plan=outline_grid(outline)?;let mut count=0_usize;
        #[cfg(test)] observe(|s|s.hatch_passes+=1);
        for step in 0..plan.lines {
            let index=plan.first.checked_add(i64::try_from(step).map_err(|_|Error::DerivedGeometry)?).ok_or(Error::DerivedGeometry)?;
            intersections(outline,index as f64*5.0,&mut scratch)?;
            count=count.checked_add(scratch.len()/2).ok_or(Error::HatchSegments)?;
        }
        total=total.checked_add(count).ok_or(Error::HatchSegments)?;plans.push((plan,count));
    }
    if total>limits.segments {return Err(Error::HatchSegments);}
    let mut result=Vec::new();reserve(&mut result,outlines.len())?;
    for (_,count) in &plans {let mut values=Vec::new();reserve(&mut values,*count)?;result.push(values);}
    for ((outline,(plan,count)),values) in outlines.iter().zip(&plans).zip(&mut result) {
        #[cfg(test)] observe(|s|s.hatch_passes+=1);
        for step in 0..plan.lines {
            let index=plan.first.checked_add(i64::try_from(step).map_err(|_|Error::DerivedGeometry)?).ok_or(Error::DerivedGeometry)?;
            intersections(outline,index as f64*5.0,&mut scratch)?;
            for pair in scratch.chunks_exact(2) {
                if values.len()>=*count {return Err(Error::DerivedGeometry);}
                values.push(LayoutedIshikawaHatch{from:pair[0].point.clone(),to:pair[1].point.clone()});
            }
        }
        if values.len()!=*count {return Err(Error::DerivedGeometry);}
    }
    Ok(result)
}

#[cfg(test)]
#[derive(Clone,Copy,Default)]
struct Stats {collision_pairs:usize,outline_builds:usize,hatch_passes:usize,clip_lines:usize,edge_visits:usize}
#[cfg(test)]
std::thread_local! {
    static STATS:std::cell::Cell<Stats>=const{std::cell::Cell::new(Stats{collision_pairs:0,outline_builds:0,hatch_passes:0,clip_lines:0,edge_visits:0})};
    static ALLOCATION:std::cell::Cell<Option<usize>>=const{std::cell::Cell::new(None)};
}
#[cfg(test)] fn observe(f:impl FnOnce(&mut Stats)) {STATS.with(|s|{let mut value=s.get();f(&mut value);s.set(value);});}
#[cfg(test)] fn reset_stats() {STATS.with(|s|s.set(Stats::default()));}
#[cfg(test)] fn stats()->Stats {STATS.with(|s|s.get())}
#[cfg(test)] fn allocation_probe()->Result<(),Error> {ALLOCATION.with(|s|match s.get() {Some(0)=>Err(Error::Allocation),Some(n)=>{s.set(Some(n-1));Ok(())},None=>Ok(())})}
#[cfg(test)] struct AllocationFailure(std::marker::PhantomData<std::rc::Rc<()>>);
#[cfg(test)] impl AllocationFailure {fn after(count:usize)->Self {ALLOCATION.with(|s|{assert!(s.get().is_none());s.set(Some(count));});Self(std::marker::PhantomData)}}
#[cfg(test)] impl Drop for AllocationFailure {fn drop(&mut self){ALLOCATION.with(|s|s.set(None));}}
#[cfg(test)] #[path="ishikawa_resources_tests.rs"] mod tests;
