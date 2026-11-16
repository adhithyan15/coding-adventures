//! Deterministic VM runner executing lifted `InterpreterIR` modules on `vm-core`.

#![deny(unsafe_code)]

use interpreter_ir::module::IIRModule;
use vm_core::value::Value;
use vm_core::VMCore;

use super::environment::AvmEnvironment;
use crate::error::{LifterError, MAX_VM_STEPS};

fn format_avm_value(v: &Value) -> String {
    match v {
        Value::Str(bytes) => String::from_utf8_lossy(bytes).to_string(),
        Value::Null => "null".to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => {
            if f.fract() == 0.0 && *f >= (i64::MIN as f64) && *f <= (i64::MAX as f64) {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// AVM execution runner harness managing `vm-core` and Flash built-ins.
pub struct AvmRunner {
    environment: AvmEnvironment,
}

impl Default for AvmRunner {
    fn default() -> Self {
        Self::runner_new()
    }
}

impl AvmRunner {
    /// Create a new AVM execution runner.
    pub fn runner_new() -> Self {
        Self {
            environment: AvmEnvironment::env_new(),
        }
    }

    /// Retrieve reference to runtime environment.
    pub fn runner_environment(&self) -> &AvmEnvironment {
        &self.environment
    }

    /// Execute an IIR module and return the result of `main`.
    pub fn runner_execute(&mut self, module: &IIRModule) -> Result<Value, LifterError> {
        let mut vm = VMCore::new();
        // SWF00 §7.2: Hard instruction budget <= 2,000,000 steps per frame tick
        vm.max_instructions = Some(MAX_VM_STEPS);
        self.runner_register_builtins(&mut vm)?;

        let mut mod_clone = module.clone();
        let entry = mod_clone
            .entry_point
            .clone()
            .unwrap_or_else(|| "main".to_string());

        let res = vm.execute(&mut mod_clone, &entry, &[]).map_err(|e| {
            let msg = format!("{:?}", e);
            if msg.contains("execution step limit") {
                LifterError::ScriptTimeout
            } else if msg.contains("prototype pollution") {
                LifterError::PrototypePollutionBlocked(msg)
            } else {
                LifterError::ExecutionError(msg)
            }
        })?;

        Ok(res.unwrap_or(Value::Null))
    }

    /// Retrieve captured trace/print output.
    pub fn runner_output(&self) -> String {
        self.environment.env_get_output()
    }

    /// Retrieve logged Flash timeline actions.
    pub fn runner_action_log(&self) -> Vec<String> {
        self.environment.env_get_action_log()
    }

    fn runner_register_builtins(&self, vm: &mut VMCore) -> Result<(), LifterError> {
        let env = self.environment.clone();
        let builtins = vm.builtins_mut();

        // 1. Trace
        let env_tr = env.clone();
        builtins.register("flash_trace", move |args| {
            let s = if let Some(v) = args.first() {
                format_avm_value(v)
            } else {
                String::new()
            };
            env_tr.env_append_output(&s);
            Ok(Value::Null)
        });

        let env_tr2 = env.clone();
        builtins.register("trace", move |args| {
            let s = if let Some(v) = args.first() {
                format_avm_value(v)
            } else {
                String::new()
            };
            env_tr2.env_append_output(&s);
            Ok(Value::Null)
        });

        // 2. Constants: Null & Undefined
        builtins.register("flash_null", |_| Ok(Value::Null));
        builtins.register("flash_undefined", |_| Ok(Value::Null));

        // 3. Polymorphic arithmetic
        builtins.register("flash_add2", |args| {
            if args.len() < 2 {
                return Ok(Value::Int(0));
            }
            let a = &args[0];
            let b = &args[1];
            // If either operand is a string, perform string concatenation
            if let Value::Str(bytes_a) = a {
                let sa = String::from_utf8_lossy(bytes_a);
                let sb = format_avm_value(b);
                return Ok(Value::str(format!("{}{}", sa, sb)));
            }
            if let Value::Str(bytes_b) = b {
                let sa = format_avm_value(a);
                let sb = String::from_utf8_lossy(bytes_b);
                return Ok(Value::str(format!("{}{}", sa, sb)));
            }
            // Otherwise numeric addition
            match (a, b) {
                (Value::Float(fa), Value::Float(fb)) => Ok(Value::Float(fa + fb)),
                (Value::Float(fa), Value::Int(ib)) => Ok(Value::Float(fa + (*ib as f64))),
                (Value::Int(ia), Value::Float(fb)) => Ok(Value::Float((*ia as f64) + fb)),
                (Value::Int(ia), Value::Int(ib)) => Ok(Value::Int(ia.wrapping_add(*ib))),
                _ => Ok(Value::Int(0)),
            }
        });

        builtins.register("flash_equals2", |args| {
            if args.len() < 2 {
                return Ok(Value::Bool(false));
            }
            let eq = match (&args[0], &args[1]) {
                (Value::Int(a), Value::Int(b)) => a == b,
                (Value::Float(a), Value::Float(b)) => a == b,
                (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
                (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
                (Value::Str(a), Value::Str(b)) => a == b,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                (Value::Null, Value::Null) => true,
                _ => false,
            };
            Ok(Value::Bool(eq))
        });

        builtins.register("flash_less2", |args| {
            if args.len() < 2 {
                return Ok(Value::Bool(false));
            }
            let lt = match (&args[0], &args[1]) {
                (Value::Int(a), Value::Int(b)) => a < b,
                (Value::Float(a), Value::Float(b)) => a < b,
                (Value::Int(a), Value::Float(b)) => (*a as f64) < *b,
                (Value::Float(a), Value::Int(b)) => *a < (*b as f64),
                (Value::Str(a), Value::Str(b)) => a < b,
                _ => false,
            };
            Ok(Value::Bool(lt))
        });

        // 4. Variables (with CWE-1321 prototype protection)
        let env_gvar = env.clone();
        builtins.register("flash_get_variable", move |args| {
            let name = args.first().map_or("".to_string(), format_avm_value);
            Ok(env_gvar.env_get_variable(&name))
        });

        let env_svar = env.clone();
        builtins.register("flash_set_variable", move |args| {
            if args.len() >= 2 {
                let name = format_avm_value(&args[0]);
                let val = args[1].clone();
                env_svar
                    .env_set_variable(&name, val)
                    .map_err(|e| vm_core::VMError::Custom(e.to_string()))?;
            }
            Ok(Value::Null)
        });

        // 5. Member Access (with CWE-1321 prototype protection)
        let env_gmem = env.clone();
        builtins.register("flash_get_member", move |args| {
            if args.len() >= 2 {
                let obj_id = args[0].as_i64().unwrap_or(0) as usize;
                let prop = format_avm_value(&args[1]);
                Ok(env_gmem.env_get_member(obj_id, &prop))
            } else {
                Ok(Value::Null)
            }
        });

        let env_smem = env.clone();
        builtins.register("flash_set_member", move |args| {
            if args.len() >= 3 {
                let obj_id = args[0].as_i64().unwrap_or(0) as usize;
                let prop = format_avm_value(&args[1]);
                let val = args[2].clone();
                env_smem
                    .env_set_member(obj_id, &prop, val)
                    .map_err(|e| vm_core::VMError::Custom(e.to_string()))?;
            }
            Ok(Value::Null)
        });

        // 6. Object Creation
        let env_nobj = env.clone();
        builtins.register("flash_new_object", move |args| {
            let ctor = args.first().map_or("Object".to_string(), format_avm_value);
            let id = env_nobj.env_new_object(&ctor);
            Ok(Value::Int(id as i64))
        });

        // 7. Timeline verbs
        let env_stop = env.clone();
        builtins.register("flash_stop", move |_| {
            env_stop.env_log_action("stop");
            Ok(Value::Null)
        });

        let env_play = env.clone();
        builtins.register("flash_play", move |_| {
            env_play.env_log_action("play");
            Ok(Value::Null)
        });

        let env_goto = env.clone();
        builtins.register("flash_goto_frame", move |args| {
            let f = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            env_goto.env_log_action(&format!("gotoFrame({})", f));
            Ok(Value::Null)
        });

        // 8. Dynamic call / property stubs
        let env_cprop = env.clone();
        builtins.register("flash_call_property", move |args| {
            let method = args.first().map_or("".to_string(), format_avm_value);
            env_cprop.env_log_action(&format!("callProperty({})", method));
            Ok(Value::Null)
        });

        let env_cpvoid = env.clone();
        builtins.register("flash_call_propvoid", move |args| {
            let method = args.first().map_or("".to_string(), format_avm_value);
            env_cpvoid.env_log_action(&format!("callPropVoid({})", method));
            Ok(Value::Null)
        });

        builtins.register("flash_find_property", move |args| {
            let prop = args.first().map_or("".to_string(), format_avm_value);
            Ok(Value::str(prop))
        });

        builtins.register("flash_call_function", move |args| {
            let fname = args.first().map_or("".to_string(), format_avm_value);
            Ok(Value::str(fname))
        });

        builtins.register("flash_call_method", move |args| {
            let mname = args.get(1).map_or("".to_string(), format_avm_value);
            Ok(Value::str(mname))
        });

        Ok(())
    }
}
