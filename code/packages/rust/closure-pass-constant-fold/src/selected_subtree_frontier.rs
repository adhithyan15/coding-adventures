//! A selected subtree can contain newer identities than its composite root.
//!
//! Walk borrowed native fields in preorder without recursion or a sibling
//! queue. Count first, reuse the frames, then replay into a metered first-seen
//! table. Both scans share one work budget and every live requested buffer
//! participates in the same storage cap before allocation or growth.
use coding_adventures_correlation_vector::GraphLimits;
use coding_adventures_javascript_ast::statement::TaggedStatement;
use coding_adventures_javascript_ast::*;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

type Allocate<'a> = dyn FnMut(usize) -> Result<(), String> + 'a;
type Visit<'a, 'b> = dyn FnMut(&'a str, &mut Budget) -> Result<(), String> + 'b;

struct Budget {
    work: usize,
    bytes: usize,
}
impl Budget {
    fn take(&mut self, units: usize) -> Result<(), String> {
        self.work = self
            .work
            .checked_sub(units)
            .ok_or("CV selected frontier work limit exceeded")?;
        Ok(())
    }
    fn storage(&self, bytes: Option<usize>) -> Result<(), String> {
        let bytes = bytes.ok_or("CV selected frontier buffer accounting overflow")?;
        if bytes > self.bytes {
            return Err("CV selected frontier buffer limit exceeded".into());
        }
        Ok(())
    }
}

enum Frame<'a> {
    ArrayExpression(&'a ArrayExpression),
    ArrowBody(&'a ArrowBody),
    ArrowFunctionExpression(&'a ArrowFunctionExpression),
    AssignmentExpression(&'a AssignmentExpression),
    AssignmentPattern(&'a AssignmentPattern),
    AssignmentTarget(&'a AssignmentTarget),
    AwaitExpression(&'a AwaitExpression),
    BigIntLiteral(&'a BigIntLiteral),
    BinaryExpression(&'a BinaryExpression),
    BindingTarget(&'a BindingTarget),
    BlockStatement(&'a BlockStatement),
    BooleanLiteral(&'a BooleanLiteral),
    BreakStatement(&'a BreakStatement),
    CallExpression(&'a CallExpression),
    CatchClause(&'a CatchClause),
    ChainExpression(&'a ChainExpression),
    ClassDeclaration(&'a ClassDeclaration),
    ClassExpression(&'a ClassExpression),
    ClassMember(&'a ClassMember),
    ConditionalExpression(&'a ConditionalExpression),
    ContinueStatement(&'a ContinueStatement),
    DebuggerStatement(&'a DebuggerStatement),
    Declaration(&'a Declaration),
    DoWhileStatement(&'a DoWhileStatement),
    EmptyStatement(&'a EmptyStatement),
    ExportAllDeclaration(&'a ExportAllDeclaration),
    ExportDefaultDeclaration(&'a ExportDefaultDeclaration),
    ExportDefaultKind(&'a ExportDefaultKind),
    ExportNamedDeclaration(&'a ExportNamedDeclaration),
    ExportSpecifier(&'a ExportSpecifier),
    Expression(&'a Expression),
    ExpressionStatement(&'a ExpressionStatement),
    ForInStatement(&'a ForInStatement),
    ForInit(&'a ForInit),
    ForOfStatement(&'a ForOfStatement),
    ForStatement(&'a ForStatement),
    FunctionDeclaration(&'a FunctionDeclaration),
    FunctionExpression(&'a FunctionExpression),
    FunctionParam(&'a FunctionParam),
    Identifier(&'a Identifier),
    IfStatement(&'a IfStatement),
    ImportDeclaration(&'a ImportDeclaration),
    ImportExpression(&'a ImportExpression),
    ImportMeta(&'a ImportMeta),
    ImportSpecifier(&'a ImportSpecifier),
    LabeledStatement(&'a LabeledStatement),
    LogicalExpression(&'a LogicalExpression),
    MemberExpression(&'a MemberExpression),
    MethodDefinition(&'a MethodDefinition),
    NewExpression(&'a NewExpression),
    NewTarget(&'a NewTarget),
    NullLiteral(&'a NullLiteral),
    NumericLiteral(&'a NumericLiteral),
    ObjectExpression(&'a ObjectExpression),
    ObjectMember(&'a ObjectMember),
    OptionalCallExpression(&'a OptionalCallExpression),
    OptionalMemberExpression(&'a OptionalMemberExpression),
    PrivateName(&'a PrivateName),
    Property(&'a Property),
    PropertyDefinition(&'a PropertyDefinition),
    PropertyKey(&'a PropertyKey),
    RegExpLiteral(&'a RegExpLiteral),
    RestElement(&'a RestElement),
    ReturnStatement(&'a ReturnStatement),
    SequenceExpression(&'a SequenceExpression),
    SpreadElement(&'a SpreadElement),
    Statement(&'a Statement),
    StringLiteral(&'a StringLiteral),
    Super(&'a Super),
    SwitchCase(&'a SwitchCase),
    SwitchStatement(&'a SwitchStatement),
    TaggedStatement(&'a TaggedStatement),
    TaggedTemplateExpression(&'a TaggedTemplateExpression),
    TemplateElement(&'a TemplateElement),
    TemplateLiteral(&'a TemplateLiteral),
    ThisExpression(&'a ThisExpression),
    ThrowStatement(&'a ThrowStatement),
    TryStatement(&'a TryStatement),
    UnaryExpression(&'a UnaryExpression),
    UndefinedLiteral(&'a UndefinedLiteral),
    UpdateExpression(&'a UpdateExpression),
    VariableDeclaration(&'a VariableDeclaration),
    VariableDeclarator(&'a VariableDeclarator),
    WhileStatement(&'a WhileStatement),
    WithStatement(&'a WithStatement),
    YieldExpression(&'a YieldExpression),
    ClassMemberList(std::slice::Iter<'a, ClassMember>),
    ExportSpecifierList(std::slice::Iter<'a, ExportSpecifier>),
    ExpressionList(std::slice::Iter<'a, Expression>),
    FunctionParamList(std::slice::Iter<'a, FunctionParam>),
    ImportSpecifierList(std::slice::Iter<'a, ImportSpecifier>),
    ObjectMemberList(std::slice::Iter<'a, ObjectMember>),
    OptionalExpressionList(std::slice::Iter<'a, Option<Expression>>),
    StatementList(std::slice::Iter<'a, Statement>),
    SwitchCaseList(std::slice::Iter<'a, SwitchCase>),
    TemplateElementList(std::slice::Iter<'a, TemplateElement>),
    VariableDeclaratorList(std::slice::Iter<'a, VariableDeclarator>),
}

struct Stack<'a> {
    frames: Vec<Frame<'a>>,
    fixed_bytes: usize,
}
impl<'a> Stack<'a> {
    fn new() -> Self {
        Self {
            frames: Vec::new(),
            fixed_bytes: 0,
        }
    }
    fn push(
        &mut self,
        frame: Frame<'a>,
        budget: &mut Budget,
        allocate: &mut Allocate<'_>,
    ) -> Result<(), String> {
        budget.take(1)?;
        if self.frames.len() == self.frames.capacity() {
            let old = self.frames.capacity();
            let next = if old == 0 {
                4
            } else {
                old.checked_mul(2)
                    .ok_or("CV selected frontier frame overflow")?
            };
            let requested = next
                .checked_mul(std::mem::size_of::<Frame<'a>>())
                .ok_or("CV selected frontier frame overflow")?;
            // During migration both frame allocations coexist with any ID
            // buffers. Charge setup and each copied frame before allocation.
            budget.storage(
                old.checked_add(next)
                    .and_then(|n| n.checked_mul(std::mem::size_of::<Frame<'a>>()))
                    .and_then(|n| n.checked_add(self.fixed_bytes)),
            )?;
            budget.take(
                1usize
                    .checked_add(self.frames.len())
                    .ok_or("CV selected frontier work overflow")?,
            )?;
            allocate(requested)?;
            let mut replacement = Vec::new();
            replacement
                .try_reserve_exact(next)
                .map_err(|_| "CV selected frontier allocation refused")?;
            // try_reserve_exact is a request, not an allocator-memory theorem.
            // A different reported capacity cannot silently escape accounting.
            if replacement.capacity() != next {
                return Err("CV selected frontier unexpected buffer capacity".into());
            }
            replacement.append(&mut self.frames);
            self.frames = replacement;
        }
        self.frames.push(frame);
        Ok(())
    }
    fn bytes(&self) -> Option<usize> {
        self.frames
            .capacity()
            .checked_mul(std::mem::size_of::<Frame<'a>>())
    }
}

fn cv_slot<'a>(
    id: Option<&'a str>,
    budget: &mut Budget,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    budget.take(1)?; // An absent identity is still an inspected source slot.
    if let Some(id) = id {
        budget.take(id.len())?;
        visit(id, budget)?;
    }
    Ok(())
}

fn scan<'a>(
    prefix: [Option<&'a str>; 3],
    selected: &'a Expression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    for id in prefix {
        cv_slot(id, budget, visit)?;
    }
    stack.push(Frame::Expression(selected), budget, allocate)?;
    while !stack.frames.is_empty() {
        budget.take(1)?; // Pop precedes inspection; empty loop does not pop.
        let frame = stack.frames.pop().expect("checked nonempty frame stack");
        budget.take(1)?; // Every native enum/struct/list dispatch is charged.
        step(frame, stack, budget, allocate, visit)?;
    }
    Ok(())
}

fn step<'a>(
    frame: Frame<'a>,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match frame {
        Frame::ArrayExpression(node) => step_arrayexpression(node, stack, budget, allocate, visit),
        Frame::ArrowBody(node) => step_arrowbody(node, stack, budget, allocate, visit),
        Frame::ArrowFunctionExpression(node) => {
            step_arrowfunctionexpression(node, stack, budget, allocate, visit)
        }
        Frame::AssignmentExpression(node) => {
            step_assignmentexpression(node, stack, budget, allocate, visit)
        }
        Frame::AssignmentPattern(node) => {
            step_assignmentpattern(node, stack, budget, allocate, visit)
        }
        Frame::AssignmentTarget(node) => {
            step_assignmenttarget(node, stack, budget, allocate, visit)
        }
        Frame::AwaitExpression(node) => step_awaitexpression(node, stack, budget, allocate, visit),
        Frame::BigIntLiteral(node) => step_bigintliteral(node, stack, budget, allocate, visit),
        Frame::BinaryExpression(node) => {
            step_binaryexpression(node, stack, budget, allocate, visit)
        }
        Frame::BindingTarget(node) => step_bindingtarget(node, stack, budget, allocate, visit),
        Frame::BlockStatement(node) => step_blockstatement(node, stack, budget, allocate, visit),
        Frame::BooleanLiteral(node) => step_booleanliteral(node, stack, budget, allocate, visit),
        Frame::BreakStatement(node) => step_breakstatement(node, stack, budget, allocate, visit),
        Frame::CallExpression(node) => step_callexpression(node, stack, budget, allocate, visit),
        Frame::CatchClause(node) => step_catchclause(node, stack, budget, allocate, visit),
        Frame::ChainExpression(node) => step_chainexpression(node, stack, budget, allocate, visit),
        Frame::ClassDeclaration(node) => {
            step_classdeclaration(node, stack, budget, allocate, visit)
        }
        Frame::ClassExpression(node) => step_classexpression(node, stack, budget, allocate, visit),
        Frame::ClassMember(node) => step_classmember(node, stack, budget, allocate, visit),
        Frame::ConditionalExpression(node) => {
            step_conditionalexpression(node, stack, budget, allocate, visit)
        }
        Frame::ContinueStatement(node) => {
            step_continuestatement(node, stack, budget, allocate, visit)
        }
        Frame::DebuggerStatement(node) => {
            step_debuggerstatement(node, stack, budget, allocate, visit)
        }
        Frame::Declaration(node) => step_declaration(node, stack, budget, allocate, visit),
        Frame::DoWhileStatement(node) => {
            step_dowhilestatement(node, stack, budget, allocate, visit)
        }
        Frame::EmptyStatement(node) => step_emptystatement(node, stack, budget, allocate, visit),
        Frame::ExportAllDeclaration(node) => {
            step_exportalldeclaration(node, stack, budget, allocate, visit)
        }
        Frame::ExportDefaultDeclaration(node) => {
            step_exportdefaultdeclaration(node, stack, budget, allocate, visit)
        }
        Frame::ExportDefaultKind(node) => {
            step_exportdefaultkind(node, stack, budget, allocate, visit)
        }
        Frame::ExportNamedDeclaration(node) => {
            step_exportnameddeclaration(node, stack, budget, allocate, visit)
        }
        Frame::ExportSpecifier(node) => step_exportspecifier(node, stack, budget, allocate, visit),
        Frame::Expression(node) => step_expression(node, stack, budget, allocate, visit),
        Frame::ExpressionStatement(node) => {
            step_expressionstatement(node, stack, budget, allocate, visit)
        }
        Frame::ForInStatement(node) => step_forinstatement(node, stack, budget, allocate, visit),
        Frame::ForInit(node) => step_forinit(node, stack, budget, allocate, visit),
        Frame::ForOfStatement(node) => step_forofstatement(node, stack, budget, allocate, visit),
        Frame::ForStatement(node) => step_forstatement(node, stack, budget, allocate, visit),
        Frame::FunctionDeclaration(node) => {
            step_functiondeclaration(node, stack, budget, allocate, visit)
        }
        Frame::FunctionExpression(node) => {
            step_functionexpression(node, stack, budget, allocate, visit)
        }
        Frame::FunctionParam(node) => step_functionparam(node, stack, budget, allocate, visit),
        Frame::Identifier(node) => step_identifier(node, stack, budget, allocate, visit),
        Frame::IfStatement(node) => step_ifstatement(node, stack, budget, allocate, visit),
        Frame::ImportDeclaration(node) => {
            step_importdeclaration(node, stack, budget, allocate, visit)
        }
        Frame::ImportExpression(node) => {
            step_importexpression(node, stack, budget, allocate, visit)
        }
        Frame::ImportMeta(node) => step_importmeta(node, stack, budget, allocate, visit),
        Frame::ImportSpecifier(node) => step_importspecifier(node, stack, budget, allocate, visit),
        Frame::LabeledStatement(node) => {
            step_labeledstatement(node, stack, budget, allocate, visit)
        }
        Frame::LogicalExpression(node) => {
            step_logicalexpression(node, stack, budget, allocate, visit)
        }
        Frame::MemberExpression(node) => {
            step_memberexpression(node, stack, budget, allocate, visit)
        }
        Frame::MethodDefinition(node) => {
            step_methoddefinition(node, stack, budget, allocate, visit)
        }
        Frame::NewExpression(node) => step_newexpression(node, stack, budget, allocate, visit),
        Frame::NewTarget(node) => step_newtarget(node, stack, budget, allocate, visit),
        Frame::NullLiteral(node) => step_nullliteral(node, stack, budget, allocate, visit),
        Frame::NumericLiteral(node) => step_numericliteral(node, stack, budget, allocate, visit),
        Frame::ObjectExpression(node) => {
            step_objectexpression(node, stack, budget, allocate, visit)
        }
        Frame::ObjectMember(node) => step_objectmember(node, stack, budget, allocate, visit),
        Frame::OptionalCallExpression(node) => {
            step_optionalcallexpression(node, stack, budget, allocate, visit)
        }
        Frame::OptionalMemberExpression(node) => {
            step_optionalmemberexpression(node, stack, budget, allocate, visit)
        }
        Frame::PrivateName(node) => step_privatename(node, stack, budget, allocate, visit),
        Frame::Property(node) => step_property(node, stack, budget, allocate, visit),
        Frame::PropertyDefinition(node) => {
            step_propertydefinition(node, stack, budget, allocate, visit)
        }
        Frame::PropertyKey(node) => step_propertykey(node, stack, budget, allocate, visit),
        Frame::RegExpLiteral(node) => step_regexpliteral(node, stack, budget, allocate, visit),
        Frame::RestElement(node) => step_restelement(node, stack, budget, allocate, visit),
        Frame::ReturnStatement(node) => step_returnstatement(node, stack, budget, allocate, visit),
        Frame::SequenceExpression(node) => {
            step_sequenceexpression(node, stack, budget, allocate, visit)
        }
        Frame::SpreadElement(node) => step_spreadelement(node, stack, budget, allocate, visit),
        Frame::Statement(node) => step_statement(node, stack, budget, allocate, visit),
        Frame::StringLiteral(node) => step_stringliteral(node, stack, budget, allocate, visit),
        Frame::Super(node) => step_super(node, stack, budget, allocate, visit),
        Frame::SwitchCase(node) => step_switchcase(node, stack, budget, allocate, visit),
        Frame::SwitchStatement(node) => step_switchstatement(node, stack, budget, allocate, visit),
        Frame::TaggedStatement(node) => step_taggedstatement(node, stack, budget, allocate, visit),
        Frame::TaggedTemplateExpression(node) => {
            step_taggedtemplateexpression(node, stack, budget, allocate, visit)
        }
        Frame::TemplateElement(node) => step_templateelement(node, stack, budget, allocate, visit),
        Frame::TemplateLiteral(node) => step_templateliteral(node, stack, budget, allocate, visit),
        Frame::ThisExpression(node) => step_thisexpression(node, stack, budget, allocate, visit),
        Frame::ThrowStatement(node) => step_throwstatement(node, stack, budget, allocate, visit),
        Frame::TryStatement(node) => step_trystatement(node, stack, budget, allocate, visit),
        Frame::UnaryExpression(node) => step_unaryexpression(node, stack, budget, allocate, visit),
        Frame::UndefinedLiteral(node) => {
            step_undefinedliteral(node, stack, budget, allocate, visit)
        }
        Frame::UpdateExpression(node) => {
            step_updateexpression(node, stack, budget, allocate, visit)
        }
        Frame::VariableDeclaration(node) => {
            step_variabledeclaration(node, stack, budget, allocate, visit)
        }
        Frame::VariableDeclarator(node) => {
            step_variabledeclarator(node, stack, budget, allocate, visit)
        }
        Frame::WhileStatement(node) => step_whilestatement(node, stack, budget, allocate, visit),
        Frame::WithStatement(node) => step_withstatement(node, stack, budget, allocate, visit),
        Frame::YieldExpression(node) => step_yieldexpression(node, stack, budget, allocate, visit),
        Frame::ClassMemberList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::ClassMemberList(items), budget, allocate)?;
                stack.push(Frame::ClassMember(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::ExportSpecifierList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::ExportSpecifierList(items), budget, allocate)?;
                stack.push(Frame::ExportSpecifier(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::ExpressionList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::ExpressionList(items), budget, allocate)?;
                stack.push(Frame::Expression(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::FunctionParamList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::FunctionParamList(items), budget, allocate)?;
                stack.push(Frame::FunctionParam(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::ImportSpecifierList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::ImportSpecifierList(items), budget, allocate)?;
                stack.push(Frame::ImportSpecifier(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::ObjectMemberList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::ObjectMemberList(items), budget, allocate)?;
                stack.push(Frame::ObjectMember(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::OptionalExpressionList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::OptionalExpressionList(items), budget, allocate)?;
                budget.take(1)?; // Every hole/present source cell.
                if let Some(item) = item {
                    stack.push(Frame::Expression(item), budget, allocate)?;
                }
            }
            Ok(())
        }
        Frame::StatementList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::StatementList(items), budget, allocate)?;
                stack.push(Frame::Statement(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::SwitchCaseList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::SwitchCaseList(items), budget, allocate)?;
                stack.push(Frame::SwitchCase(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::TemplateElementList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::TemplateElementList(items), budget, allocate)?;
                stack.push(Frame::TemplateElement(item), budget, allocate)?;
            }
            Ok(())
        }
        Frame::VariableDeclaratorList(mut items) => {
            budget.take(1)?; // Includes terminal iterator advance.
            if let Some(item) = items.next() {
                stack.push(Frame::VariableDeclaratorList(items), budget, allocate)?;
                stack.push(Frame::VariableDeclarator(item), budget, allocate)?;
            }
            Ok(())
        }
    }
}

fn step_arrayexpression<'a>(
    node: &'a ArrayExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ArrayExpression { cv, elements } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(
        Frame::OptionalExpressionList(elements.iter()),
        budget,
        allocate,
    )?;
    Ok(())
}

fn step_arrowbody<'a>(
    node: &'a ArrowBody,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        ArrowBody::Expression(node) => {
            stack.push(Frame::Expression(node), budget, allocate)?;
        }
        ArrowBody::Block(node) => {
            stack.push(Frame::BlockStatement(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_arrowfunctionexpression<'a>(
    node: &'a ArrowFunctionExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ArrowFunctionExpression {
        cv,
        params,
        body,
        is_async: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ArrowBody(body), budget, allocate)?;
    stack.push(Frame::FunctionParamList(params.iter()), budget, allocate)?;
    Ok(())
}

fn step_assignmentexpression<'a>(
    node: &'a AssignmentExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let AssignmentExpression {
        cv,
        operator: _,
        left,
        right,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(right), budget, allocate)?;
    stack.push(Frame::AssignmentTarget(left), budget, allocate)?;
    Ok(())
}

fn step_assignmentpattern<'a>(
    node: &'a AssignmentPattern,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let AssignmentPattern { cv, left, right } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(right), budget, allocate)?;
    stack.push(Frame::Identifier(left), budget, allocate)?;
    Ok(())
}

fn step_assignmenttarget<'a>(
    node: &'a AssignmentTarget,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        AssignmentTarget::Identifier(node) => {
            stack.push(Frame::Identifier(node), budget, allocate)?;
        }
        AssignmentTarget::MemberExpression(node) => {
            stack.push(Frame::MemberExpression(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_awaitexpression<'a>(
    node: &'a AwaitExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let AwaitExpression { cv, argument } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(argument), budget, allocate)?;
    Ok(())
}

fn step_bigintliteral<'a>(
    node: &'a BigIntLiteral,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let BigIntLiteral {
        cv,
        value: _,
        raw: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_binaryexpression<'a>(
    node: &'a BinaryExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let BinaryExpression {
        cv,
        operator: _,
        left,
        right,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(right), budget, allocate)?;
    stack.push(Frame::Expression(left), budget, allocate)?;
    Ok(())
}

fn step_bindingtarget<'a>(
    node: &'a BindingTarget,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        BindingTarget::Identifier(node) => {
            stack.push(Frame::Identifier(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_blockstatement<'a>(
    node: &'a BlockStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let BlockStatement { cv, body } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::StatementList(body.iter()), budget, allocate)?;
    Ok(())
}

fn step_booleanliteral<'a>(
    node: &'a BooleanLiteral,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let BooleanLiteral { cv, value: _ } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_breakstatement<'a>(
    node: &'a BreakStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let BreakStatement { cv, label } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = label {
        stack.push(Frame::Identifier(value), budget, allocate)?;
    }
    Ok(())
}

fn step_callexpression<'a>(
    node: &'a CallExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let CallExpression {
        cv,
        callee,
        arguments,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ExpressionList(arguments.iter()), budget, allocate)?;
    stack.push(Frame::Expression(callee), budget, allocate)?;
    Ok(())
}

fn step_catchclause<'a>(
    node: &'a CatchClause,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let CatchClause { cv, param, body } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::BlockStatement(body), budget, allocate)?;
    budget.take(1)?;
    if let Some(value) = param {
        stack.push(Frame::Identifier(value), budget, allocate)?;
    }
    Ok(())
}

fn step_chainexpression<'a>(
    node: &'a ChainExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ChainExpression { cv, expression } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(expression), budget, allocate)?;
    Ok(())
}

fn step_classdeclaration<'a>(
    node: &'a ClassDeclaration,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ClassDeclaration {
        cv,
        id,
        super_class,
        body,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ClassMemberList(body.iter()), budget, allocate)?;
    budget.take(1)?;
    if let Some(value) = super_class {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    stack.push(Frame::Identifier(id), budget, allocate)?;
    Ok(())
}

fn step_classexpression<'a>(
    node: &'a ClassExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ClassExpression {
        cv,
        id,
        super_class,
        body,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ClassMemberList(body.iter()), budget, allocate)?;
    budget.take(1)?;
    if let Some(value) = super_class {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    budget.take(1)?;
    if let Some(value) = id {
        stack.push(Frame::Identifier(value), budget, allocate)?;
    }
    Ok(())
}

fn step_classmember<'a>(
    node: &'a ClassMember,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        ClassMember::Method(node) => {
            stack.push(Frame::MethodDefinition(node), budget, allocate)?;
        }
        ClassMember::Field(node) => {
            stack.push(Frame::PropertyDefinition(node), budget, allocate)?;
        }
        ClassMember::StaticBlock(node) => {
            stack.push(Frame::BlockStatement(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_conditionalexpression<'a>(
    node: &'a ConditionalExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ConditionalExpression {
        cv,
        test,
        consequent,
        alternate,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(alternate), budget, allocate)?;
    stack.push(Frame::Expression(consequent), budget, allocate)?;
    stack.push(Frame::Expression(test), budget, allocate)?;
    Ok(())
}

fn step_continuestatement<'a>(
    node: &'a ContinueStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ContinueStatement { cv, label } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = label {
        stack.push(Frame::Identifier(value), budget, allocate)?;
    }
    Ok(())
}

fn step_debuggerstatement<'a>(
    node: &'a DebuggerStatement,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let DebuggerStatement { cv } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_declaration<'a>(
    node: &'a Declaration,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        Declaration::VariableDeclaration(node) => {
            stack.push(Frame::VariableDeclaration(node), budget, allocate)?;
        }
        Declaration::FunctionDeclaration(node) => {
            stack.push(Frame::FunctionDeclaration(node), budget, allocate)?;
        }
        Declaration::ClassDeclaration(node) => {
            stack.push(Frame::ClassDeclaration(node), budget, allocate)?;
        }
        Declaration::ImportDeclaration(node) => {
            stack.push(Frame::ImportDeclaration(node), budget, allocate)?;
        }
        Declaration::ExportNamedDeclaration(node) => {
            stack.push(Frame::ExportNamedDeclaration(node), budget, allocate)?;
        }
        Declaration::ExportDefaultDeclaration(node) => {
            stack.push(Frame::ExportDefaultDeclaration(node), budget, allocate)?;
        }
        Declaration::ExportAllDeclaration(node) => {
            stack.push(Frame::ExportAllDeclaration(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_dowhilestatement<'a>(
    node: &'a DoWhileStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let DoWhileStatement { cv, body, test } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(test), budget, allocate)?;
    stack.push(Frame::Statement(body), budget, allocate)?;
    Ok(())
}

fn step_emptystatement<'a>(
    node: &'a EmptyStatement,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let EmptyStatement { cv } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_exportalldeclaration<'a>(
    node: &'a ExportAllDeclaration,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ExportAllDeclaration {
        cv,
        exported,
        source,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::StringLiteral(source), budget, allocate)?;
    budget.take(1)?;
    if let Some(value) = exported {
        stack.push(Frame::Identifier(value), budget, allocate)?;
    }
    Ok(())
}

fn step_exportdefaultdeclaration<'a>(
    node: &'a ExportDefaultDeclaration,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ExportDefaultDeclaration { cv, declaration } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ExportDefaultKind(declaration), budget, allocate)?;
    Ok(())
}

fn step_exportdefaultkind<'a>(
    node: &'a ExportDefaultKind,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        ExportDefaultKind::Expression(node) => {
            stack.push(Frame::Expression(node), budget, allocate)?;
        }
        ExportDefaultKind::FunctionDeclaration(node) => {
            stack.push(Frame::FunctionDeclaration(node), budget, allocate)?;
        }
        ExportDefaultKind::ClassDeclaration(node) => {
            stack.push(Frame::ClassDeclaration(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_exportnameddeclaration<'a>(
    node: &'a ExportNamedDeclaration,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ExportNamedDeclaration {
        cv,
        declaration,
        specifiers,
        source,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = source {
        stack.push(Frame::StringLiteral(value), budget, allocate)?;
    }
    stack.push(
        Frame::ExportSpecifierList(specifiers.iter()),
        budget,
        allocate,
    )?;
    budget.take(1)?;
    if let Some(value) = declaration {
        stack.push(Frame::Declaration(value), budget, allocate)?;
    }
    Ok(())
}

fn step_exportspecifier<'a>(
    node: &'a ExportSpecifier,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ExportSpecifier { local, exported } = node;
    stack.push(Frame::Identifier(exported), budget, allocate)?;
    stack.push(Frame::Identifier(local), budget, allocate)?;
    Ok(())
}

fn step_expression<'a>(
    node: &'a Expression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        Expression::Identifier(node) => {
            stack.push(Frame::Identifier(node), budget, allocate)?;
        }
        Expression::NumericLiteral(node) => {
            stack.push(Frame::NumericLiteral(node), budget, allocate)?;
        }
        Expression::StringLiteral(node) => {
            stack.push(Frame::StringLiteral(node), budget, allocate)?;
        }
        Expression::BooleanLiteral(node) => {
            stack.push(Frame::BooleanLiteral(node), budget, allocate)?;
        }
        Expression::NullLiteral(node) => {
            stack.push(Frame::NullLiteral(node), budget, allocate)?;
        }
        Expression::BigIntLiteral(node) => {
            stack.push(Frame::BigIntLiteral(node), budget, allocate)?;
        }
        Expression::UndefinedLiteral(node) => {
            stack.push(Frame::UndefinedLiteral(node), budget, allocate)?;
        }
        Expression::RegExpLiteral(node) => {
            stack.push(Frame::RegExpLiteral(node), budget, allocate)?;
        }
        Expression::BinaryExpression(node) => {
            stack.push(Frame::BinaryExpression(node), budget, allocate)?;
        }
        Expression::LogicalExpression(node) => {
            stack.push(Frame::LogicalExpression(node), budget, allocate)?;
        }
        Expression::UnaryExpression(node) => {
            stack.push(Frame::UnaryExpression(node), budget, allocate)?;
        }
        Expression::AssignmentExpression(node) => {
            stack.push(Frame::AssignmentExpression(node), budget, allocate)?;
        }
        Expression::ConditionalExpression(node) => {
            stack.push(Frame::ConditionalExpression(node), budget, allocate)?;
        }
        Expression::CallExpression(node) => {
            stack.push(Frame::CallExpression(node), budget, allocate)?;
        }
        Expression::MemberExpression(node) => {
            stack.push(Frame::MemberExpression(node), budget, allocate)?;
        }
        Expression::OptionalMemberExpression(node) => {
            stack.push(Frame::OptionalMemberExpression(node), budget, allocate)?;
        }
        Expression::OptionalCallExpression(node) => {
            stack.push(Frame::OptionalCallExpression(node), budget, allocate)?;
        }
        Expression::ChainExpression(node) => {
            stack.push(Frame::ChainExpression(node), budget, allocate)?;
        }
        Expression::ArrayExpression(node) => {
            stack.push(Frame::ArrayExpression(node), budget, allocate)?;
        }
        Expression::ObjectExpression(node) => {
            stack.push(Frame::ObjectExpression(node), budget, allocate)?;
        }
        Expression::FunctionExpression(node) => {
            stack.push(Frame::FunctionExpression(node), budget, allocate)?;
        }
        Expression::ArrowFunctionExpression(node) => {
            stack.push(Frame::ArrowFunctionExpression(node), budget, allocate)?;
        }
        Expression::ClassExpression(node) => {
            stack.push(Frame::ClassExpression(node), budget, allocate)?;
        }
        Expression::TemplateLiteral(node) => {
            stack.push(Frame::TemplateLiteral(node), budget, allocate)?;
        }
        Expression::UpdateExpression(node) => {
            stack.push(Frame::UpdateExpression(node), budget, allocate)?;
        }
        Expression::NewExpression(node) => {
            stack.push(Frame::NewExpression(node), budget, allocate)?;
        }
        Expression::SequenceExpression(node) => {
            stack.push(Frame::SequenceExpression(node), budget, allocate)?;
        }
        Expression::TaggedTemplateExpression(node) => {
            stack.push(Frame::TaggedTemplateExpression(node), budget, allocate)?;
        }
        Expression::SpreadElement(node) => {
            stack.push(Frame::SpreadElement(node), budget, allocate)?;
        }
        Expression::YieldExpression(node) => {
            stack.push(Frame::YieldExpression(node), budget, allocate)?;
        }
        Expression::AwaitExpression(node) => {
            stack.push(Frame::AwaitExpression(node), budget, allocate)?;
        }
        Expression::ThisExpression(node) => {
            stack.push(Frame::ThisExpression(node), budget, allocate)?;
        }
        Expression::Super(node) => {
            stack.push(Frame::Super(node), budget, allocate)?;
        }
        Expression::NewTarget(node) => {
            stack.push(Frame::NewTarget(node), budget, allocate)?;
        }
        Expression::ImportMeta(node) => {
            stack.push(Frame::ImportMeta(node), budget, allocate)?;
        }
        Expression::ImportExpression(node) => {
            stack.push(Frame::ImportExpression(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_expressionstatement<'a>(
    node: &'a ExpressionStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ExpressionStatement { cv, expression } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(expression), budget, allocate)?;
    Ok(())
}

fn step_forinstatement<'a>(
    node: &'a ForInStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ForInStatement {
        cv,
        left,
        right,
        body,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Statement(body), budget, allocate)?;
    stack.push(Frame::Expression(right), budget, allocate)?;
    stack.push(Frame::ForInit(left), budget, allocate)?;
    Ok(())
}

fn step_forinit<'a>(
    node: &'a ForInit,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        ForInit::VariableDeclaration(node) => {
            stack.push(Frame::VariableDeclaration(node), budget, allocate)?;
        }
        ForInit::Expression(node) => {
            stack.push(Frame::Expression(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_forofstatement<'a>(
    node: &'a ForOfStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ForOfStatement {
        cv,
        left,
        right,
        body,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Statement(body), budget, allocate)?;
    stack.push(Frame::Expression(right), budget, allocate)?;
    stack.push(Frame::ForInit(left), budget, allocate)?;
    Ok(())
}

fn step_forstatement<'a>(
    node: &'a ForStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ForStatement {
        cv,
        init,
        test,
        update,
        body,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Statement(body), budget, allocate)?;
    budget.take(1)?;
    if let Some(value) = update {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    budget.take(1)?;
    if let Some(value) = test {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    budget.take(1)?;
    if let Some(value) = init {
        stack.push(Frame::ForInit(value), budget, allocate)?;
    }
    Ok(())
}

fn step_functiondeclaration<'a>(
    node: &'a FunctionDeclaration,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let FunctionDeclaration {
        cv,
        id,
        params,
        body,
        generator: _,
        is_async: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::BlockStatement(body), budget, allocate)?;
    stack.push(Frame::FunctionParamList(params.iter()), budget, allocate)?;
    stack.push(Frame::Identifier(id), budget, allocate)?;
    Ok(())
}

fn step_functionexpression<'a>(
    node: &'a FunctionExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let FunctionExpression {
        cv,
        id,
        params,
        body,
        generator: _,
        is_async: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::BlockStatement(body), budget, allocate)?;
    stack.push(Frame::FunctionParamList(params.iter()), budget, allocate)?;
    budget.take(1)?;
    if let Some(value) = id {
        stack.push(Frame::Identifier(value), budget, allocate)?;
    }
    Ok(())
}

fn step_functionparam<'a>(
    node: &'a FunctionParam,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        FunctionParam::Identifier(node) => {
            stack.push(Frame::Identifier(node), budget, allocate)?;
        }
        FunctionParam::RestElement(node) => {
            stack.push(Frame::RestElement(node), budget, allocate)?;
        }
        FunctionParam::AssignmentPattern(node) => {
            stack.push(Frame::AssignmentPattern(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_identifier<'a>(
    node: &'a Identifier,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let Identifier { cv, name: _ } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_ifstatement<'a>(
    node: &'a IfStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let IfStatement {
        cv,
        test,
        consequent,
        alternate,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = alternate {
        stack.push(Frame::Statement(value), budget, allocate)?;
    }
    stack.push(Frame::Statement(consequent), budget, allocate)?;
    stack.push(Frame::Expression(test), budget, allocate)?;
    Ok(())
}

fn step_importdeclaration<'a>(
    node: &'a ImportDeclaration,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ImportDeclaration {
        cv,
        specifiers,
        source,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::StringLiteral(source), budget, allocate)?;
    stack.push(
        Frame::ImportSpecifierList(specifiers.iter()),
        budget,
        allocate,
    )?;
    Ok(())
}

fn step_importexpression<'a>(
    node: &'a ImportExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ImportExpression { cv, source } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(source), budget, allocate)?;
    Ok(())
}

fn step_importmeta<'a>(
    node: &'a ImportMeta,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ImportMeta { cv } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_importspecifier<'a>(
    node: &'a ImportSpecifier,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        ImportSpecifier::Default(node) => {
            stack.push(Frame::Identifier(node), budget, allocate)?;
        }
        ImportSpecifier::Namespace(node) => {
            stack.push(Frame::Identifier(node), budget, allocate)?;
        }
        ImportSpecifier::Named { imported, local } => {
            stack.push(Frame::Identifier(local), budget, allocate)?;
            stack.push(Frame::Identifier(imported), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_labeledstatement<'a>(
    node: &'a LabeledStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let LabeledStatement { cv, label, body } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Statement(body), budget, allocate)?;
    stack.push(Frame::Identifier(label), budget, allocate)?;
    Ok(())
}

fn step_logicalexpression<'a>(
    node: &'a LogicalExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let LogicalExpression {
        cv,
        operator: _,
        left,
        right,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(right), budget, allocate)?;
    stack.push(Frame::Expression(left), budget, allocate)?;
    Ok(())
}

fn step_memberexpression<'a>(
    node: &'a MemberExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let MemberExpression {
        cv,
        object,
        property,
        computed: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(property), budget, allocate)?;
    stack.push(Frame::Expression(object), budget, allocate)?;
    Ok(())
}

fn step_methoddefinition<'a>(
    node: &'a MethodDefinition,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let MethodDefinition {
        cv,
        key,
        kind: _,
        value,
        computed: _,
        is_static: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::FunctionExpression(value), budget, allocate)?;
    stack.push(Frame::PropertyKey(key), budget, allocate)?;
    Ok(())
}

fn step_newexpression<'a>(
    node: &'a NewExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let NewExpression {
        cv,
        callee,
        arguments,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ExpressionList(arguments.iter()), budget, allocate)?;
    stack.push(Frame::Expression(callee), budget, allocate)?;
    Ok(())
}

fn step_newtarget<'a>(
    node: &'a NewTarget,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let NewTarget { cv } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_nullliteral<'a>(
    node: &'a NullLiteral,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let NullLiteral { cv } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_numericliteral<'a>(
    node: &'a NumericLiteral,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let NumericLiteral {
        cv,
        value: _,
        raw: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_objectexpression<'a>(
    node: &'a ObjectExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ObjectExpression { cv, properties } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ObjectMemberList(properties.iter()), budget, allocate)?;
    Ok(())
}

fn step_objectmember<'a>(
    node: &'a ObjectMember,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        ObjectMember::Property(node) => {
            stack.push(Frame::Property(node), budget, allocate)?;
        }
        ObjectMember::Spread(node) => {
            stack.push(Frame::SpreadElement(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_optionalcallexpression<'a>(
    node: &'a OptionalCallExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let OptionalCallExpression {
        cv,
        callee,
        arguments,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ExpressionList(arguments.iter()), budget, allocate)?;
    stack.push(Frame::Expression(callee), budget, allocate)?;
    Ok(())
}

fn step_optionalmemberexpression<'a>(
    node: &'a OptionalMemberExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let OptionalMemberExpression {
        cv,
        object,
        property,
        computed: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(property), budget, allocate)?;
    stack.push(Frame::Expression(object), budget, allocate)?;
    Ok(())
}

fn step_privatename<'a>(
    node: &'a PrivateName,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let PrivateName { cv, name: _ } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_property<'a>(
    node: &'a Property,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let Property {
        cv,
        kind: _,
        key,
        value,
        computed: _,
        shorthand: _,
        method: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(value), budget, allocate)?;
    stack.push(Frame::PropertyKey(key), budget, allocate)?;
    Ok(())
}

fn step_propertydefinition<'a>(
    node: &'a PropertyDefinition,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let PropertyDefinition {
        cv,
        key,
        value,
        computed: _,
        is_static: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = value {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    stack.push(Frame::PropertyKey(key), budget, allocate)?;
    Ok(())
}

fn step_propertykey<'a>(
    node: &'a PropertyKey,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        PropertyKey::Identifier(node) => {
            stack.push(Frame::Identifier(node), budget, allocate)?;
        }
        PropertyKey::PrivateName(node) => {
            stack.push(Frame::PrivateName(node), budget, allocate)?;
        }
        PropertyKey::StringLiteral(node) => {
            stack.push(Frame::StringLiteral(node), budget, allocate)?;
        }
        PropertyKey::NumericLiteral(node) => {
            stack.push(Frame::NumericLiteral(node), budget, allocate)?;
        }
        PropertyKey::Expression(node) => {
            stack.push(Frame::Expression(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_regexpliteral<'a>(
    node: &'a RegExpLiteral,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let RegExpLiteral {
        cv,
        pattern: _,
        flags: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_restelement<'a>(
    node: &'a RestElement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let RestElement { cv, argument } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Identifier(argument), budget, allocate)?;
    Ok(())
}

fn step_returnstatement<'a>(
    node: &'a ReturnStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ReturnStatement { cv, argument } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = argument {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    Ok(())
}

fn step_sequenceexpression<'a>(
    node: &'a SequenceExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let SequenceExpression { cv, expressions } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ExpressionList(expressions.iter()), budget, allocate)?;
    Ok(())
}

fn step_spreadelement<'a>(
    node: &'a SpreadElement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let SpreadElement { cv, argument } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(argument), budget, allocate)?;
    Ok(())
}

fn step_statement<'a>(
    node: &'a Statement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        Statement::Tagged(node) => {
            stack.push(Frame::TaggedStatement(node), budget, allocate)?;
        }
        Statement::Declaration(node) => {
            stack.push(Frame::Declaration(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_stringliteral<'a>(
    node: &'a StringLiteral,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let StringLiteral {
        cv,
        value: _,
        raw: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_super<'a>(
    node: &'a Super,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let Super { cv } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_switchcase<'a>(
    node: &'a SwitchCase,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let SwitchCase {
        cv,
        test,
        consequent,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::StatementList(consequent.iter()), budget, allocate)?;
    budget.take(1)?;
    if let Some(value) = test {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    Ok(())
}

fn step_switchstatement<'a>(
    node: &'a SwitchStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let SwitchStatement {
        cv,
        discriminant,
        cases,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::SwitchCaseList(cases.iter()), budget, allocate)?;
    stack.push(Frame::Expression(discriminant), budget, allocate)?;
    Ok(())
}

fn step_taggedstatement<'a>(
    node: &'a TaggedStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    _visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    match node {
        TaggedStatement::ExpressionStatement(node) => {
            stack.push(Frame::ExpressionStatement(node), budget, allocate)?;
        }
        TaggedStatement::BlockStatement(node) => {
            stack.push(Frame::BlockStatement(node), budget, allocate)?;
        }
        TaggedStatement::IfStatement(node) => {
            stack.push(Frame::IfStatement(node), budget, allocate)?;
        }
        TaggedStatement::WhileStatement(node) => {
            stack.push(Frame::WhileStatement(node), budget, allocate)?;
        }
        TaggedStatement::DoWhileStatement(node) => {
            stack.push(Frame::DoWhileStatement(node), budget, allocate)?;
        }
        TaggedStatement::ForStatement(node) => {
            stack.push(Frame::ForStatement(node), budget, allocate)?;
        }
        TaggedStatement::ForInStatement(node) => {
            stack.push(Frame::ForInStatement(node), budget, allocate)?;
        }
        TaggedStatement::ForOfStatement(node) => {
            stack.push(Frame::ForOfStatement(node), budget, allocate)?;
        }
        TaggedStatement::ReturnStatement(node) => {
            stack.push(Frame::ReturnStatement(node), budget, allocate)?;
        }
        TaggedStatement::BreakStatement(node) => {
            stack.push(Frame::BreakStatement(node), budget, allocate)?;
        }
        TaggedStatement::ContinueStatement(node) => {
            stack.push(Frame::ContinueStatement(node), budget, allocate)?;
        }
        TaggedStatement::LabeledStatement(node) => {
            stack.push(Frame::LabeledStatement(node), budget, allocate)?;
        }
        TaggedStatement::ThrowStatement(node) => {
            stack.push(Frame::ThrowStatement(node), budget, allocate)?;
        }
        TaggedStatement::SwitchStatement(node) => {
            stack.push(Frame::SwitchStatement(node), budget, allocate)?;
        }
        TaggedStatement::TryStatement(node) => {
            stack.push(Frame::TryStatement(node), budget, allocate)?;
        }
        TaggedStatement::EmptyStatement(node) => {
            stack.push(Frame::EmptyStatement(node), budget, allocate)?;
        }
        TaggedStatement::DebuggerStatement(node) => {
            stack.push(Frame::DebuggerStatement(node), budget, allocate)?;
        }
        TaggedStatement::WithStatement(node) => {
            stack.push(Frame::WithStatement(node), budget, allocate)?;
        }
    }
    Ok(())
}

fn step_taggedtemplateexpression<'a>(
    node: &'a TaggedTemplateExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let TaggedTemplateExpression { cv, tag, quasi } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::TemplateLiteral(quasi), budget, allocate)?;
    stack.push(Frame::Expression(tag), budget, allocate)?;
    Ok(())
}

fn step_templateelement<'a>(
    node: &'a TemplateElement,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let TemplateElement {
        cv,
        raw: _,
        cooked: _,
        tail: _,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_templateliteral<'a>(
    node: &'a TemplateLiteral,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let TemplateLiteral {
        cv,
        quasis,
        expressions,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::ExpressionList(expressions.iter()), budget, allocate)?;
    stack.push(Frame::TemplateElementList(quasis.iter()), budget, allocate)?;
    Ok(())
}

fn step_thisexpression<'a>(
    node: &'a ThisExpression,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ThisExpression { cv } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_throwstatement<'a>(
    node: &'a ThrowStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let ThrowStatement { cv, argument } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(argument), budget, allocate)?;
    Ok(())
}

fn step_trystatement<'a>(
    node: &'a TryStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let TryStatement {
        cv,
        block,
        handler,
        finalizer,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = finalizer {
        stack.push(Frame::BlockStatement(value), budget, allocate)?;
    }
    budget.take(1)?;
    if let Some(value) = handler {
        stack.push(Frame::CatchClause(value), budget, allocate)?;
    }
    stack.push(Frame::BlockStatement(block), budget, allocate)?;
    Ok(())
}

fn step_unaryexpression<'a>(
    node: &'a UnaryExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let UnaryExpression {
        cv,
        operator: _,
        prefix: _,
        argument,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(argument), budget, allocate)?;
    Ok(())
}

fn step_undefinedliteral<'a>(
    node: &'a UndefinedLiteral,
    _stack: &mut Stack<'a>,
    budget: &mut Budget,
    _allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let UndefinedLiteral { cv } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    Ok(())
}

fn step_updateexpression<'a>(
    node: &'a UpdateExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let UpdateExpression {
        cv,
        operator: _,
        prefix: _,
        argument,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Expression(argument), budget, allocate)?;
    Ok(())
}

fn step_variabledeclaration<'a>(
    node: &'a VariableDeclaration,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let VariableDeclaration {
        cv,
        kind: _,
        declarations,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(
        Frame::VariableDeclaratorList(declarations.iter()),
        budget,
        allocate,
    )?;
    Ok(())
}

fn step_variabledeclarator<'a>(
    node: &'a VariableDeclarator,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let VariableDeclarator { cv, id, init } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = init {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    stack.push(Frame::BindingTarget(id), budget, allocate)?;
    Ok(())
}

fn step_whilestatement<'a>(
    node: &'a WhileStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let WhileStatement { cv, test, body } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Statement(body), budget, allocate)?;
    stack.push(Frame::Expression(test), budget, allocate)?;
    Ok(())
}

fn step_withstatement<'a>(
    node: &'a WithStatement,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let WithStatement { cv, object, body } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    stack.push(Frame::Statement(body), budget, allocate)?;
    stack.push(Frame::Expression(object), budget, allocate)?;
    Ok(())
}

fn step_yieldexpression<'a>(
    node: &'a YieldExpression,
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    let YieldExpression {
        cv,
        delegate: _,
        argument,
    } = node;
    cv_slot(cv.as_deref(), budget, visit)?;
    budget.take(1)?;
    if let Some(value) = argument {
        stack.push(Frame::Expression(value), budget, allocate)?;
    }
    Ok(())
}

/// Preserve exact singleton behavior for C37/C38 callers and their oracles.
fn collect_with<'a, H, A>(
    prefix: [Option<&'a str>; 3],
    selected: &'a Expression,
    work: usize,
    bytes: usize,
    hash: H,
    allocate: A,
) -> Result<Vec<&'a str>, String>
where
    H: FnMut(&str) -> u64,
    A: FnMut(usize) -> Result<(), String>,
{
    collect_roots_with(prefix, &[selected], work, bytes, hash, allocate)
}

/// Each complete scan visits prefix/test, then absent-prefix consequent and
/// alternate. One Budget and retained stack cover all trees before replay.
fn scan_roots<'a>(
    prefix: [Option<&'a str>; 3],
    roots: &[&'a Expression],
    stack: &mut Stack<'a>,
    budget: &mut Budget,
    allocate: &mut Allocate<'_>,
    visit: &mut Visit<'a, '_>,
) -> Result<(), String> {
    for (index, root) in roots.iter().enumerate() {
        scan(
            if index == 0 { prefix } else { [None; 3] },
            root,
            stack,
            budget,
            allocate,
            visit,
        )?;
    }
    Ok(())
}

fn collect_roots_with<'a, H, A>(
    prefix: [Option<&'a str>; 3],
    roots: &[&'a Expression],
    work: usize,
    bytes: usize,
    mut hash: H,
    mut allocate: A,
) -> Result<Vec<&'a str>, String>
where
    H: FnMut(&str) -> u64,
    A: FnMut(usize) -> Result<(), String>,
{
    let mut budget = Budget { work, bytes };
    let mut stack = Stack::new();
    let mut count = 0usize;
    scan_roots(
        prefix,
        roots,
        &mut stack,
        &mut budget,
        &mut allocate,
        &mut |_, _| {
            count = count
                .checked_add(1)
                .ok_or("CV selected frontier ID count overflow")?;
            Ok(())
        },
    )?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let buckets = count
        .checked_mul(2)
        .and_then(usize::checked_next_power_of_two)
        .ok_or("CV selected frontier bucket overflow")?;
    let unique_bytes = count
        .checked_mul(std::mem::size_of::<&str>())
        .ok_or("CV selected frontier buffer overflow")?;
    let table_bytes = buckets
        .checked_mul(std::mem::size_of::<Option<&str>>())
        .ok_or("CV selected frontier buffer overflow")?;
    stack.fixed_bytes = unique_bytes
        .checked_add(table_bytes)
        .ok_or("CV selected frontier buffer overflow")?;
    budget.storage(stack.bytes().and_then(|n| n.checked_add(stack.fixed_bytes)))?;
    budget.take(
        buckets
            .checked_add(2)
            .ok_or("CV selected frontier initialization overflow")?,
    )?;
    let mut unique = Vec::new();
    allocate(unique_bytes)?;
    unique
        .try_reserve_exact(count)
        .map_err(|_| "CV selected frontier allocation refused")?;
    if unique.capacity() != count {
        return Err("CV selected frontier unexpected buffer capacity".into());
    }
    let mut table = Vec::<Option<&'a str>>::new();
    allocate(table_bytes)?;
    table
        .try_reserve_exact(buckets)
        .map_err(|_| "CV selected frontier allocation refused")?;
    if table.capacity() != buckets {
        return Err("CV selected frontier unexpected buffer capacity".into());
    }
    table.resize(buckets, None);
    let mask = buckets - 1;
    scan_roots(
        prefix,
        roots,
        &mut stack,
        &mut budget,
        &mut allocate,
        &mut |id, budget| {
            budget.take(4)?; // Hash setup/terminator/finish; bytes are separate.
            budget.take(id.len())?;
            let mut slot = (hash(id) as usize) & mask;
            for _ in 0..buckets {
                budget.take(1)?;
                if let Some(previous) = table[slot] {
                    budget.take(id.len())?;
                    budget.take(previous.len())?;
                    if previous == id {
                        return Ok(());
                    }
                } else {
                    table[slot] = Some(id);
                    unique.push(id); // count includes duplicates, so no growth.
                    return Ok(());
                }
                slot = slot.wrapping_add(1) & mask;
            }
            Err("CV selected frontier probe capacity exhausted".into())
        },
    )?;
    Ok(unique)
}

pub(super) fn collect<'a>(
    prefix: [Option<&'a str>; 3],
    selected: &'a Expression,
) -> Result<Vec<&'a str>, String> {
    let limits = GraphLimits::default();
    let random = RandomState::new();
    collect_with(
        prefix,
        selected,
        limits.max_work,
        limits.max_input_bytes,
        |id| random.hash_one(id),
        |_| Ok(()),
    )
}

#[cfg(test)]
#[path = "selected_subtree_frontier_tests.rs"]
mod tests;

/// Both equivalent arms and the evaluated test supply rewrite evidence, even
/// though only one branch executes. No synthetic AST or independent budgets.
pub(super) fn collect_equal_conditional<'a>(
    conditional: Option<&'a str>,
    test: &'a Expression,
    consequent: &'a Expression,
    alternate: &'a Expression,
) -> Result<Vec<&'a str>, String> {
    let limits = GraphLimits::default();
    let random = RandomState::new();
    collect_roots_with(
        [conditional, None, None],
        &[test, consequent, alternate],
        limits.max_work,
        limits.max_input_bytes,
        |id| random.hash_one(id),
        |_| Ok(()),
    )
}

#[cfg(test)]
#[path = "equal_conditional_frontier_tests.rs"]
mod equal_conditional_tests;
