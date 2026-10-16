//! D1 producer evidence binds full raw-source descriptors, exact ordered piece
//! identities and actual parent vectors. Generic graph import is not source-domain
//! admission; actual typed-parser ID reuse and output-byte lineage stay open.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    dir: PathBuf,
    input: PathBuf,
    paths: [PathBuf; 4],
}
impl Fixture {
    fn new(source: &str) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "closurec-cv04b-d1-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let input = dir.join("a.js");
        std::fs::write(&input, source).unwrap();
        let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|name| dir.join(name));
        Self { dir, input, paths }
    }
    fn run(&self, level: &str, traced: bool, options: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_closurec"));
        command
            .arg("--js")
            .arg(&self.input)
            .args(["--compilation_level", level])
            .arg("--js_output_file")
            .arg(&self.paths[0])
            .arg("--create_source_map")
            .arg(&self.paths[1])
            .arg("--output_manifest")
            .arg(&self.paths[2]);
        if traced {
            command
                .arg("--correlation_vector")
                .arg("--correlation_vector_output")
                .arg(&self.paths[3]);
        }
        command
            .args(options)
            .env_remove("PSModulePath")
            .current_dir(&self.dir)
            .output()
            .unwrap()
    }
    fn sidecar(&self) -> String {
        std::fs::read_to_string(&self.paths[3]).unwrap()
    }
    fn outputs(&self) -> Vec<Vec<u8>> {
        self.paths[..3]
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.dir.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}
fn success(output: Output) {
    assert!(
        output.status.code() == Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
}
fn reload(text: &str) -> Value {
    let wire: Value = serde_json::from_str(text).unwrap();
    assert!(
        wire.get("journal").is_some(),
        "actual CLI snapshot has no chronology declaration"
    );
    let checked = CVLog::from_checked_json(text, GraphLimits::default()).unwrap();
    let compatibility = CVLog::from_json_string(text).unwrap();
    assert!(
        checked.to_json_string().unwrap() == compatibility.to_json_string().unwrap(),
        "checked and compatibility chronology imports disagree"
    );
    wire
}

use serde_json::json;
use std::collections::BTreeSet;
const NS: &str = "closurec_source_v1";
const LEVELS: [&str; 5] = ["BUNDLE", "TRANSPILE_ONLY", "WHITESPACE_ONLY", "SIMPLE", "ADVANCED"];
// Positions are independently authored (byte, scalar, UTF16, line, scalarcol, UTF16col).
type Pos = [usize; 6];
#[derive(Clone, Copy)]
struct Piece { type_: &'static str, token: Option<usize>, start: Pos, end: Pos }
fn position(p: Pos) -> Value {
    json!({"byte_offset":p[0].to_string(),"scalar_offset":p[1].to_string(),"utf16_offset":p[2].to_string(),"line":p[3].to_string(),"scalar_column":p[4].to_string(),"utf16_column":p[5].to_string()})
}
fn linear(n:usize)->Pos {[n,n,n,0,n,n]}
fn token(type_: &'static str,index:usize,start:usize,end:usize)->Piece {Piece{type_,token:Some(index),start:linear(start),end:linear(end)}}
fn trivia(start:usize,end:usize)->Piece {Piece{type_:"trivia",token:None,start:linear(start),end:linear(end)}}
fn ascii()->Vec<Piece> {vec![token("Number",0,0,1),token("Plus",1,1,2),token("Number",2,2,3),token("Semicolon",3,3,4),token("EOF",4,4,4)]}
fn input_entry<'a>(wire:&'a Value,path:&std::path::Path)->(&'a str,&'a Value) {
    let rows:Vec<_>=wire["entries"].as_object().unwrap().iter().filter(|(_,e)|e["origin"]["source"]=="input_file"&&e["origin"]["location"]==path.to_string_lossy().as_ref()).collect();
    assert_eq!(rows.len(),1,"one exact file root");(rows[0].0.as_str(),rows[0].1)
}
fn assert_inventory(wire:&Value,path:&std::path::Path,source:&str,sha:&str,rows:&[Piece])->Vec<String> {
    assert_inventory_editions(wire,path,source,sha,rows,coding_adventures_javascript_tokens::EsVersion::ALL)
}
fn assert_inventory_editions(wire:&Value,path:&std::path::Path,source:&str,sha:&str,rows:&[Piece],editions:&[coding_adventures_javascript_tokens::EsVersion])->Vec<String> {
    // Validate the independently authored fixture before asking the new producer
    // to match it. This exercises the existing captured/legacy API at every
    // edition and cannot turn a bad expected partition into a producer defect.
    let point=|p:Pos|lexer::lexical::SourcePosition{byte_offset:p[0],scalar_offset:p[1],utf16_offset:p[2],line:p[3],scalar_column:p[4],utf16_column:p[5]};
    let declared:Vec<_>=rows.iter().map(|r|lexer::lexical::LexicalPiece{kind:match r.token{None=>lexer::lexical::LexicalPieceKind::Trivia,Some(i) if r.type_=="EOF"=>lexer::lexical::LexicalPieceKind::Eof(i),Some(i)=>lexer::lexical::LexicalPieceKind::Token(i)},range:lexer::lexical::SourceRange{start:point(r.start),end:point(r.end)}}).collect();
    for &edition in editions {
        let limits=lexer::lexical::LexicalLimits::default();let captured=coding_adventures_javascript_lexer::tokenize_javascript_typed_with_ranges(source,edition,limits).unwrap();let legacy=coding_adventures_javascript_lexer::tokenize_javascript_typed(source,edition).unwrap();
        assert_eq!(captured.tokens(),legacy.as_slice(),"unchanged complete Tokens/ASI/CVs {edition}");
        assert_eq!(captured.pieces(),declared.as_slice(),"independent raw piece fixture {edition}");
        lexer::lexical::validate_lexical_partition(source,legacy.len(),&declared,limits).unwrap();
    }
    let (root_id,root)=input_entry(wire,path);
    assert_eq!(root["parent_ids"],json!([]));
    assert_eq!(root["origin"]["timestamp"],Value::Null);
    assert_eq!(root["origin"]["meta"],json!({"byte_len":source.len(),"content_sha256":sha,NS:{"schema":"1","role":"input","buffer_domain":"read_to_string_utf8","encoding":"utf8","grammar":"javascript_standard","edition":"es2025","sha256":sha,"byte_length":source.len().to_string()}}),"complete input descriptor and legacy agreement");
    let inventories:Vec<_>=root["contributions"].as_array().unwrap().iter().filter(|c|c["source"]=="source.inventory").collect();
    assert_eq!(inventories.len(),1,"one finalized root inventory");assert_eq!(inventories[0]["tag"],"finalized");
    let ids:Vec<String>=inventories[0]["meta"][NS]["piece_ids"].as_array().expect("ordered inventory IDs").iter().map(|v|v.as_str().unwrap().to_string()).collect();
    let tokens=coding_adventures_javascript_lexer::tokenize_javascript_typed(source,coding_adventures_javascript_tokens::EsVersion::Es2025).unwrap();
    assert_eq!(inventories[0]["meta"],json!({NS:{"schema":"1","role":"inventory","status":"available","piece_count":rows.len().to_string(),"token_count":tokens.len().to_string(),"piece_ids":ids}}),"complete finalized inventory");
    assert_eq!(ids.len(),rows.len());assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(),ids.len());
    for (index,(id,row)) in ids.iter().zip(rows).enumerate() {
        assert_eq!(id.len(),20);assert_eq!(&id[..4],"cv1.");assert!(id.as_bytes()[4..].iter().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(b)));
        assert_ne!(id,root_id);let piece=&wire["entries"][id];assert_eq!(piece["id"],id.as_str());assert_eq!(piece["parent_ids"],json!([root_id]),"direct exact file-root ancestry");
        let mut reserved=json!({"schema":"1","role":"piece","input_id":root_id,"piece_index":index.to_string(),"kind":if row.type_=="trivia"{"trivia"}else if row.type_=="EOF"{"eof"}else{"token"},"start":position(row.start),"end":position(row.end)});
        let expected_origin=if let Some(ordinal)=row.token {
            reserved["token_index"]=json!(ordinal.to_string());reserved["token_type"]=json!(row.type_);
            let old=&tokens[ordinal];json!({"source":"lexer_token","location":format!("{}:{}:{}",path.to_string_lossy(),old.line,old.column),"timestamp":null,"meta":{"kind":row.type_.to_ascii_lowercase(),"lexeme_byte_len":old.value.len(),"token_index":ordinal,NS:reserved}})
        } else {
            json!({"source":"lexer_trivia","location":format!("{}:{}:{}",path.to_string_lossy(),row.start[3]+1,row.start[4]+1),"timestamp":null,"meta":{NS:reserved}})
        };
        assert_eq!(piece["origin"],expected_origin,"complete piece origin #{index}");
    }
    let declared:BTreeSet<_>=ids.iter().map(String::as_str).collect();
    let actual:BTreeSet<_>=wire["entries"].as_object().unwrap().iter().filter(|(_,e)|e["origin"]["meta"][NS]["role"]=="piece"&&e["origin"]["meta"][NS]["input_id"]==root_id).map(|(id,_)|id.as_str()).collect();assert_eq!(actual,declared,"no omitted or extra namespace child");
    ids
}
#[test]
fn d1_repeated_tokens_have_exact_full_inventories_at_all_five_levels() {
    for level in LEVELS {
        let f=Fixture::new("1+1;");success(f.run(level,false,&[]));let plain=f.outputs();success(f.run(level,true,&[]));assert_eq!(f.outputs(),plain);
        let first=f.sidecar();let wire=reload(&first);let ids=assert_inventory(&wire,&f.input,"1+1;","df43fbefcd7864653c637255ab18bb3fe86fbfc887d3dd15f33d61ad67d4709f",&ascii());assert_ne!(ids[0],ids[2]);
        success(f.run(level,true,&[]));assert_eq!(f.outputs(),plain);assert_eq!(f.sidecar(),first,"whole graph/journal deterministic");
        eprintln!("CV04b-D1-CLI-COMPLETE case=repeated level={level} exactInventory");
    }
}
#[test]
fn d1_empty_comment_and_trailing_trivia_keep_exact_terminal_eof() {
    for (name,source,sha,rows) in [
        ("empty","","e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",vec![token("EOF",0,0,0)]),
        ("comment","/*x*/","d1281f437bf6e2652171445fce45d73a6d43c99410883700bbe3f25e01b5547a",vec![trivia(0,5),token("EOF",0,5,5)]),
        ("trailing","1;/*x*/","17d28f0e68a7a817b7c22e2538c64b4bad1b7505bd4c695bd6925933eea14b3f",vec![token("Number",0,0,1),token("Semicolon",1,1,2),trivia(2,7),token("EOF",2,7,7)]),
    ] {
        let f=Fixture::new(source);success(f.run("WHITESPACE_ONLY",false,&[]));let plain=f.outputs();success(f.run("WHITESPACE_ONLY",true,&[]));assert_eq!(f.outputs(),plain);assert_inventory(&reload(&f.sidecar()),&f.input,source,sha,&rows);
        eprintln!("CV04b-D1-CLI-COMPLETE case={name} level=WHITESPACE_ONLY exactInventory");
    }
}
#[test]
fn d1_escaped_astral_string_binds_raw_ranges_instead_of_processed_length() {
    let source="\"😀\\n\";";let rows=[
        Piece{type_:"String",token:Some(0),start:[0,0,0,0,0,0],end:[8,5,6,0,5,6]},
        Piece{type_:"Semicolon",token:Some(1),start:[8,5,6,0,5,6],end:[9,6,7,0,6,7]},
        Piece{type_:"EOF",token:Some(2),start:[9,6,7,0,6,7],end:[9,6,7,0,6,7]},
    ];
    let f=Fixture::new(source);success(f.run("WHITESPACE_ONLY",true,&[]));let wire=reload(&f.sidecar());let ids=assert_inventory(&wire,&f.input,source,"81a656f58118529b94001ce1c21014de058dc6c9582f8efba1fc8043c25acbc1",&rows);
    assert_eq!(wire["entries"][&ids[0]]["origin"]["meta"]["lexeme_byte_len"],5);assert_eq!(rows[0].end[0]-rows[0].start[0],8);
    eprintln!("CV04b-D1-CLI-COMPLETE case=astralEscape level=WHITESPACE_ONLY exactInventory");
}
#[test]
fn d1_crlf_bare_cr_and_matched_unicode_separators_keep_independent_coordinates() {
    for (name,source,sha,skip_end,next_start,last) in [
        ("crlf","1;\r\n2;","073481381512116d63781a55b449beae44ffaec98326df39786e5db352321ffd",[4,4,4,1,0,0],[4,4,4,1,0,0],[6,6,6,1,2,2]),
        ("cr","1;\r2;","e376b99c80b7edccd948a6d7522e3dc93ef245d63d1ece04772c0878de19e487",[3,3,3,1,0,0],[3,3,3,1,0,0],[5,5,5,1,2,2]),
    ] {
        let middle=[next_start[0]+1,next_start[1]+1,next_start[2]+1,1,1,1];let rows=[token("Number",0,0,1),token("Semicolon",1,1,2),Piece{type_:"trivia",token:None,start:linear(2),end:skip_end},Piece{type_:"Number",token:Some(2),start:next_start,end:middle},Piece{type_:"Semicolon",token:Some(3),start:middle,end:last},Piece{type_:"EOF",token:Some(4),start:last,end:last}];
        let f=Fixture::new(source);success(f.run("WHITESPACE_ONLY",true,&[]));assert_inventory(&reload(&f.sidecar()),&f.input,source,sha,&rows);eprintln!("CV04b-D1-CLI-COMPLETE case={name} level=WHITESPACE_ONLY exactInventory");
    }
    let source="/*\u{2028}\u{2029}*/1;";let end=[10,6,6,2,2,2];let last=[12,8,8,2,4,4];let rows=[Piece{type_:"trivia",token:None,start:linear(0),end},Piece{type_:"Number",token:Some(0),start:end,end:[11,7,7,2,3,3]},Piece{type_:"Semicolon",token:Some(1),start:[11,7,7,2,3,3],end:last},Piece{type_:"EOF",token:Some(2),start:last,end:last}];
    let f=Fixture::new(source);success(f.run("WHITESPACE_ONLY",true,&[]));assert_inventory(&reload(&f.sidecar()),&f.input,source,"ea00a42c6c78fbf32d484484bdadab44479d9e1af8984aabd1f6737c37b7f2a5",&rows);eprintln!("CV04b-D1-CLI-COMPLETE case=unicodeSeparators level=WHITESPACE_ONLY exactInventory");
}
#[test]
fn d1_ordinary_lexical_failure_has_exact_unavailable_inventory_in_copy_modes() {
    assert!(coding_adventures_javascript_lexer::tokenize_javascript_typed("var x=\0",coding_adventures_javascript_tokens::EsVersion::Es2025).is_err(),"the unavailable control is an actual ordinary lexical error");
    for level in ["BUNDLE","TRANSPILE_ONLY"] {
        let f=Fixture::new("var x=\0");success(f.run(level,false,&[]));let plain=f.outputs();success(f.run(level,true,&[]));assert_eq!(f.outputs(),plain);let wire=reload(&f.sidecar());let (id,root)=input_entry(&wire,&f.input);
        assert_eq!(root["origin"]["meta"][NS],json!({"schema":"1","role":"input","buffer_domain":"read_to_string_utf8","encoding":"utf8","grammar":"javascript_standard","edition":"es2025","sha256":"db347a502947b0c40419db751366a87b1d026bec2fc259de66193382c73e900f","byte_length":"7"}));
        let rows:Vec<_>=root["contributions"].as_array().unwrap().iter().filter(|c|c["source"]=="source.inventory").collect();assert_eq!(rows.len(),1);assert_eq!(rows[0]["tag"],"finalized");assert_eq!(rows[0]["meta"],json!({NS:{"schema":"1","role":"inventory","status":"unavailable","piece_count":"0","token_count":"0","piece_ids":[],"reason":"lexical_error"}}));
        assert!(!wire["entries"].as_object().unwrap().values().any(|e|e["origin"]["meta"][NS]["role"]=="piece"&&e["origin"]["meta"][NS]["input_id"]==id));
        assert!(root["contributions"].as_array().unwrap().iter().any(|c|c["source"]=="lex"&&c["tag"]=="failed"));
        eprintln!("CV04b-D1-CLI-COMPLETE case=lexicalError level={level} exactInventory");
    }
}
#[test]
fn d1_defines_attach_to_the_exact_original_ingestion_token_identity() {
    let source="FLAG+1;";let f=Fixture::new(source);success(f.run("SIMPLE",false,&["--define","FLAG=3"]));let plain=f.outputs();success(f.run("SIMPLE",true,&["--define","FLAG=3"]));assert_eq!(f.outputs(),plain);let wire=reload(&f.sidecar());
    let rows=[token("Name",0,0,4),token("Plus",1,4,5),token("Number",2,5,6),token("Semicolon",3,6,7),token("EOF",4,7,7)];let ids=assert_inventory(&wire,&f.input,source,"d2deb447843a06c64f5bf82c0c7b622d2b679f4a63d96ebbef9bc212acbc264d",&rows);
    let events:Vec<_>=wire["entries"][&ids[0]]["contributions"].as_array().unwrap().iter().filter(|c|c["source"]=="defines"&&c["tag"]=="applied").collect();assert_eq!(events.len(),1);assert_eq!(events[0]["meta"],json!({"define_name":"FLAG","define_value":3.0,"define_value_kind":"number","token_index":0}));
    eprintln!("CV04b-D1-CLI-COMPLETE case=defineOwner level=SIMPLE exactInventory");
}
#[test]
fn d1_multiple_inputs_keep_disjoint_complete_root_and_piece_domains() {
    let f=Fixture::new("1+1;");let other=f.dir.join("b.js");std::fs::write(&other,"1+1;").unwrap();let path=other.to_str().unwrap();success(f.run("SIMPLE",false,&["--js",path]));let plain=f.outputs();success(f.run("SIMPLE",true,&["--js",path]));assert_eq!(f.outputs(),plain);let wire=reload(&f.sidecar());let a=assert_inventory(&wire,&f.input,"1+1;","df43fbefcd7864653c637255ab18bb3fe86fbfc887d3dd15f33d61ad67d4709f",&ascii());let b=assert_inventory(&wire,&other,"1+1;","df43fbefcd7864653c637255ab18bb3fe86fbfc887d3dd15f33d61ad67d4709f",&ascii());assert!(a.iter().all(|id|!b.contains(id)));assert_ne!(input_entry(&wire,&f.input).0,input_entry(&wire,&other).0);
    eprintln!("CV04b-D1-CLI-COMPLETE case=multipleInputs level=SIMPLE exactInventory");
}
#[test]
fn d1_recording_refusal_preserves_every_preexisting_output_at_all_levels() {
    for level in LEVELS {
        let f=Fixture::new("1+1;");for (i,p) in f.paths.iter().enumerate(){std::fs::write(p,format!("sentinel {i}")).unwrap();}let before:Vec<_>=f.paths.iter().map(|p|std::fs::read(p).unwrap()).collect();let output=f.run(level,true,&["--correlation_vector_limits","max_nodes=1"]);assert_ne!(output.status.code(),Some(0));assert!(output.stdout.is_empty());assert!(String::from_utf8_lossy(&output.stderr).contains("provenance"));assert_eq!(f.paths.iter().map(|p|std::fs::read(p).unwrap()).collect::<Vec<_>>(),before);assert_eq!(std::fs::read_dir(&f.dir).unwrap().count(),5);
        eprintln!("CV04b-D1-CLI-COMPLETE case=graphRefusal level={level} sentinelPreserved");
    }
}

#[test]
fn d1_es2025_decorator_terminal_is_available_and_older_edition_refuses_it() {
    use coding_adventures_javascript_tokens::EsVersion;
    assert!(coding_adventures_javascript_lexer::tokenize_javascript_typed("@",EsVersion::Es5).is_err());
    let source="@";let f=Fixture::new(source);success(f.run("BUNDLE",false,&[]));let plain=f.outputs();success(f.run("BUNDLE",true,&[]));assert_eq!(f.outputs(),plain);
    assert_inventory_editions(&reload(&f.sidecar()),&f.input,source,"c3641f8544d7c02f3580b07c0f9887f0c6a27ff5ab1d4a3e29caf197cfc299ae",&[token("Name",0,0,1),token("EOF",1,1,1)],&[EsVersion::Es2025]);
    eprintln!("CV04b-D1-CLI-COMPLETE case=decoratorLex level=BUNDLE exactInventory");
}
