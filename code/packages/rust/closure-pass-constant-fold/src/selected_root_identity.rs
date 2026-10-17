//! A selected clone changes history at its root, not its JavaScript payload.
//!
//! This exhaustive borrowed slot is deliberately smaller than a general AST
//! visitor. Adding an Expression variant must receive an explicit decision;
//! nested identities, bodies, raw spellings and signed zero are untouched.
use coding_adventures_javascript_ast::Expression;

#[inline(never)]
pub(super) fn slot(expression: &mut Expression) -> &mut Option<String> {
    match expression {
        Expression::Identifier(n) => &mut n.cv,
        Expression::NumericLiteral(n) => &mut n.cv,
        Expression::StringLiteral(n) => &mut n.cv,
        Expression::BooleanLiteral(n) => &mut n.cv,
        Expression::NullLiteral(n) => &mut n.cv,
        Expression::BigIntLiteral(n) => &mut n.cv,
        Expression::UndefinedLiteral(n) => &mut n.cv,
        Expression::RegExpLiteral(n) => &mut n.cv,
        Expression::BinaryExpression(n) => &mut n.cv,
        Expression::LogicalExpression(n) => &mut n.cv,
        Expression::UnaryExpression(n) => &mut n.cv,
        Expression::AssignmentExpression(n) => &mut n.cv,
        Expression::ConditionalExpression(n) => &mut n.cv,
        Expression::CallExpression(n) => &mut n.cv,
        Expression::MemberExpression(n) => &mut n.cv,
        Expression::OptionalMemberExpression(n) => &mut n.cv,
        Expression::OptionalCallExpression(n) => &mut n.cv,
        Expression::ChainExpression(n) => &mut n.cv,
        Expression::ArrayExpression(n) => &mut n.cv,
        Expression::ObjectExpression(n) => &mut n.cv,
        Expression::FunctionExpression(n) => &mut n.cv,
        Expression::ArrowFunctionExpression(n) => &mut n.cv,
        Expression::ClassExpression(n) => &mut n.cv,
        Expression::TemplateLiteral(n) => &mut n.cv,
        Expression::UpdateExpression(n) => &mut n.cv,
        Expression::NewExpression(n) => &mut n.cv,
        Expression::SequenceExpression(n) => &mut n.cv,
        Expression::TaggedTemplateExpression(n) => &mut n.cv,
        Expression::SpreadElement(n) => &mut n.cv,
        Expression::YieldExpression(n) => &mut n.cv,
        Expression::AwaitExpression(n) => &mut n.cv,
        Expression::ThisExpression(n) => &mut n.cv,
        Expression::Super(n) => &mut n.cv,
        Expression::NewTarget(n) => &mut n.cv,
        Expression::ImportMeta(n) => &mut n.cv,
        Expression::ImportExpression(n) => &mut n.cv,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_javascript_ast::{Identifier, SpreadElement};

    #[test]
    fn c36_spread_root_slot_preserves_child_without_changing_selection_eligibility() {
        // Other 35 slot arms execute through actual native selections. Spread
        // correctly refuses array selection, so exercise its slot directly.
        let child = Expression::Identifier(Identifier {
            cv: Some("child".into()),
            name: "xs".into(),
        });
        let mut expression = Expression::SpreadElement(SpreadElement {
            cv: Some("old".into()),
            argument: Box::new(child.clone()),
        });
        assert_eq!(slot(&mut expression).as_deref(), Some("old"));
        *slot(&mut expression) = Some("replacement".into());
        let Expression::SpreadElement(spread) = expression else {
            panic!("variant preserved");
        };
        assert_eq!(spread.cv.as_deref(), Some("replacement"));
        assert_eq!(*spread.argument, child);
    }
}
