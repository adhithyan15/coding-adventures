//! The interpreter: build the PICTURE-typed data model from WORKING-STORAGE and
//! execute the PROCEDURE DIVISION, capturing everything `DISPLAY`ed.

use crate::error::RuntimeError;
use crate::picture::Picture;
use crate::program::{Fig, Lit, Operand, Program, Stmt};
use crate::value::{add, move_into_char, move_into_numeric, mul, sub, Decimal};
use std::collections::HashMap;

/// One field in the data model. Elementary items carry a picture and character
/// storage; group items (no picture) are the concatenation of their children.
struct Item {
    level: u32,
    picture: Option<Picture>,
    storage: String,
    children: Vec<usize>,
}

/// A source value in flight during a `MOVE` or `DISPLAY`.
enum Src {
    Num(Decimal),
    Chars(String),
    Fig(Fig),
}

/// Turn an arithmetic result into a `Result`, reporting `i128` overflow (a value
/// beyond ~38 digits — larger than any real COBOL numeric field) as an error
/// rather than panicking or wrapping.
fn checked(r: Option<Decimal>) -> Result<Decimal, RuntimeError> {
    r.ok_or_else(|| RuntimeError::Unsupported("arithmetic overflow (result exceeds ~38 digits)".into()))
}

/// The running machine: the item table, a name→index map, and captured output.
pub struct Machine {
    items: Vec<Item>,
    by_name: HashMap<String, usize>,
    output: String,
}

impl Machine {
    /// Build the data model from a program's WORKING-STORAGE and initialise it.
    pub fn new(program: &Program) -> Result<Machine, RuntimeError> {
        let mut m = Machine { items: Vec::new(), by_name: HashMap::new(), output: String::new() };
        m.build_items(program)?;
        Ok(m)
    }

    fn build_items(&mut self, program: &Program) -> Result<(), RuntimeError> {
        // `stack` holds indices of currently-open group ancestors.
        let mut stack: Vec<usize> = Vec::new();

        for def in &program.data {
            // Only the hierarchy levels 01–49 and the standalone 77 are modelled
            // in v0.1. Rejecting anything else is faithful COBOL (66/88 are
            // deferred features; 50+ are invalid) and bounds the item-tree depth
            // to ≤ 49, so `group_image` recursion can never overflow the stack.
            if !(1..=49).contains(&def.level) && def.level != 77 {
                return Err(RuntimeError::Unsupported(format!(
                    "level number {:02} (v0.1 supports 01–49 and 77)",
                    def.level
                )));
            }

            let picture = match &def.picture {
                Some(p) => Some(Picture::parse(p)?),
                None => None,
            };
            // Default initial content: zeros for numeric, spaces for character.
            let storage = match &picture {
                Some(p) if p.is_numeric() => "0".repeat(p.size()),
                Some(p) => " ".repeat(p.size()),
                None => String::new(),
            };
            let idx = self.items.len();
            self.items.push(Item {
                level: def.level,
                picture,
                storage,
                children: Vec::new(),
            });

            // Register the name (duplicates need qualification — not yet supported).
            if let Some(name) = &def.name {
                if self.by_name.insert(name.clone(), idx).is_some() {
                    return Err(RuntimeError::DuplicateName(name.clone()));
                }
            }

            // Attach into the level hierarchy. 01 and 77 are top-level; 77 never
            // parents subordinates; 02–49 attach to the nearest shallower group.
            if def.level == 1 || def.level == 77 {
                stack.clear();
                if def.level == 1 {
                    stack.push(idx);
                }
            } else {
                while let Some(&top) = stack.last() {
                    if self.items[top].level >= def.level {
                        stack.pop();
                    } else {
                        break;
                    }
                }
                if let Some(&parent) = stack.last() {
                    self.items[parent].children.push(idx);
                }
                stack.push(idx);
            }

            // Apply a VALUE clause as an initialising MOVE.
            if let Some(lit) = &def.value {
                let src = self.src_from_lit(lit)?;
                self.move_into(idx, src)?;
            }
        }
        Ok(())
    }

    // ----------------------------------------------------------------------
    // Execution
    // ----------------------------------------------------------------------

    /// Run the procedure division and return the captured console output.
    pub fn run(mut self, program: &Program) -> Result<String, RuntimeError> {
        for para in &program.paragraphs {
            for stmt in &para.stmts {
                match stmt {
                    Stmt::StopRun => return Ok(self.output),
                    Stmt::Display(ops) => self.exec_display(ops)?,
                    Stmt::Move { src, dsts } => self.exec_move(src, dsts)?,
                    Stmt::Add { operands, to, giving } => self.exec_add(operands, to, giving)?,
                    Stmt::Subtract { operands, from, giving } => {
                        self.exec_subtract(operands, from, giving)?
                    }
                    Stmt::Multiply { a, by, giving } => self.exec_multiply(a, by, giving)?,
                }
            }
        }
        // Falling off the end of the procedure division ends the run too.
        Ok(self.output)
    }

    fn exec_display(&mut self, ops: &[Operand]) -> Result<(), RuntimeError> {
        let mut line = String::new();
        for op in ops {
            line.push_str(&self.display_image(op)?);
        }
        self.output.push_str(&line);
        self.output.push('\n');
        Ok(())
    }

    fn exec_move(&mut self, src: &Operand, dsts: &[String]) -> Result<(), RuntimeError> {
        for dst in dsts {
            // Resolve the source afresh per receiver (its category can differ).
            let value = self.src_from_operand(src)?;
            let idx = *self.by_name.get(dst).ok_or_else(|| RuntimeError::UndefinedName(dst.clone()))?;
            self.move_into(idx, value)?;
        }
        Ok(())
    }

    // ----------------------------------------------------------------------
    // Arithmetic (fixed-point decimal, truncating; unsigned receivers)
    // ----------------------------------------------------------------------

    /// `ADD op… TO name [GIVING g]` → (name + op1 + … + opN) into g or name.
    fn exec_add(
        &mut self,
        operands: &[Operand],
        to: &str,
        giving: &Option<String>,
    ) -> Result<(), RuntimeError> {
        let mut acc = self.named_decimal(to)?;
        for op in operands {
            acc = checked(add(&acc, &self.operand_decimal(op)?))?;
        }
        self.store_number(giving.as_deref().unwrap_or(to), acc)
    }

    /// `SUBTRACT op… FROM name [GIVING g]` → (name − op1 − … − opN) into g or name.
    fn exec_subtract(
        &mut self,
        operands: &[Operand],
        from: &str,
        giving: &Option<String>,
    ) -> Result<(), RuntimeError> {
        let mut acc = self.named_decimal(from)?;
        for op in operands {
            acc = checked(sub(&acc, &self.operand_decimal(op)?))?;
        }
        self.store_number(giving.as_deref().unwrap_or(from), acc)
    }

    /// `MULTIPLY a BY b [GIVING g]` → (a × b) into g, or into b when no GIVING.
    fn exec_multiply(
        &mut self,
        a: &Operand,
        by: &Operand,
        giving: &Option<String>,
    ) -> Result<(), RuntimeError> {
        let product = checked(mul(&self.operand_decimal(a)?, &self.operand_decimal(by)?))?;
        let target = match (giving, by) {
            (Some(g), _) => g.clone(),
            (None, Operand::Ident(name)) => name.clone(),
            (None, _) => {
                return Err(RuntimeError::Unsupported(
                    "MULTIPLY … BY <literal> without GIVING has no receiver".into(),
                ))
            }
        };
        self.store_number(&target, product)
    }

    /// The numeric value of an operand (numeric literal, `ZERO`, or numeric
    /// item). Non-numeric operands are an error — you cannot do arithmetic on
    /// an alphanumeric value.
    fn operand_decimal(&self, op: &Operand) -> Result<Decimal, RuntimeError> {
        match self.src_from_operand(op)? {
            Src::Num(d) => Ok(d),
            Src::Fig(Fig::Zero) => Ok(Decimal::zero()),
            Src::Fig(Fig::Space) | Src::Chars(_) => {
                Err(RuntimeError::Unsupported("arithmetic on a non-numeric operand".into()))
            }
        }
    }

    /// The numeric value of a named field (must be a numeric item).
    fn named_decimal(&self, name: &str) -> Result<Decimal, RuntimeError> {
        let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.into()))?;
        match &self.items[idx].picture {
            Some(p) if p.is_numeric() => Ok(self.item_as_decimal(idx)),
            _ => Err(RuntimeError::Unsupported(format!("arithmetic on non-numeric field {name}"))),
        }
    }

    /// Store a computed number into a named receiver (reshaped to its picture;
    /// an unsigned receiver keeps the magnitude).
    fn store_number(&mut self, name: &str, value: Decimal) -> Result<(), RuntimeError> {
        let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.into()))?;
        self.move_into(idx, Src::Num(value))
    }

    // ----------------------------------------------------------------------
    // MOVE
    // ----------------------------------------------------------------------

    fn move_into(&mut self, dst: usize, src: Src) -> Result<(), RuntimeError> {
        let picture = self.items[dst]
            .picture
            .clone()
            .ok_or_else(|| RuntimeError::Unsupported("MOVE into a group item".into()))?;

        let new_storage = match picture {
            Picture::Numeric { int_digits, dec_digits } => {
                let d = match src {
                    Src::Num(d) => d,
                    Src::Fig(Fig::Zero) => Decimal::zero(),
                    Src::Fig(Fig::Space) => {
                        return Err(RuntimeError::Unsupported("MOVE SPACES to a numeric item".into()))
                    }
                    Src::Chars(_) => {
                        return Err(RuntimeError::Unsupported(
                            "MOVE of an alphanumeric value to a numeric item".into(),
                        ))
                    }
                };
                move_into_numeric(&d, int_digits, dec_digits)
            }
            Picture::Alphanumeric { size } | Picture::Alphabetic { size } => {
                let chars = match src {
                    Src::Chars(s) => s,
                    Src::Num(d) => d.digits(),
                    Src::Fig(Fig::Zero) => "0".repeat(size),
                    Src::Fig(Fig::Space) => " ".repeat(size),
                };
                move_into_char(&chars, size)
            }
        };
        self.items[dst].storage = new_storage;
        Ok(())
    }

    // ----------------------------------------------------------------------
    // Source / display resolution
    // ----------------------------------------------------------------------

    fn src_from_lit(&self, lit: &Lit) -> Result<Src, RuntimeError> {
        match lit {
            Lit::Str(s) => Ok(Src::Chars(s.clone())),
            Lit::Fig(f) => Ok(Src::Fig(f.clone())),
            Lit::Num(s) => Decimal::parse_literal(s)
                .map(Src::Num)
                .ok_or_else(|| RuntimeError::Unsupported(format!("numeric literal {s}"))),
        }
    }

    fn src_from_operand(&self, op: &Operand) -> Result<Src, RuntimeError> {
        match op {
            Operand::Lit(lit) => self.src_from_lit(lit),
            Operand::Ident(name) => {
                let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                let item = &self.items[idx];
                match &item.picture {
                    Some(p) if p.is_numeric() => Ok(Src::Num(self.item_as_decimal(idx))),
                    Some(_) => Ok(Src::Chars(item.storage.clone())),
                    // A group item is treated as an alphanumeric string.
                    None => Ok(Src::Chars(self.group_image(idx))),
                }
            }
        }
    }

    /// A numeric item's value as a [`Decimal`], split by its implied decimal.
    fn item_as_decimal(&self, idx: usize) -> Decimal {
        let item = &self.items[idx];
        if let Some(Picture::Numeric { int_digits, .. }) = &item.picture {
            let int: String = item.storage.chars().take(*int_digits).collect();
            let frac: String = item.storage.chars().skip(*int_digits).collect();
            Decimal { neg: false, int, frac }
        } else {
            Decimal::zero()
        }
    }

    /// The display image of an operand.
    fn display_image(&self, op: &Operand) -> Result<String, RuntimeError> {
        match op {
            Operand::Lit(Lit::Str(s)) => Ok(s.clone()),
            Operand::Lit(Lit::Num(s)) => Ok(s.clone()),
            Operand::Lit(Lit::Fig(Fig::Zero)) => Ok("0".into()),
            Operand::Lit(Lit::Fig(Fig::Space)) => Ok(" ".into()),
            Operand::Ident(name) => {
                let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                Ok(self.item_image(idx))
            }
        }
    }

    /// An item's stored image (elementary → its storage; group → its children).
    fn item_image(&self, idx: usize) -> String {
        let item = &self.items[idx];
        if item.picture.is_some() {
            item.storage.clone()
        } else {
            self.group_image(idx)
        }
    }

    /// A group item's image: the concatenation of its children's images.
    fn group_image(&self, idx: usize) -> String {
        let mut s = String::new();
        for &child in &self.items[idx].children {
            s.push_str(&self.item_image(child));
        }
        s
    }
}
