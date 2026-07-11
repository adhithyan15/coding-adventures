//! The interpreter: build the PICTURE-typed data model from WORKING-STORAGE and
//! execute the PROCEDURE DIVISION, capturing everything `DISPLAY`ed.

use crate::error::RuntimeError;
use crate::picture::Picture;
use crate::program::{Fig, Lit, Operand, Program, Stmt};
use crate::value::{move_into_char, move_into_numeric, Decimal};
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
