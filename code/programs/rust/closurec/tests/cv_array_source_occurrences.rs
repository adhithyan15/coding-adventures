//! CLOC33 starts at a real folded result and verifies its own array source node.
//! Original bytes, shared coordinates and ordered physical parents are exact;
//! equal source text in another occurrence/input is not interchangeable.
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const SOURCE: &str = "closurec_source_v1";
const NODE: &str = "closurec_node_v1";

fn compile(sources: &[&str], level: &str) -> (Value, String, Vec<String>) {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!("closurec-cloc33-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir(&dir).unwrap();
    let paths: Vec<_> = sources.iter().enumerate().map(|(i, source)| {
        let path=dir.join(format!("input-{i}.js"));std::fs::write(&path,source).unwrap();path
    }).collect();
    let mut outputs=Vec::new();let mut sidecar=None;
    for traced in [true,false] {
        let out=dir.join(if traced {"traced.js"} else {"plain.js"});
        let mut command=Command::new(BINARY);
        command.args(["--compilation_level",level,"--js_output_file"]).arg(&out);
        for path in &paths {command.arg("--js").arg(path);}
        if traced {command.arg("--correlation_vector");}
        let result=command.output().unwrap();
        assert!(result.status.success(),"{level}: {}",String::from_utf8_lossy(&result.stderr));
        outputs.push(std::fs::read_to_string(&out).unwrap());
        if traced {sidecar=Some(serde_json::from_str(&std::fs::read_to_string(format!("{}.cv.json",out.display())).unwrap()).unwrap());}
    }
    assert_eq!(outputs[0],outputs[1],"array tracing changed output bytes");
    let names=paths.iter().map(|p|p.to_string_lossy().into_owned()).collect();
    std::fs::remove_dir_all(&dir).unwrap();
    (sidecar.unwrap(),outputs.remove(0),names)
}

/// Independent expected coordinates from raw UTF8; CRLF is one line terminator.
fn position(source:&str, byte:usize)->Value {
    let (mut scalar,mut utf16,mut line,mut column,mut utf16_column)=(0,0,0,0,0);
    let mut previous_cr=false;
    for ch in source[..byte].chars() {
        scalar+=1;utf16+=ch.len_utf16();
        if ch=='\n' && previous_cr {} else if matches!(ch,'\r'|'\n'|'\u{2028}'|'\u{2029}') {
            line+=1;column=0;utf16_column=0;
        } else {column+=1;utf16_column+=ch.len_utf16();}
        previous_cr=ch=='\r';
    }
    json!({"byte_offset":byte.to_string(),"scalar_offset":scalar.to_string(),"utf16_offset":utf16.to_string(),"line":line.to_string(),"scalar_column":column.to_string(),"utf16_column":utf16_column.to_string()})
}
fn ancestry(j:&Value,id:&str)->BTreeSet<String> {
    fn visit(j:&Value,id:&str,seen:&mut BTreeSet<String>) {
        if !seen.insert(id.to_owned()) {return;}
        for p in j["entries"][id]["parent_ids"].as_array().unwrap() {visit(j,p.as_str().unwrap(),seen);}
    }
    let mut seen=BTreeSet::new();visit(j,id,&mut seen);seen
}

fn assert_array(j:&Value,source:&str,path:&str,start:usize,end:usize,fold:Option<&str>)->String {
    let entries=j["entries"].as_object().unwrap();
    let roots:Vec<_>=entries.iter().filter(|(_,e)|e["origin"]["source"]=="input_file" && e["origin"]["location"]==path).collect();
    assert_eq!(roots.len(),1);let (root,root_entry)=roots[0];
    assert_eq!(root_entry["origin"]["meta"][SOURCE]["byte_length"],source.len().to_string());
    let candidates:Vec<_>=entries.iter().filter(|(_,e)| {
        let m=&e["origin"]["meta"][NODE];e["origin"]["source"]=="typed_array" && m["input_id"]==root.as_str() && m["start"]["byte_offset"]==start.to_string() && m["end"]["byte_offset"]==end.to_string()
    }).collect();
    assert_eq!(candidates.len(),1,"actual array source occurrence missing for {start}..{end}: {}",&source[start..end]);
    let (id,entry)=candidates[0];let meta=&entry["origin"]["meta"][NODE];
    assert_eq!(entry["origin"]["meta"].as_object().unwrap().len(),1);
    assert_eq!(entry["origin"]["location"],"array_literal");assert!(entry["origin"]["timestamp"].is_null());
    assert_eq!(meta["schema"],"1");assert_eq!(meta["kind"],"ArrayExpression");assert_eq!(meta.as_object().unwrap().len(),8);
    assert_eq!(meta["start"],position(source,start));assert_eq!(meta["end"],position(source,end));
    let inventories:Vec<_>=root_entry["contributions"].as_array().unwrap().iter().filter(|c|c["source"]=="source.inventory" && c["tag"]=="finalized").collect();
    assert_eq!(inventories.len(),1);let pieces=inventories[0]["meta"][SOURCE]["piece_ids"].as_array().unwrap();
    let first=pieces.iter().position(|p|p==&meta["first_piece_id"]).unwrap();
    let last=pieces.iter().position(|p|p==&meta["last_piece_id"]).unwrap();assert!(first<last);
    assert_eq!(entry["parent_ids"],Value::Array(pieces[first..=last].to_vec()),"complete source-order parent interval");
    assert!(!pieces.iter().any(|p|p==id.as_str()),"array must own a fresh composite ID");
    let mut cursor=start;
    for p in &pieces[first..=last] {
        let physical=&entries[p.as_str().unwrap()];let m=&physical["origin"]["meta"][SOURCE];
        assert_eq!(physical["parent_ids"],json!([root]));assert_eq!(m["input_id"],root.as_str());
        let next=m["end"]["byte_offset"].as_str().unwrap().parse::<usize>().unwrap();
        assert_eq!(m["start"],position(source,cursor));assert_eq!(m["end"],position(source,next));assert!(next>cursor);cursor=next;
    }
    assert_eq!(cursor,end);
    assert_eq!(entries[meta["first_piece_id"].as_str().unwrap()]["origin"]["meta"][SOURCE]["token_type"],"LBracket");
    assert_eq!(entries[meta["last_piece_id"].as_str().unwrap()]["origin"]["meta"][SOURCE]["token_type"],"RBracket");
    let events=j["journal"]["events"].as_array().unwrap();
    let merge:Vec<_>=events.iter().filter(|e|e["event"]["kind"]=="merge" && e["event"]["entity"]==id.as_str()).collect();
    assert_eq!(merge.len(),1);assert!(merge[0]["context"].is_null());
    if let Some(after)=fold {
        let folds:Vec<_>=entries.iter().filter(|(_,e)|e["parent_ids"].as_array().unwrap().iter().any(|p|p==id.as_str()) && e["contributions"].as_array().unwrap().iter().any(|c|c["source"]=="constant-fold" && c["tag"]=="folded" && c["meta"]["after"]==after)).collect();
        assert_eq!(folds.len(),1,"result-owned array length lineage");let (result_id,result)=folds[0];
        assert!(result["contributions"].as_array().unwrap().iter().any(|c|c["meta"]["new_cv"]==result_id.as_str()));
        let reachable=ancestry(j,result_id);assert!(reachable.contains(id));
        for p in &pieces[first..=last] {assert!(reachable.contains(p.as_str().unwrap()));}
        for p in pieces.iter().take(first).chain(pieces.iter().skip(last+1)) {
            // The computed property key is a legitimate separate operand.
            let physical=&entries[p.as_str().unwrap()];
            if physical["origin"]["meta"][SOURCE]["token_type"]!="String" {assert!(!reachable.contains(p.as_str().unwrap()),"unrelated physical sibling reached");}
        }
        let recorded=events.iter().find(|e|e["event"]["kind"]=="merge" && e["event"]["entity"]==result_id.as_str()).unwrap();
        assert!(merge[0]["sequence"].as_str().unwrap()<recorded["sequence"].as_str().unwrap(),"array construction precedes its fold");
    }
    id.clone()
}

#[test]
fn empty_and_hole_array_length_results_reach_exact_array_occurrences() {
    let source="report([].length);report([,,][\"length\"]);\n";
    for level in ["SIMPLE","ADVANCED"] {
        let (j,out,paths)=compile(&[source],level);assert_eq!(out,"report(0);report(2);\n");
        let a=assert_array(&j,source,&paths[0],7,9,Some("0"));let first=source.find("[,,]").unwrap();
        let b=assert_array(&j,source,&paths[0],first,first+4,Some("2"));assert_ne!(a,b);
    }
}
#[test]
fn nested_and_repeated_arrays_preserve_raw_ranges_and_distinct_occurrences() {
    let source="/*😀*/\r\nreport([/*x*/\"\\u0061\",,[]].length)\nreport([[],[]].length)\n";
    for level in ["SIMPLE","ADVANCED"] {
        let (j,_,paths)=compile(&[source],level);
        let outer=source.find("[/*x*/").unwrap();let inner=source.find("[]").unwrap();
        let a=assert_array(&j,source,&paths[0],outer,inner+3,Some("3"));
        let b=assert_array(&j,source,&paths[0],inner,inner+2,None);assert_ne!(a,b);
        let second=source.find("[[],[]]").unwrap();
        let c=assert_array(&j,source,&paths[0],second,second+7,Some("2"));
        let d=assert_array(&j,source,&paths[0],second+1,second+3,None);
        let e=assert_array(&j,source,&paths[0],second+4,second+6,None);
        assert_eq!(BTreeSet::from([a,b,c,d,e]).len(),5);
    }
}
#[test]
fn identical_inputs_do_not_share_array_identity_or_source_parents() {
    let source="report([].length)\n";
    for level in ["SIMPLE","ADVANCED"] {
        let (j,out,paths)=compile(&[source,source],level);assert_eq!(out,"report(0);\nreport(0);\n");
        let a=assert_array(&j,source,&paths[0],7,9,Some("0"));let b=assert_array(&j,source,&paths[1],7,9,Some("0"));
        assert_ne!(a,b);let left=ancestry(&j,&a);let right=ancestry(&j,&b);assert!(left.is_disjoint(&right));
    }
}
