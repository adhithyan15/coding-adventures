use ibm_system360_gatelevel::gates;

type MatchArmTokens = (Vec<String>, Vec<String>);

fn rust_tokens(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        } else if bytes[cursor..].starts_with(b"//") {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else if bytes[cursor..].starts_with(b"/*") {
            cursor += 2;
            let mut depth = 1;
            while cursor < bytes.len() && depth != 0 {
                if bytes[cursor..].starts_with(b"/*") {
                    depth += 1;
                    cursor += 2;
                } else if bytes[cursor..].starts_with(b"*/") {
                    depth -= 1;
                    cursor += 2;
                } else {
                    cursor += 1;
                }
            }
        } else if bytes[cursor] == b'"' {
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor += 2;
                } else if bytes[cursor] == b'"' {
                    cursor += 1;
                    break;
                } else {
                    cursor += 1;
                }
            }
            tokens.push("<string>".to_owned());
        } else if bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(source[start..cursor].to_owned());
        } else {
            tokens.push((bytes[cursor] as char).to_string());
            cursor += 1;
        }
    }
    let mut normalized = Vec::with_capacity(tokens.len());
    for (index, token) in tokens.iter().enumerate() {
        let redundant_trailing_comma = token == ","
            && tokens
                .get(index + 1)
                .is_some_and(|next| matches!(next.as_str(), ")" | "]" | "}"));
        if !redundant_trailing_comma {
            normalized.push(token.clone());
        }
    }
    normalized
}

fn function_body(tokens: &[String], name: &str) -> Result<Vec<String>, String> {
    let starts: Vec<_> = tokens
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| (pair[0] == "fn" && pair[1] == name).then_some(index))
        .collect();
    if starts.len() != 1 {
        return Err(format!(
            "expected exactly one function named {name}, found {}",
            starts.len()
        ));
    }
    let open = (starts[0]..tokens.len())
        .find(|&index| tokens[index] == "{")
        .ok_or_else(|| format!("function {name} has no body"))?;
    let close = matching_delimiter(tokens, open, "{", "}")?;
    Ok(tokens[open + 1..close].to_vec())
}

fn matching_delimiter(
    tokens: &[String],
    open: usize,
    opening: &str,
    closing: &str,
) -> Result<usize, String> {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token == opening {
            depth += 1;
        } else if token == closing {
            depth -= 1;
            if depth == 0 {
                return Ok(index);
            }
        }
    }
    Err(format!("unclosed {opening} delimiter"))
}

fn top_level_opcode_arms(body: &[String]) -> Result<Vec<MatchArmTokens>, String> {
    let mut curly_depth = 0;
    let mut match_open = None;
    for index in 0..body.len().saturating_sub(2) {
        match body[index].as_str() {
            "{" => curly_depth += 1,
            "}" => curly_depth -= 1,
            "match" if curly_depth == 0 && body[index + 1] == "opcode" => {
                match_open = (index + 2..body.len()).find(|&next| body[next] == "{");
                break;
            }
            _ => {}
        }
    }
    let open = match_open.ok_or_else(|| "missing top-level match opcode".to_owned())?;
    let close = matching_delimiter(body, open, "{", "}")?;
    let mut arms = Vec::new();
    let mut cursor = open + 1;
    while cursor < close {
        while cursor < close && body[cursor] == "," {
            cursor += 1;
        }
        if cursor == close {
            break;
        }
        let pattern_start = cursor;
        let mut nested = 0;
        while cursor + 1 < close {
            match body[cursor].as_str() {
                "(" | "[" | "{" => nested += 1,
                ")" | "]" | "}" => nested -= 1,
                "=" if nested == 0 && body[cursor + 1] == ">" => break,
                _ => {}
            }
            cursor += 1;
        }
        if cursor + 1 >= close {
            return Err(format!(
                "malformed opcode match arm near {:?}",
                &body[pattern_start..close.min(pattern_start + 16)]
            ));
        }
        let pattern = body[pattern_start..cursor].to_vec();
        cursor += 2;
        let value_start = cursor;
        if body[cursor] == "{" {
            cursor = matching_delimiter(body, cursor, "{", "}")? + 1;
        } else {
            let mut paren = 0;
            let mut square = 0;
            while cursor < close {
                match body[cursor].as_str() {
                    "(" => paren += 1,
                    ")" => paren -= 1,
                    "[" => square += 1,
                    "]" => square -= 1,
                    "," if paren == 0 && square == 0 => break,
                    _ => {}
                }
                cursor += 1;
            }
        }
        arms.push((pattern, body[value_start..cursor].to_vec()));
    }
    Ok(arms)
}

fn require_exact_body(source: &str, function: &str, expected: &str) -> Result<(), String> {
    let actual = function_body(&rust_tokens(source), function)?;
    let expected = rust_tokens(expected);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{function} does not match its approved gate body\nactual: {actual:?}\nexpected: {expected:?}"
        ))
    }
}

fn require_exact_arm(
    source: &str,
    function: &str,
    pattern: &str,
    expected: &str,
) -> Result<(), String> {
    let body = function_body(&rust_tokens(source), function)?;
    if body.iter().any(|token| token == "use" || token == "mod") {
        return Err(format!("{function} contains a local path shadow"));
    }
    let pattern = rust_tokens(pattern);
    let matching: Vec<_> = top_level_opcode_arms(&body)
        .map_err(|error| format!("{function}: {error}"))?
        .into_iter()
        .filter_map(|(candidate, value)| (candidate == pattern).then_some(value))
        .collect();
    if matching.len() != 1 {
        return Err(format!(
            "{function} must have exactly one {:?} opcode arm",
            pattern
        ));
    }
    if matching[0] == rust_tokens(expected) {
        Ok(())
    } else {
        let expected = rust_tokens(expected);
        Err(format!(
            "{function} opcode pattern {:?} does not match its approved gate data flow\nactual: {:?}\nexpected: {:?}",
            pattern, matching[0], expected
        ))
    }
}

fn require_exact_opcode_match_shell(
    source: &str,
    function: &str,
    expected_prefix: &str,
    expected_tail: &str,
) -> Result<(), String> {
    let body = function_body(&rust_tokens(source), function)?;
    let mut curly_depth = 0;
    let mut match_start = None;
    let mut match_open = None;
    for index in 0..body.len().saturating_sub(2) {
        match body[index].as_str() {
            "{" => curly_depth += 1,
            "}" => curly_depth -= 1,
            "match" if curly_depth == 0 && body[index + 1] == "opcode" => {
                match_start = Some(index);
                match_open = (index + 2..body.len()).find(|&next| body[next] == "{");
                break;
            }
            _ => {}
        }
    }
    let start = match_start.ok_or_else(|| format!("{function} has no top-level match opcode"))?;
    let open = match_open.ok_or_else(|| format!("{function} has no top-level match opcode"))?;
    let close = matching_delimiter(&body, open, "{", "}")?;
    let actual_prefix = &body[..start];
    let actual_tail = &body[close + 1..];
    let expected_prefix = rust_tokens(expected_prefix);
    let expected_tail = rust_tokens(expected_tail);
    if actual_prefix == expected_prefix && actual_tail == expected_tail {
        Ok(())
    } else {
        Err(format!(
            "{function} has unapproved control flow around opcode dispatch\nactual prefix: {actual_prefix:?}\nexpected prefix: {expected_prefix:?}\nactual tail: {actual_tail:?}\nexpected tail: {expected_tail:?}"
        ))
    }
}

fn verify_branch_structure(networks: &str, executor: &str, validator: &str) -> Result<(), String> {
    let network_tokens = rust_tokens(networks);
    let decode_fn = network_tokens
        .windows(2)
        .position(|pair| pair == ["fn", "decode_byte"])
        .ok_or_else(|| "missing decode_byte".to_owned())?;
    let approved_imports = rust_tokens(
        "use arithmetic::adders::ripple_carry_adder_with_carry;
         use logic_gates::combinational::mux2;
         use logic_gates::gates::{and_gate,not_gate,or_gate,xor_gate};
         pub",
    );
    if network_tokens[..decode_fn] != approved_imports {
        return Err("gate primitive bindings are shadowed or changed".to_owned());
    }

    let executor_tokens = rust_tokens(executor);
    let execute_fn = executor_tokens
        .windows(2)
        .position(|pair| pair == ["fn", "execute"])
        .ok_or_else(|| "missing execute".to_owned())?;
    let approved_executor_bindings = rust_tokens(
        "use crate::{gates,hfp::GateHfp,StepTrace,System360Error,System360State};
         const ADDRESS_MASK:u32=0x00ff_ffff;
         pub(crate)",
    );
    if executor_tokens[..execute_fn] != approved_executor_bindings {
        return Err("executor gate-module binding is shadowed or changed".to_owned());
    }

    let validator_tokens = rust_tokens(validator);
    let gate_modules: Vec<_> = validator_tokens
        .windows(4)
        .enumerate()
        .filter_map(|(index, tokens)| (tokens == ["pub", "mod", "gates", ";"]).then_some(index))
        .collect();
    if gate_modules.len() != 1
        || validator_tokens
            .windows(2)
            .any(|tokens| tokens == ["as", "gates"])
        || gate_modules[0] > 0 && validator_tokens[gate_modules[0] - 1] == "]"
    {
        return Err("validator gate-module binding is shadowed or attributed".to_owned());
    }

    require_exact_body(
        networks,
        "decode_byte",
        "let inputs:[u8;8]=std::array::from_fn(|bit|(value>>bit)&1);
         std::array::from_fn(|line|{(0..8).fold(1,|selected,bit|{
             let expected=((line>>bit)&1)as u8;
             let signal=if expected==1{inputs[bit]}else{not_gate(inputs[bit])};
             and_gate(selected,signal)
         })})",
    )?;
    require_exact_body(
        networks,
        "u32_to_bits",
        "std::array::from_fn(|bit|((value>>bit)&1)as u8)",
    )?;
    require_exact_body(
        networks,
        "bits_to_u32",
        "bits.iter().enumerate().fold(0,|value,(bit,q)|value|(u32::from(*q)<<bit))",
    )?;
    require_exact_body(
        networks,
        "select_u32",
        "let d0=u32_to_bits(d0);
         let d1=u32_to_bits(d1);
         bits_to_u32(&std::array::from_fn::<_,32,_>(|bit|{mux2(d0[bit],d1[bit],select)}))",
    )?;
    require_exact_body(
        networks,
        "add_32",
        "let a_bits=u32_to_bits(a);
         let b_bits=u32_to_bits(b);
         let full=ripple_carry_adder_with_carry(&a_bits,&b_bits,carry_in);
         let low=ripple_carry_adder_with_carry(&a_bits[..31],&b_bits[..31],carry_in);
         (bits_to_u32(&full.sum),full.carry_out,xor_gate(low.carry_out,full.carry_out),)",
    )?;
    require_exact_body(
        networks,
        "add_24",
        "let a_bits=u32_to_bits(a);
         let b_bits=u32_to_bits(b);
         bits_to_u32(&ripple_carry_adder_with_carry(&a_bits[..24],&b_bits[..24],0).sum)",
    )?;
    require_exact_body(
        networks,
        "bit_not_32",
        "bits_to_u32(&u32_to_bits(value).map(not_gate))",
    )?;
    require_exact_body(
        networks,
        "branch_selected",
        "let condition=[condition_code&1,(condition_code>>1)&1];
         let mask=[mask&1,(mask>>1)&1,(mask>>2)&1,(mask>>3)&1];
         let cc_0_or_1=mux2(mask[3],mask[2],condition[0]);
         let cc_2_or_3=mux2(mask[1],mask[0],condition[0]);
         mux2(cc_0_or_1,cc_2_or_3,condition[1])",
    )?;
    require_exact_body(
        networks,
        "register_field_nonzero",
        "(0..4).fold(0,|present,bit|or_gate(present,(field>>bit)&1))",
    )?;
    require_exact_body(networks, "nonzero_32", "not_gate(is_zero_32(value))")?;
    require_exact_body(
        networks,
        "is_zero_32",
        "not_gate(u32_to_bits(value).into_iter().fold(0,or_gate))",
    )?;
    require_exact_body(
        networks,
        "count_branch_selected",
        "and_gate(nonzero_32(count),register_field_nonzero(target_register))",
    )?;
    require_exact_body(
        networks,
        "index_branch_selected",
        "let decode=decode_byte(opcode);
         let low=compare_condition_code&1;
         let high_bit=(compare_condition_code>>1)&1;
         let high=and_gate(high_bit,not_gate(low));
         or_gate(and_gate(decode[0x86],high),and_gate(decode[0x87],not_gate(high)),)",
    )?;
    require_exact_body(
        networks,
        "signed_compare_condition_code",
        "let a_sign=u32_to_bits(a)[31];
         let b_sign=u32_to_bits(b)[31];
         let signs_differ=xor_gate(a_sign,b_sign);
         let(difference,_,_)=add_32(a,bit_not_32(b),1);
         let same_sign_less=u32_to_bits(difference)[31];
         let less=or_gate(and_gate(signs_differ,a_sign),and_gate(not_gate(signs_differ),same_sign_less),);
         let equal=is_zero_32(difference);
         let greater=and_gate(not_gate(less),not_gate(equal));
         less|(greater<<1)",
    )?;

    require_exact_body(
        executor,
        "execute",
        "let mut next=before.clone();
         next.pc=gates::add_24(before.pc,u32::from(trace.width));
         match trace.raw.len(){
             2=>execute_rr(before,&mut next,&trace.raw)?,
             4=>execute_four(before,&mut next,&trace.raw)?,
             6=>execute_ss(before,&mut next,&trace.raw)?,
             _=>return gate_error(\"reason\"),
         }
         Ok(next)",
    )?;
    require_exact_body(
        executor,
        "execute_four",
        "match raw[0]{
            0x91..=0x97=>execute_si(before,next,raw),
            0x86..=0x90|0x98=>execute_rs(before,next,raw),
            _=>execute_rx(before,next,raw),
         }",
    )?;
    require_exact_opcode_match_shell(
        executor,
        "execute_rr",
        "let opcode=raw[0];
         let r1=usize::from(raw[1]>>4);
         let r2=usize::from(raw[1]&15);
         let a=before.gprs[r1];
         let b=before.gprs[r2];",
        "Ok(())",
    )?;
    require_exact_opcode_match_shell(
        executor,
        "execute_rx",
        "let opcode=raw[0];
         let register=usize::from(raw[1]>>4);
         let address=effective_address_rx(raw,before)as usize;
         let a=before.gprs[register];
         if matches!(opcode,0x60..=0x7f){return execute_float_rx(before,next,opcode,register,address);}
         let word=||read_be32(&before.memory,address);
         let half=||i32::from(read_be16(&before.memory,address)as i16)as u32;",
        "Ok(())",
    )?;
    require_exact_opcode_match_shell(
        executor,
        "execute_rs",
        "let opcode=raw[0];
         let r1=usize::from(raw[1]>>4);
         let r3=usize::from(raw[1]&15);
         let address=effective_address_base(raw,2,before)as usize;
         let count=(address as u32)&63;
         let source=before.gprs[r1];",
        "Ok(())",
    )?;
    require_exact_opcode_match_shell(
        validator,
        "validate_control_transition",
        "let opcode=trace.raw[0];let expected=",
        ";if expected.is_some_and(|pc|next.pc!=(pc&0x00ff_ffff)){
             return gate_disagrees(\"reason\");
         }
         Ok(())",
    )?;
    require_exact_body(
        validator,
        "validate_gate_transition",
        r#"let sequential_pc=gates::add_24(before.pc,u32::from(trace.width));
        let opcode=trace.raw[0];
        let decode=gates::decode_byte(opcode);
        if decode[usize::from(opcode)]!=1||decode.into_iter().map(u16::from).sum::<u16>()!=1{
            return gate_disagrees("reason");
        }
        if matches!(opcode,0x10..=0x1f){
            let r1=usize::from(trace.raw[1]>>4);
            let r2=usize::from(trace.raw[1]&15);
            let a=before.gprs[r1];
            let b=before.gprs[r2];
            let expected=match opcode{
                0x10=>Some(if b&0x8000_0000==0{b}else{gates::negate_32(b).0}),
                0x11=>Some(if b&0x8000_0000==0&&b!=0{gates::negate_32(b).0}else{b}),
                0x12|0x18=>Some(b),
                0x13=>Some(gates::negate_32(b).0),
                0x14=>Some(gates::bit_and_32(a,b)),
                0x16=>Some(gates::bit_or_32(a,b)),
                0x17=>Some(gates::bit_xor_32(a,b)),
                0x1a|0x1e=>Some(gates::add_32(a,b,0).0),
                0x1b|0x1f=>Some(gates::add_32(a,gates::bit_not_32(b),1).0),
                _=>None,
            };
            if expected.is_some_and(|value|next.gprs[r1]!=value){
                return Err(System360Error::Specification{reason:"reason"});
            }
            let expected_cc=match opcode{
                0x10..=0x13|0x1a|0x1b=>expected.map(gates::signed_condition_code),
                0x14=>expected.map(gates::logical_condition_code),
                0x15=>Some(gates::unsigned_compare_condition_code(a,b)),
                0x16|0x17=>expected.map(gates::logical_condition_code),
                0x19=>Some(gates::signed_compare_condition_code(a,b)),
                0x1e=>expected.map(|value|{
                    let carry=gates::add_32(a,b,0).1;
                    (carry<<1)|gates::logical_condition_code(value)
                }),
                0x1f=>expected.map(|value|{
                    let carry=gates::add_32(a,gates::bit_not_32(b),1).1;
                    (carry<<1)|gates::logical_condition_code(value)
                }),
                _=>None,
            };
            if expected_cc.is_some_and(|condition_code|next.condition_code!=condition_code){
                return Err(System360Error::Specification{reason:"reason"});
            }
            if opcode==0x1c{
                let product=gates::multiply_signed_32(before.gprs[r1+1],b);
                if next.gprs[r1]!=(product>>32)as u32||next.gprs[r1+1]!=product as u32{
                    return gate_disagrees("reason");
                }
            }
            if opcode==0x1d{
                let dividend=(u64::from(before.gprs[r1])<<32)|u64::from(before.gprs[r1+1]);
                let(remainder,quotient)=gates::divide_signed_64_by_32(dividend,b).ok_or(
                    System360Error::Specification{reason:"reason"},
                )?;
                if next.gprs[r1]!=remainder||next.gprs[r1+1]!=quotient{
                    return gate_disagrees("reason");
                }
            }
        }
        if matches!(opcode,0x20..=0x3f){
            validate_float_rr_transition(before,next,trace)?;
        }
        if trace.raw.len()==4{
            if matches!(opcode,0x91..=0x97){
                validate_si_transition(before,next,trace)?;
            }else if matches!(opcode,0x86..=0x90|0x98){
                validate_rs_transition(before,next,trace)?;
            }else{
                validate_rx_transition(before,next,trace)?;
            }
        }else if trace.raw.len()==6{
            validate_ss_transition(before,next,trace)?;
        }
        validate_control_transition(before,next,trace,sequential_pc)?;
        if !matches!(opcode,0x05..=0x07|0x45..=0x47|0x86|0x87)&&next.pc!=sequential_pc{
            return Err(System360Error::Specification{reason:"reason"});
        }
        Ok(())"#,
    )?;
    require_exact_body(
        validator,
        "step",
        r#"let before=self.get_state();
        let mut transition=System360Simulator::new();
        transition.restore(before.clone())?;
        let trace=transition.step()?;
        let oracle_next=transition.get_state();
        let gate_next=executor::execute(&before,&trace)?;
        validate_gate_transition(&before,&gate_next,&trace)?;
        if gate_next!=oracle_next{
            return Err(System360Error::Specification{reason:"reason"});
        }
        self.commit_transition(&gate_next);
        Ok(trace)"#,
    )?;
    require_exact_body(
        validator,
        "run",
        r#"let before=self.get_state();
        let mut traces=Vec::new();
        for _ in 0..max_steps{
            match self.step(){
                Ok(trace)=>{
                    traces.push(trace);
                    if self.halted!=0{
                        return Ok(ExecutionResult{
                            halted:true,
                            steps:traces.len(),
                            final_state:self.get_state(),
                            traces,
                        });
                    }
                }
                Err(error)=>{
                    self.restore_validated(&before);
                    return Err(error);
                }
            }
        }
        self.restore_validated(&before);
        Err(System360Error::MaxStepsExceeded{max_steps})"#,
    )?;
    require_exact_body(
        validator,
        "execute",
        "self.load(program,origin)?;self.run(max_steps)",
    )?;

    require_exact_arm(
        executor,
        "execute_rr",
        "0x05",
        "{next.gprs[r1]=next.pc;next.pc=gates::select_u32(next.pc,b&ADDRESS_MASK,gates::register_field_nonzero(r2 as u8));}",
    )?;
    require_exact_arm(
        executor,
        "execute_rr",
        "0x06",
        "{let value=gates::add_32(a,u32::MAX,0).0;next.gprs[r1]=value;next.pc=gates::select_u32(next.pc,b&ADDRESS_MASK,gates::count_branch_selected(value,r2 as u8));}",
    )?;
    require_exact_arm(
        executor,
        "execute_rr",
        "0x07",
        "{next.pc=gates::select_u32(next.pc,b&ADDRESS_MASK,gates::branch_selected(r1 as u8,before.condition_code));}",
    )?;
    require_exact_arm(
        executor,
        "execute_rx",
        "0x46",
        "{let decremented=gates::add_32(a,u32::MAX,0).0;next.gprs[register]=decremented;next.pc=gates::select_u32(next.pc,address as u32,gates::nonzero_32(decremented));}",
    )?;
    require_exact_arm(
        executor,
        "execute_rx",
        "0x47",
        "{next.pc=gates::select_u32(next.pc,address as u32,gates::branch_selected(register as u8,before.condition_code));}",
    )?;
    require_exact_arm(
        executor,
        "execute_rs",
        "0x86|0x87",
        "{let compare=if r3.is_multiple_of(2){r3+1}else{r3};let incremented=gates::add_32(source,before.gprs[r3],0).0;next.gprs[r1]=incremented;let comparison=gates::signed_compare_condition_code(incremented,before.gprs[compare]);next.pc=gates::select_u32(next.pc,address as u32,gates::index_branch_selected(opcode,comparison));}",
    )?;

    require_exact_arm(
        validator,
        "validate_control_transition",
        "0x05",
        "{let r1=usize::from(trace.raw[1]>>4);let r2=usize::from(trace.raw[1]&15);if next.gprs[r1]!=sequential_pc{return gate_disagrees(\"reason\");}Some(gates::select_u32(sequential_pc,before.gprs[r2]&0x00ff_ffff,gates::register_field_nonzero(r2 as u8)))}",
    )?;
    require_exact_arm(
        validator,
        "validate_control_transition",
        "0x06",
        "{let r1=usize::from(trace.raw[1]>>4);let r2=usize::from(trace.raw[1]&15);let decremented=gates::add_32(before.gprs[r1],u32::MAX,0).0;if next.gprs[r1]!=decremented{return gate_disagrees(\"reason\");}Some(gates::select_u32(sequential_pc,before.gprs[r2]&0x00ff_ffff,gates::count_branch_selected(decremented,r2 as u8)))}",
    )?;
    require_exact_arm(
        validator,
        "validate_control_transition",
        "0x07",
        "{let mask=trace.raw[1]>>4;let r2=usize::from(trace.raw[1]&15);Some(gates::select_u32(sequential_pc,before.gprs[r2]&0x00ff_ffff,gates::branch_selected(mask,before.condition_code)))}",
    )?;
    require_exact_arm(
        validator,
        "validate_control_transition",
        "0x45..=0x47",
        "{let r1=usize::from(trace.raw[1]>>4);let target=gate_ea_rx(&trace.raw,before);let select=match opcode{
            0x45=>{if next.gprs[r1]!=sequential_pc{return gate_disagrees(\"reason\");}1}
            0x46=>{let decremented=gates::add_32(before.gprs[r1],u32::MAX,0).0;if next.gprs[r1]!=decremented{return gate_disagrees(\"reason\");}gates::nonzero_32(decremented)}
            0x47=>gates::branch_selected(r1 as u8,before.condition_code),
            _=>unreachable!(),
         };Some(gates::select_u32(sequential_pc,target,select))}",
    )?;
    require_exact_arm(
        validator,
        "validate_control_transition",
        "0x86|0x87",
        "{let r1=usize::from(trace.raw[1]>>4);let r3=usize::from(trace.raw[1]&15);let compare=if r3.is_multiple_of(2){r3+1}else{r3};let incremented=gates::add_32(before.gprs[r1],before.gprs[r3],0).0;let comparison=gates::signed_compare_condition_code(incremented,before.gprs[compare]);Some(gates::select_u32(sequential_pc,gate_ea_base(&trace.raw,2,before),gates::index_branch_selected(opcode,comparison)))}",
    )?;
    Ok(())
}

#[test]
fn branch_predicates_cover_every_selector_wire_and_boundary() {
    for mask in 0_u8..16 {
        for condition_code in 0_u8..4 {
            let expected = u8::from(mask & (8 >> condition_code) != 0);
            assert_eq!(
                gates::branch_selected(mask, condition_code),
                expected,
                "mask={mask:#06b} cc={condition_code}"
            );
        }
    }

    for target_register in 0_u8..16 {
        let target_present = u8::from(target_register != 0);
        assert_eq!(
            gates::register_field_nonzero(target_register),
            target_present
        );
        for count in [0, 1, 2, u32::MAX, 0x8000_0000] {
            assert_eq!(
                gates::count_branch_selected(count, target_register),
                u8::from(count != 0 && target_register != 0),
                "count={count:#010x} target=r{target_register}"
            );
        }
    }

    assert_eq!(gates::nonzero_32(0), 0);
    for bit in 0..32 {
        assert_eq!(gates::nonzero_32(1_u32 << bit), 1, "count wire {bit}");
    }

    for opcode in [0x86, 0x87] {
        for comparison in 0_u8..4 {
            let expected = u8::from(if opcode == 0x86 {
                comparison == 2
            } else {
                comparison != 2
            });
            assert_eq!(
                gates::index_branch_selected(opcode, comparison),
                expected,
                "opcode={opcode:#04x} comparison={comparison}"
            );
        }
    }
    assert_eq!(gates::index_branch_selected(0xff, 2), 0);
}

#[test]
fn production_branch_paths_do_not_reconstruct_gate_predicates_on_the_host() {
    let networks = include_str!("../src/gates.rs");
    let executor = include_str!("../src/executor.rs");
    let validator = include_str!("../src/lib.rs");
    verify_branch_structure(networks, executor, validator)
        .expect("production branch data flow must match the approved gate structure");
}

#[test]
fn structural_verifier_rejects_adversarial_host_shortcuts_and_dead_anchors() {
    let networks = include_str!("../src/gates.rs");
    let executor = include_str!("../src/executor.rs");
    let validator = include_str!("../src/lib.rs");

    let alternate_host_predicate = executor.replacen(
        "gates::nonzero_32(decremented)",
        "u8::from(decremented > 0)",
        1,
    );
    assert!(verify_branch_structure(networks, &alternate_host_predicate, validator).is_err());

    let indirect_helper = executor.replacen(
        "gates::nonzero_32(decremented)",
        "branch_via_helper(decremented)",
        1,
    )
        + "\nfn branch_via_helper(value: u32) -> u8 { gates::nonzero_32(value) }\n";
    assert!(verify_branch_structure(networks, &indirect_helper, validator).is_err());

    let comment_anchor = executor.replacen(
        "gates::nonzero_32(decremented)",
        "u8::from(decremented != 0) /* gates::nonzero_32(decremented) */",
        1,
    );
    assert!(verify_branch_structure(networks, &comment_anchor, validator).is_err());

    let dead_anchor = executor.replacen(
        "gates::nonzero_32(decremented)",
        "{ if false { let _ = gates::nonzero_32(decremented); } u8::from(decremented != 0) }",
        1,
    );
    assert!(verify_branch_structure(networks, &dead_anchor, validator).is_err());

    let shadowed_primitives = networks.replacen(
        "use logic_gates::combinational::mux2;",
        "use fake_gates::mux2;",
        1,
    );
    assert!(verify_branch_structure(&shadowed_primitives, executor, validator).is_err());

    let shadowed_helpers = executor.replacen(
        "use crate::{gates, hfp::GateHfp, StepTrace, System360Error, System360State};",
        "use fake_gates as gates;\nuse crate::{hfp::GateHfp, StepTrace, System360Error, System360State};",
        1,
    );
    assert!(verify_branch_structure(networks, &shadowed_helpers, validator).is_err());

    let host_select_with_dead_anchor = networks.replacen(
        "bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {\n        mux2(d0[bit], d1[bit], select)\n    }))",
        "{ if false { let _ = mux2(d0[0], d1[0], select); } if select != 0 { d1 } else { d0 } }",
        1,
    );
    assert!(verify_branch_structure(&host_select_with_dead_anchor, executor, validator).is_err());

    let host_bit_expansion = networks.replacen(
        "std::array::from_fn(|bit| ((value >> bit) & 1) as u8)",
        "std::array::from_fn(|bit| u8::from(value & (1 << bit) != 0))",
        1,
    );
    assert!(verify_branch_structure(&host_bit_expansion, executor, validator).is_err());

    let host_add_with_dead_anchor = networks.replacen(
        "let full = ripple_carry_adder_with_carry(&a_bits, &b_bits, carry_in);\n    let low = ripple_carry_adder_with_carry(&a_bits[..31], &b_bits[..31], carry_in);\n    (\n        bits_to_u32(&full.sum),",
        "let full = ripple_carry_adder_with_carry(&a_bits, &b_bits, carry_in);\n    let low = ripple_carry_adder_with_carry(&a_bits[..31], &b_bits[..31], carry_in);\n    if false { let _ = bits_to_u32(&full.sum); }\n    (\n        a.wrapping_add(b).wrapping_add(u32::from(carry_in)),",
        1,
    );
    assert!(verify_branch_structure(&host_add_with_dead_anchor, executor, validator).is_err());

    let host_signed_compare = networks.replacen(
        "let a_sign = u32_to_bits(a)[31];",
        "if false { let _ = u32_to_bits(a)[31]; }\n    let a_sign = u8::from((a as i32) < (b as i32));",
        1,
    );
    assert!(verify_branch_structure(&host_signed_compare, executor, validator).is_err());

    let post_dispatch_overwrite = executor.replacen(
        "    Ok(next)\n}",
        "    if false { let _ = gates::select_u32(next.pc, next.pc, 0); }\n    next.pc = before.pc;\n    Ok(next)\n}",
        1,
    );
    assert!(verify_branch_structure(networks, &post_dispatch_overwrite, validator).is_err());

    let rr_post_match_overwrite = executor.replacen(
        "    Ok(())\n}\n\nfn execute_four",
        "    next.pc = before.pc;\n    Ok(())\n}\n\nfn execute_four",
        1,
    );
    assert!(verify_branch_structure(networks, &rr_post_match_overwrite, validator).is_err());

    let rx_post_match_overwrite = executor.replacen(
        "    Ok(())\n}\n\nfn execute_rs",
        "    next.pc = before.pc;\n    Ok(())\n}\n\nfn execute_rs",
        1,
    );
    assert!(verify_branch_structure(networks, &rx_post_match_overwrite, validator).is_err());

    let rs_post_match_overwrite = executor.replacen(
        "    Ok(())\n}\n\nfn execute_si",
        "    next.pc = before.pc;\n    Ok(())\n}\n\nfn execute_si",
        1,
    );
    assert!(verify_branch_structure(networks, &rs_post_match_overwrite, validator).is_err());

    let dead_four_byte_dispatch = executor.replacen(
        "    match raw[0] {",
        "    if false { return match raw[0] {",
        1,
    )
        .replacen(
            "        _ => execute_rx(before, next, raw),\n    }\n}\n\nfn execute_rx",
            "        _ => execute_rx(before, next, raw),\n        }; }\n    Ok(())\n}\n\nfn execute_rx",
            1,
        );
    assert!(verify_branch_structure(networks, &dead_four_byte_dispatch, validator).is_err());

    let shadowed_expected = validator.replacen(
        "        _ => None,\n    };\n    if expected.is_some_and(|pc| next.pc != (pc & 0x00ff_ffff)) {",
        "        _ => None,\n    };\n    let expected: Option<u32> = None;\n    if expected.is_some_and(|pc| next.pc != (pc & 0x00ff_ffff)) {",
        1,
    );
    assert!(verify_branch_structure(networks, executor, &shadowed_expected).is_err());

    let dead_validator_dispatch = validator.replacen(
        "    validate_control_transition(before, next, trace, sequential_pc)?;",
        "    if false { validate_control_transition(before, next, trace, sequential_pc)?; }",
        1,
    );
    assert!(verify_branch_structure(networks, executor, &dead_validator_dispatch).is_err());

    let oracle_as_gate_result = validator.replacen(
        "let gate_next = executor::execute(&before, &trace)?;",
        "let gate_next = oracle_next.clone();",
        1,
    );
    assert!(verify_branch_structure(networks, executor, &oracle_as_gate_result).is_err());

    let oracle_commit = validator.replacen(
        "self.commit_transition(&gate_next);",
        "self.commit_transition(&oracle_next);",
        1,
    );
    assert!(verify_branch_structure(networks, executor, &oracle_commit).is_err());

    let bypassed_run_step = validator.replacen(
        "match self.step() {",
        "match System360Simulator::new().step() {",
        1,
    );
    assert!(verify_branch_structure(networks, executor, &bypassed_run_step).is_err());

    let bypassed_execute = validator.replacen(
        "self.run(max_steps)",
        "System360Simulator::new().execute(program, origin, max_steps)",
        1,
    );
    assert!(verify_branch_structure(networks, executor, &bypassed_execute).is_err());
}
