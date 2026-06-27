use std::env;
use std::io::{self, Read};

use math_frontend::{BinOp, MathExpr, UnaryOp};

fn main() {
    let mut src = env::args().skip(1).collect::<Vec<_>>().join(" ");
    if src.trim().is_empty() {
        let mut stdin = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut stdin) {
            eprintln!("failed to read stdin: {e}");
            std::process::exit(2);
        }
        src = stdin;
    }

    match to_adj_formula_from_latex(&src) {
        Ok(formula) => println!("{formula}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn to_adj_formula_from_latex(src: &str) -> Result<String, String> {
    let math = strip_math_delimiters(src);
    let reg = latex::registry();
    let expr = reg
        .parse("latex", math)
        .map_err(|e| format!("latex parse failed: {}", e.message))?;
    let rendered = render_expr(&expr)?;
    Ok(rendered.text)
}

fn strip_math_delimiters(src: &str) -> &str {
    let mut s = src.trim();
    loop {
        let next = if s.starts_with("\\(") && s.ends_with("\\)") && s.len() >= 4 {
            Some(&s[2..s.len() - 2])
        } else if s.starts_with("\\[") && s.ends_with("\\]") && s.len() >= 4 {
            Some(&s[2..s.len() - 2])
        } else if s.starts_with("$$") && s.ends_with("$$") && s.len() >= 4 {
            Some(&s[2..s.len() - 2])
        } else if s.starts_with('$') && s.ends_with('$') && s.len() >= 2 {
            Some(&s[1..s.len() - 1])
        } else {
            None
        };
        match next {
            Some(inner) => s = inner.trim(),
            None => return s,
        }
    }
}

struct Rendered {
    text: String,
    prec: u8,
}

fn render_expr(expr: &MathExpr) -> Result<Rendered, String> {
    match expr {
        MathExpr::Number(n) => Ok(Rendered {
            text: n.as_written().to_string(),
            prec: 5,
        }),
        MathExpr::Group(x) => {
            let x = render_expr(x)?;
            Ok(Rendered {
                text: format!("({})", x.text),
                prec: 5,
            })
        }
        MathExpr::Unary(UnaryOp::Neg, x) => render_unary("-", x),
        MathExpr::Unary(UnaryOp::Pos, x) => render_unary("+", x),
        MathExpr::Bin(op, x, y) => render_bin(*op, x, y),
        MathExpr::Frac(x, y) => render_binary("/", 2, true, x, y),
        other => Err(format!("unsupported ADJ arithmetic subset: {other:?}")),
    }
}

fn render_unary(prefix: &str, x: &MathExpr) -> Result<Rendered, String> {
    let x = render_expr(x)?;
    let body = if x.prec < 4 {
        format!("({})", x.text)
    } else {
        x.text
    };
    Ok(Rendered {
        text: format!("{prefix}{body}"),
        prec: 4,
    })
}

fn render_bin(op: BinOp, x: &MathExpr, y: &MathExpr) -> Result<Rendered, String> {
    match op {
        BinOp::Add => render_binary("+", 1, false, x, y),
        BinOp::Sub => render_binary("-", 1, true, x, y),
        BinOp::Mul => render_binary("*", 2, false, x, y),
        BinOp::Div => render_binary("/", 2, true, x, y),
        BinOp::Pow | BinOp::PlusMinus | BinOp::MinusPlus => {
            Err(format!("unsupported ADJ arithmetic operator: {op:?}"))
        }
    }
}

fn render_binary(
    op: &str,
    prec: u8,
    right_assoc_sensitive: bool,
    x: &MathExpr,
    y: &MathExpr,
) -> Result<Rendered, String> {
    let left = render_expr(x)?;
    let right = render_expr(y)?;
    let left_text = if left.prec < prec {
        format!("({})", left.text)
    } else {
        left.text
    };
    let right_text = if right.prec < prec || (right_assoc_sensitive && right.prec == prec) {
        format!("({})", right.text)
    } else {
        right.text
    };
    Ok(Rendered {
        text: format!("{left_text} {op} {right_text}"),
        prec,
    })
}

#[cfg(test)]
mod tests {
    use super::{strip_math_delimiters, to_adj_formula_from_latex};

    #[test]
    fn strips_common_math_delimiters() {
        assert_eq!(strip_math_delimiters(r"$5 \times 12$"), r"5 \times 12");
        assert_eq!(strip_math_delimiters(r"\(5 + 12\)"), "5 + 12");
        assert_eq!(strip_math_delimiters(r"\[\frac{12}{3}\]"), r"\frac{12}{3}");
    }

    #[test]
    fn lowers_latex_arithmetic_to_adj_formula() {
        assert_eq!(
            to_adj_formula_from_latex(r"$5 \times 12$").unwrap(),
            "5 * 12"
        );
        assert_eq!(
            to_adj_formula_from_latex(r"\frac{12}{3}").unwrap(),
            "12 / 3"
        );
        assert_eq!(
            to_adj_formula_from_latex(r"5 \cdot (12 + 3)").unwrap(),
            "5 * (12 + 3)"
        );
    }

    #[test]
    fn rejects_non_arithmetic_subset() {
        assert!(to_adj_formula_from_latex(r"x + 1").is_err());
        assert!(to_adj_formula_from_latex(r"5^2").is_err());
    }
}
