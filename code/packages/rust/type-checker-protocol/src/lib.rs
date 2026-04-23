use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeErrorDiagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeCheckResult<T> {
    pub typed_ast: T,
    pub errors: Vec<TypeErrorDiagnostic>,
    pub ok: bool,
}

pub trait TypeChecker<AstIn, AstOut> {
    fn check(&self, ast: AstIn) -> TypeCheckResult<AstOut>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotHandled;

pub type Locator = Box<dyn Fn(&dyn std::any::Any) -> (usize, usize)>;
pub type NodeKind<T> = Box<dyn Fn(&T) -> String>;
pub type Hook<T> = Box<dyn Fn(&T, &[&dyn std::any::Any]) -> Option<Box<dyn std::any::Any>>>;

pub struct GenericTypeChecker<T> {
    hooks: HashMap<String, Vec<Hook<T>>>,
    errors: Vec<TypeErrorDiagnostic>,
    node_kind: Option<NodeKind<T>>,
    locate: Locator,
}

impl<T> GenericTypeChecker<T> {
    pub fn new(node_kind: Option<NodeKind<T>>, locate: Option<Locator>) -> Self {
        Self {
            hooks: HashMap::new(),
            errors: Vec::new(),
            node_kind,
            locate: locate.unwrap_or_else(|| Box::new(|_| (1, 1))),
        }
    }

    pub fn reset(&mut self) {
        self.errors.clear();
    }

    pub fn register_hook(&mut self, phase: &str, kind: &str, hook: Hook<T>) {
        let key = format!("{}:{}", phase, normalize_kind(kind));
        self.hooks.entry(key).or_default().push(hook);
    }

    pub fn dispatch(
        &self,
        phase: &str,
        node: &T,
        args: &[&dyn std::any::Any],
    ) -> Option<Box<dyn std::any::Any>> {
        let normalized_kind = self
            .node_kind
            .as_ref()
            .map(|kind| normalize_kind(&kind(node)))
            .unwrap_or_default();

        for key in [
            format!("{}:{}", phase, normalized_kind),
            format!("{}:*", phase),
        ] {
            if let Some(hooks) = self.hooks.get(&key) {
                for hook in hooks {
                    if let Some(value) = hook(node, args) {
                        if value.downcast_ref::<NotHandled>().is_none() {
                            return Some(value);
                        }
                    }
                }
            }
        }

        None
    }

    pub fn error(&mut self, message: impl Into<String>, subject: &dyn std::any::Any) {
        let (line, column) = (self.locate)(subject);
        self.errors.push(TypeErrorDiagnostic {
            message: message.into(),
            line,
            column,
        });
    }

    pub fn errors(&self) -> Vec<TypeErrorDiagnostic> {
        self.errors.clone()
    }
}

fn normalize_kind(kind: &str) -> String {
    let mut out = String::new();
    let mut last_underscore = false;

    for ch in kind.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_underscore = false;
        } else if !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }

    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_kind_collapses_punctuation() {
        assert_eq!(normalize_kind("expr:add"), "expr_add");
        assert_eq!(normalize_kind("  fn decl "), "fn_decl");
    }
}
