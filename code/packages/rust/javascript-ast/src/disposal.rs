//! Iterative destruction for AST ownership boundaries.
//!
//! Normal derived Rust drop descends through every Box and vector element.
//! A pass can construct a deeper tree than its caller stack can destroy. Here
//! each work item relinquishes all AST children before ordinary drop runs.
//! Vector iterators retain the original allocation and yield one child at a
//! time; wide lists therefore need no copied sibling queue. Exhaustive field
//! patterns intentionally force new AST fields to receive a disposal decision.
use crate::statement::TaggedStatement;
use crate::*;

// Large node payloads live on the heap work stack; no recursive calls occur.
#[allow(clippy::large_enum_variant)]
enum Work {
    ArrayExpression(ArrayExpression),
    ArrowBody(ArrowBody),
    ArrowFunctionExpression(ArrowFunctionExpression),
    AssignmentExpression(AssignmentExpression),
    AssignmentPattern(AssignmentPattern),
    AssignmentTarget(AssignmentTarget),
    AwaitExpression(AwaitExpression),
    BigIntLiteral(BigIntLiteral),
    BinaryExpression(BinaryExpression),
    BindingTarget(BindingTarget),
    BlockStatement(BlockStatement),
    BooleanLiteral(BooleanLiteral),
    BreakStatement(BreakStatement),
    CallExpression(CallExpression),
    CatchClause(CatchClause),
    ChainExpression(ChainExpression),
    ClassDeclaration(ClassDeclaration),
    ClassExpression(ClassExpression),
    ClassMember(ClassMember),
    ConditionalExpression(ConditionalExpression),
    ContinueStatement(ContinueStatement),
    DebuggerStatement(DebuggerStatement),
    Declaration(Declaration),
    DoWhileStatement(DoWhileStatement),
    EmptyStatement(EmptyStatement),
    ExportAllDeclaration(ExportAllDeclaration),
    ExportDefaultDeclaration(ExportDefaultDeclaration),
    ExportDefaultKind(ExportDefaultKind),
    ExportNamedDeclaration(ExportNamedDeclaration),
    ExportSpecifier(ExportSpecifier),
    Expression(Expression),
    ExpressionStatement(ExpressionStatement),
    ForInStatement(ForInStatement),
    ForInit(ForInit),
    ForOfStatement(ForOfStatement),
    ForStatement(ForStatement),
    FunctionDeclaration(FunctionDeclaration),
    FunctionExpression(FunctionExpression),
    FunctionParam(FunctionParam),
    Identifier(Identifier),
    IfStatement(IfStatement),
    ImportDeclaration(ImportDeclaration),
    ImportExpression(ImportExpression),
    ImportMeta(ImportMeta),
    ImportSpecifier(ImportSpecifier),
    LabeledStatement(LabeledStatement),
    LogicalExpression(LogicalExpression),
    MemberExpression(MemberExpression),
    MethodDefinition(MethodDefinition),
    NewExpression(NewExpression),
    NewTarget(NewTarget),
    NullLiteral(NullLiteral),
    NumericLiteral(NumericLiteral),
    ObjectExpression(ObjectExpression),
    ObjectMember(ObjectMember),
    OptionalCallExpression(OptionalCallExpression),
    OptionalMemberExpression(OptionalMemberExpression),
    PrivateName(PrivateName),
    Program(Program),
    ProgramItem(ProgramItem),
    Property(Property),
    PropertyDefinition(PropertyDefinition),
    PropertyKey(PropertyKey),
    RegExpLiteral(RegExpLiteral),
    RestElement(RestElement),
    ReturnStatement(ReturnStatement),
    SequenceExpression(SequenceExpression),
    SpreadElement(SpreadElement),
    Statement(Statement),
    StringLiteral(StringLiteral),
    Super(Super),
    SwitchCase(SwitchCase),
    SwitchStatement(SwitchStatement),
    TaggedStatement(TaggedStatement),
    TaggedTemplateExpression(TaggedTemplateExpression),
    TemplateElement(TemplateElement),
    TemplateLiteral(TemplateLiteral),
    ThisExpression(ThisExpression),
    ThrowStatement(ThrowStatement),
    TryStatement(TryStatement),
    UnaryExpression(UnaryExpression),
    UndefinedLiteral(UndefinedLiteral),
    UpdateExpression(UpdateExpression),
    VariableDeclaration(VariableDeclaration),
    VariableDeclarator(VariableDeclarator),
    WhileStatement(WhileStatement),
    WithStatement(WithStatement),
    YieldExpression(YieldExpression),
    ClassMemberList(std::vec::IntoIter<ClassMember>),
    ExportSpecifierList(std::vec::IntoIter<ExportSpecifier>),
    ExpressionList(std::vec::IntoIter<Expression>),
    ExpressionPresentList(std::iter::Flatten<std::vec::IntoIter<Option<Expression>>>),
    FunctionParamList(std::vec::IntoIter<FunctionParam>),
    ImportSpecifierList(std::vec::IntoIter<ImportSpecifier>),
    ObjectMemberList(std::vec::IntoIter<ObjectMember>),
    ProgramItemList(std::vec::IntoIter<ProgramItem>),
    StatementList(std::vec::IntoIter<Statement>),
    SwitchCaseList(std::vec::IntoIter<SwitchCase>),
    TemplateElementList(std::vec::IntoIter<TemplateElement>),
    VariableDeclaratorList(std::vec::IntoIter<VariableDeclarator>),
}

/// Consume and destroy an owned program without recursive AST drop.
///
/// This is useful when rejecting a pass candidate or replacing an intermediate
/// program on a small caller stack. It moves children; it does not clone, leak,
/// serialize, or create a cleanup thread. The heap work stack grows with tree
/// depth, with one iterator per active list instead of one entry per sibling.
pub fn dispose_program(program: Program) {
    let mut pending = vec![Work::Program(program)];
    while let Some(item) = pending.pop() {
        match item {
            Work::ArrayExpression(node) => step_arrayexpression(node, &mut pending),
            Work::ArrowBody(node) => step_arrowbody(node, &mut pending),
            Work::ArrowFunctionExpression(node) => step_arrowfunctionexpression(node, &mut pending),
            Work::AssignmentExpression(node) => step_assignmentexpression(node, &mut pending),
            Work::AssignmentPattern(node) => step_assignmentpattern(node, &mut pending),
            Work::AssignmentTarget(node) => step_assignmenttarget(node, &mut pending),
            Work::AwaitExpression(node) => step_awaitexpression(node, &mut pending),
            Work::BigIntLiteral(node) => step_bigintliteral(node),
            Work::BinaryExpression(node) => step_binaryexpression(node, &mut pending),
            Work::BindingTarget(node) => step_bindingtarget(node, &mut pending),
            Work::BlockStatement(node) => step_blockstatement(node, &mut pending),
            Work::BooleanLiteral(node) => step_booleanliteral(node),
            Work::BreakStatement(node) => step_breakstatement(node, &mut pending),
            Work::CallExpression(node) => step_callexpression(node, &mut pending),
            Work::CatchClause(node) => step_catchclause(node, &mut pending),
            Work::ChainExpression(node) => step_chainexpression(node, &mut pending),
            Work::ClassDeclaration(node) => step_classdeclaration(node, &mut pending),
            Work::ClassExpression(node) => step_classexpression(node, &mut pending),
            Work::ClassMember(node) => step_classmember(node, &mut pending),
            Work::ConditionalExpression(node) => step_conditionalexpression(node, &mut pending),
            Work::ContinueStatement(node) => step_continuestatement(node, &mut pending),
            Work::DebuggerStatement(node) => step_debuggerstatement(node),
            Work::Declaration(node) => step_declaration(node, &mut pending),
            Work::DoWhileStatement(node) => step_dowhilestatement(node, &mut pending),
            Work::EmptyStatement(node) => step_emptystatement(node),
            Work::ExportAllDeclaration(node) => step_exportalldeclaration(node, &mut pending),
            Work::ExportDefaultDeclaration(node) => {
                step_exportdefaultdeclaration(node, &mut pending)
            }
            Work::ExportDefaultKind(node) => step_exportdefaultkind(node, &mut pending),
            Work::ExportNamedDeclaration(node) => step_exportnameddeclaration(node, &mut pending),
            Work::ExportSpecifier(node) => step_exportspecifier(node, &mut pending),
            Work::Expression(node) => step_expression(node, &mut pending),
            Work::ExpressionStatement(node) => step_expressionstatement(node, &mut pending),
            Work::ForInStatement(node) => step_forinstatement(node, &mut pending),
            Work::ForInit(node) => step_forinit(node, &mut pending),
            Work::ForOfStatement(node) => step_forofstatement(node, &mut pending),
            Work::ForStatement(node) => step_forstatement(node, &mut pending),
            Work::FunctionDeclaration(node) => step_functiondeclaration(node, &mut pending),
            Work::FunctionExpression(node) => step_functionexpression(node, &mut pending),
            Work::FunctionParam(node) => step_functionparam(node, &mut pending),
            Work::Identifier(node) => step_identifier(node),
            Work::IfStatement(node) => step_ifstatement(node, &mut pending),
            Work::ImportDeclaration(node) => step_importdeclaration(node, &mut pending),
            Work::ImportExpression(node) => step_importexpression(node, &mut pending),
            Work::ImportMeta(node) => step_importmeta(node),
            Work::ImportSpecifier(node) => step_importspecifier(node, &mut pending),
            Work::LabeledStatement(node) => step_labeledstatement(node, &mut pending),
            Work::LogicalExpression(node) => step_logicalexpression(node, &mut pending),
            Work::MemberExpression(node) => step_memberexpression(node, &mut pending),
            Work::MethodDefinition(node) => step_methoddefinition(node, &mut pending),
            Work::NewExpression(node) => step_newexpression(node, &mut pending),
            Work::NewTarget(node) => step_newtarget(node),
            Work::NullLiteral(node) => step_nullliteral(node),
            Work::NumericLiteral(node) => step_numericliteral(node),
            Work::ObjectExpression(node) => step_objectexpression(node, &mut pending),
            Work::ObjectMember(node) => step_objectmember(node, &mut pending),
            Work::OptionalCallExpression(node) => step_optionalcallexpression(node, &mut pending),
            Work::OptionalMemberExpression(node) => {
                step_optionalmemberexpression(node, &mut pending)
            }
            Work::PrivateName(node) => step_privatename(node),
            Work::Program(node) => step_program(node, &mut pending),
            Work::ProgramItem(node) => step_programitem(node, &mut pending),
            Work::Property(node) => step_property(node, &mut pending),
            Work::PropertyDefinition(node) => step_propertydefinition(node, &mut pending),
            Work::PropertyKey(node) => step_propertykey(node, &mut pending),
            Work::RegExpLiteral(node) => step_regexpliteral(node),
            Work::RestElement(node) => step_restelement(node, &mut pending),
            Work::ReturnStatement(node) => step_returnstatement(node, &mut pending),
            Work::SequenceExpression(node) => step_sequenceexpression(node, &mut pending),
            Work::SpreadElement(node) => step_spreadelement(node, &mut pending),
            Work::Statement(node) => step_statement(node, &mut pending),
            Work::StringLiteral(node) => step_stringliteral(node),
            Work::Super(node) => step_super(node),
            Work::SwitchCase(node) => step_switchcase(node, &mut pending),
            Work::SwitchStatement(node) => step_switchstatement(node, &mut pending),
            Work::TaggedStatement(node) => step_taggedstatement(node, &mut pending),
            Work::TaggedTemplateExpression(node) => {
                step_taggedtemplateexpression(node, &mut pending)
            }
            Work::TemplateElement(node) => step_templateelement(node),
            Work::TemplateLiteral(node) => step_templateliteral(node, &mut pending),
            Work::ThisExpression(node) => step_thisexpression(node),
            Work::ThrowStatement(node) => step_throwstatement(node, &mut pending),
            Work::TryStatement(node) => step_trystatement(node, &mut pending),
            Work::UnaryExpression(node) => step_unaryexpression(node, &mut pending),
            Work::UndefinedLiteral(node) => step_undefinedliteral(node),
            Work::UpdateExpression(node) => step_updateexpression(node, &mut pending),
            Work::VariableDeclaration(node) => step_variabledeclaration(node, &mut pending),
            Work::VariableDeclarator(node) => step_variabledeclarator(node, &mut pending),
            Work::WhileStatement(node) => step_whilestatement(node, &mut pending),
            Work::WithStatement(node) => step_withstatement(node, &mut pending),
            Work::YieldExpression(node) => step_yieldexpression(node, &mut pending),
            Work::ClassMemberList(items) => step_classmemberlist(items, &mut pending),
            Work::ExportSpecifierList(items) => step_exportspecifierlist(items, &mut pending),
            Work::ExpressionList(items) => step_expressionlist(items, &mut pending),
            Work::ExpressionPresentList(items) => step_expressionpresentlist(items, &mut pending),
            Work::FunctionParamList(items) => step_functionparamlist(items, &mut pending),
            Work::ImportSpecifierList(items) => step_importspecifierlist(items, &mut pending),
            Work::ObjectMemberList(items) => step_objectmemberlist(items, &mut pending),
            Work::ProgramItemList(items) => step_programitemlist(items, &mut pending),
            Work::StatementList(items) => step_statementlist(items, &mut pending),
            Work::SwitchCaseList(items) => step_switchcaselist(items, &mut pending),
            Work::TemplateElementList(items) => step_templateelementlist(items, &mut pending),
            Work::VariableDeclaratorList(items) => step_variabledeclaratorlist(items, &mut pending),
        }
    }
}

// Each step has a bounded stack frame, including in debug builds.
fn step_arrayexpression(node: ArrayExpression, pending: &mut Vec<Work>) {
    {
        let ArrayExpression { cv: _, elements } = node;
        pending.push(Work::ExpressionPresentList(elements.into_iter().flatten()));
    }
}

fn step_arrowbody(node: ArrowBody, pending: &mut Vec<Work>) {
    match node {
        ArrowBody::Expression(node) => {
            pending.push(Work::Expression(*node));
        }
        ArrowBody::Block(node) => {
            pending.push(Work::BlockStatement(node));
        }
    }
}

fn step_arrowfunctionexpression(node: ArrowFunctionExpression, pending: &mut Vec<Work>) {
    {
        let ArrowFunctionExpression {
            cv: _,
            params,
            body,
            is_async: _,
        } = node;
        pending.push(Work::FunctionParamList(params.into_iter()));
        pending.push(Work::ArrowBody(body));
    }
}

fn step_assignmentexpression(node: AssignmentExpression, pending: &mut Vec<Work>) {
    {
        let AssignmentExpression {
            cv: _,
            operator: _,
            left,
            right,
        } = node;
        pending.push(Work::AssignmentTarget(left));
        pending.push(Work::Expression(*right));
    }
}

fn step_assignmentpattern(node: AssignmentPattern, pending: &mut Vec<Work>) {
    {
        let AssignmentPattern { cv: _, left, right } = node;
        pending.push(Work::Identifier(left));
        pending.push(Work::Expression(right));
    }
}

fn step_assignmenttarget(node: AssignmentTarget, pending: &mut Vec<Work>) {
    match node {
        AssignmentTarget::MemberExpression(node) => {
            pending.push(Work::MemberExpression(*node));
        }
        AssignmentTarget::Identifier(node) => {
            pending.push(Work::Identifier(node));
        }
    }
}

fn step_awaitexpression(node: AwaitExpression, pending: &mut Vec<Work>) {
    {
        let AwaitExpression { cv: _, argument } = node;
        pending.push(Work::Expression(*argument));
    }
}

fn step_bigintliteral(node: BigIntLiteral) {
    {
        let BigIntLiteral {
            cv: _,
            value: _,
            raw: _,
        } = node;
    }
}

fn step_binaryexpression(node: BinaryExpression, pending: &mut Vec<Work>) {
    {
        let BinaryExpression {
            cv: _,
            operator: _,
            left,
            right,
        } = node;
        pending.push(Work::Expression(*left));
        pending.push(Work::Expression(*right));
    }
}

fn step_bindingtarget(node: BindingTarget, pending: &mut Vec<Work>) {
    match node {
        BindingTarget::Identifier(node) => {
            pending.push(Work::Identifier(node));
        }
    }
}

fn step_blockstatement(node: BlockStatement, pending: &mut Vec<Work>) {
    {
        let BlockStatement { cv: _, body } = node;
        pending.push(Work::StatementList(body.into_iter()));
    }
}

fn step_booleanliteral(node: BooleanLiteral) {
    {
        let BooleanLiteral { cv: _, value: _ } = node;
    }
}

fn step_breakstatement(node: BreakStatement, pending: &mut Vec<Work>) {
    {
        let BreakStatement { cv: _, label } = node;
        if let Some(value) = label {
            pending.push(Work::Identifier(value));
        }
    }
}

fn step_callexpression(node: CallExpression, pending: &mut Vec<Work>) {
    {
        let CallExpression {
            cv: _,
            callee,
            arguments,
        } = node;
        pending.push(Work::Expression(*callee));
        pending.push(Work::ExpressionList(arguments.into_iter()));
    }
}

fn step_catchclause(node: CatchClause, pending: &mut Vec<Work>) {
    {
        let CatchClause { cv: _, param, body } = node;
        if let Some(value) = param {
            pending.push(Work::Identifier(value));
        }
        pending.push(Work::BlockStatement(body));
    }
}

fn step_chainexpression(node: ChainExpression, pending: &mut Vec<Work>) {
    {
        let ChainExpression { cv: _, expression } = node;
        pending.push(Work::Expression(*expression));
    }
}

fn step_classdeclaration(node: ClassDeclaration, pending: &mut Vec<Work>) {
    {
        let ClassDeclaration {
            cv: _,
            id,
            super_class,
            body,
        } = node;
        pending.push(Work::Identifier(id));
        if let Some(value) = super_class {
            pending.push(Work::Expression(*value));
        }
        pending.push(Work::ClassMemberList(body.into_iter()));
    }
}

fn step_classexpression(node: ClassExpression, pending: &mut Vec<Work>) {
    {
        let ClassExpression {
            cv: _,
            id,
            super_class,
            body,
        } = node;
        if let Some(value) = id {
            pending.push(Work::Identifier(value));
        }
        if let Some(value) = super_class {
            pending.push(Work::Expression(*value));
        }
        pending.push(Work::ClassMemberList(body.into_iter()));
    }
}

fn step_classmember(node: ClassMember, pending: &mut Vec<Work>) {
    match node {
        ClassMember::Method(node) => {
            pending.push(Work::MethodDefinition(node));
        }
        ClassMember::Field(node) => {
            pending.push(Work::PropertyDefinition(node));
        }
        ClassMember::StaticBlock(node) => {
            pending.push(Work::BlockStatement(node));
        }
    }
}

fn step_conditionalexpression(node: ConditionalExpression, pending: &mut Vec<Work>) {
    {
        let ConditionalExpression {
            cv: _,
            test,
            consequent,
            alternate,
        } = node;
        pending.push(Work::Expression(*test));
        pending.push(Work::Expression(*consequent));
        pending.push(Work::Expression(*alternate));
    }
}

fn step_continuestatement(node: ContinueStatement, pending: &mut Vec<Work>) {
    {
        let ContinueStatement { cv: _, label } = node;
        if let Some(value) = label {
            pending.push(Work::Identifier(value));
        }
    }
}

fn step_debuggerstatement(node: DebuggerStatement) {
    {
        let DebuggerStatement { cv: _ } = node;
    }
}

fn step_declaration(node: Declaration, pending: &mut Vec<Work>) {
    match node {
        Declaration::VariableDeclaration(node) => {
            pending.push(Work::VariableDeclaration(node));
        }
        Declaration::FunctionDeclaration(node) => {
            pending.push(Work::FunctionDeclaration(node));
        }
        Declaration::ClassDeclaration(node) => {
            pending.push(Work::ClassDeclaration(node));
        }
        Declaration::ImportDeclaration(node) => {
            pending.push(Work::ImportDeclaration(node));
        }
        Declaration::ExportNamedDeclaration(node) => {
            pending.push(Work::ExportNamedDeclaration(node));
        }
        Declaration::ExportDefaultDeclaration(node) => {
            pending.push(Work::ExportDefaultDeclaration(node));
        }
        Declaration::ExportAllDeclaration(node) => {
            pending.push(Work::ExportAllDeclaration(node));
        }
    }
}

fn step_dowhilestatement(node: DoWhileStatement, pending: &mut Vec<Work>) {
    {
        let DoWhileStatement { cv: _, body, test } = node;
        pending.push(Work::Statement(*body));
        pending.push(Work::Expression(test));
    }
}

fn step_emptystatement(node: EmptyStatement) {
    {
        let EmptyStatement { cv: _ } = node;
    }
}

fn step_exportalldeclaration(node: ExportAllDeclaration, pending: &mut Vec<Work>) {
    {
        let ExportAllDeclaration {
            cv: _,
            exported,
            source,
        } = node;
        if let Some(value) = exported {
            pending.push(Work::Identifier(value));
        }
        pending.push(Work::StringLiteral(source));
    }
}

fn step_exportdefaultdeclaration(node: ExportDefaultDeclaration, pending: &mut Vec<Work>) {
    {
        let ExportDefaultDeclaration { cv: _, declaration } = node;
        pending.push(Work::ExportDefaultKind(declaration));
    }
}

fn step_exportdefaultkind(node: ExportDefaultKind, pending: &mut Vec<Work>) {
    match node {
        ExportDefaultKind::Expression(node) => {
            pending.push(Work::Expression(*node));
        }
        ExportDefaultKind::FunctionDeclaration(node) => {
            pending.push(Work::FunctionDeclaration(node));
        }
        ExportDefaultKind::ClassDeclaration(node) => {
            pending.push(Work::ClassDeclaration(node));
        }
    }
}

fn step_exportnameddeclaration(node: ExportNamedDeclaration, pending: &mut Vec<Work>) {
    {
        let ExportNamedDeclaration {
            cv: _,
            declaration,
            specifiers,
            source,
        } = node;
        if let Some(value) = declaration {
            pending.push(Work::Declaration(*value));
        }
        pending.push(Work::ExportSpecifierList(specifiers.into_iter()));
        if let Some(value) = source {
            pending.push(Work::StringLiteral(value));
        }
    }
}

fn step_exportspecifier(node: ExportSpecifier, pending: &mut Vec<Work>) {
    {
        let ExportSpecifier { local, exported } = node;
        pending.push(Work::Identifier(local));
        pending.push(Work::Identifier(exported));
    }
}

fn step_expression(node: Expression, pending: &mut Vec<Work>) {
    match node {
        Expression::Identifier(node) => {
            pending.push(Work::Identifier(node));
        }
        Expression::NumericLiteral(node) => {
            pending.push(Work::NumericLiteral(node));
        }
        Expression::StringLiteral(node) => {
            pending.push(Work::StringLiteral(node));
        }
        Expression::BooleanLiteral(node) => {
            pending.push(Work::BooleanLiteral(node));
        }
        Expression::NullLiteral(node) => {
            pending.push(Work::NullLiteral(node));
        }
        Expression::BigIntLiteral(node) => {
            pending.push(Work::BigIntLiteral(node));
        }
        Expression::UndefinedLiteral(node) => {
            pending.push(Work::UndefinedLiteral(node));
        }
        Expression::RegExpLiteral(node) => {
            pending.push(Work::RegExpLiteral(node));
        }
        Expression::BinaryExpression(node) => {
            pending.push(Work::BinaryExpression(node));
        }
        Expression::LogicalExpression(node) => {
            pending.push(Work::LogicalExpression(node));
        }
        Expression::UnaryExpression(node) => {
            pending.push(Work::UnaryExpression(node));
        }
        Expression::AssignmentExpression(node) => {
            pending.push(Work::AssignmentExpression(node));
        }
        Expression::ConditionalExpression(node) => {
            pending.push(Work::ConditionalExpression(node));
        }
        Expression::CallExpression(node) => {
            pending.push(Work::CallExpression(node));
        }
        Expression::MemberExpression(node) => {
            pending.push(Work::MemberExpression(node));
        }
        Expression::OptionalMemberExpression(node) => {
            pending.push(Work::OptionalMemberExpression(node));
        }
        Expression::OptionalCallExpression(node) => {
            pending.push(Work::OptionalCallExpression(node));
        }
        Expression::ChainExpression(node) => {
            pending.push(Work::ChainExpression(node));
        }
        Expression::ArrayExpression(node) => {
            pending.push(Work::ArrayExpression(node));
        }
        Expression::ObjectExpression(node) => {
            pending.push(Work::ObjectExpression(node));
        }
        Expression::FunctionExpression(node) => {
            pending.push(Work::FunctionExpression(node));
        }
        Expression::ArrowFunctionExpression(node) => {
            pending.push(Work::ArrowFunctionExpression(node));
        }
        Expression::ClassExpression(node) => {
            pending.push(Work::ClassExpression(node));
        }
        Expression::TemplateLiteral(node) => {
            pending.push(Work::TemplateLiteral(node));
        }
        Expression::UpdateExpression(node) => {
            pending.push(Work::UpdateExpression(node));
        }
        Expression::NewExpression(node) => {
            pending.push(Work::NewExpression(node));
        }
        Expression::SequenceExpression(node) => {
            pending.push(Work::SequenceExpression(node));
        }
        Expression::TaggedTemplateExpression(node) => {
            pending.push(Work::TaggedTemplateExpression(node));
        }
        Expression::SpreadElement(node) => {
            pending.push(Work::SpreadElement(node));
        }
        Expression::YieldExpression(node) => {
            pending.push(Work::YieldExpression(node));
        }
        Expression::AwaitExpression(node) => {
            pending.push(Work::AwaitExpression(node));
        }
        Expression::ThisExpression(node) => {
            pending.push(Work::ThisExpression(node));
        }
        Expression::Super(node) => {
            pending.push(Work::Super(node));
        }
        Expression::NewTarget(node) => {
            pending.push(Work::NewTarget(node));
        }
        Expression::ImportMeta(node) => {
            pending.push(Work::ImportMeta(node));
        }
        Expression::ImportExpression(node) => {
            pending.push(Work::ImportExpression(node));
        }
    }
}

fn step_expressionstatement(node: ExpressionStatement, pending: &mut Vec<Work>) {
    {
        let ExpressionStatement { cv: _, expression } = node;
        pending.push(Work::Expression(expression));
    }
}

fn step_forinstatement(node: ForInStatement, pending: &mut Vec<Work>) {
    {
        let ForInStatement {
            cv: _,
            left,
            right,
            body,
        } = node;
        pending.push(Work::ForInit(left));
        pending.push(Work::Expression(right));
        pending.push(Work::Statement(*body));
    }
}

fn step_forinit(node: ForInit, pending: &mut Vec<Work>) {
    match node {
        ForInit::VariableDeclaration(node) => {
            pending.push(Work::VariableDeclaration(node));
        }
        ForInit::Expression(node) => {
            pending.push(Work::Expression(node));
        }
    }
}

fn step_forofstatement(node: ForOfStatement, pending: &mut Vec<Work>) {
    {
        let ForOfStatement {
            cv: _,
            left,
            right,
            body,
        } = node;
        pending.push(Work::ForInit(left));
        pending.push(Work::Expression(right));
        pending.push(Work::Statement(*body));
    }
}

fn step_forstatement(node: ForStatement, pending: &mut Vec<Work>) {
    {
        let ForStatement {
            cv: _,
            init,
            test,
            update,
            body,
        } = node;
        if let Some(value) = init {
            pending.push(Work::ForInit(value));
        }
        if let Some(value) = test {
            pending.push(Work::Expression(value));
        }
        if let Some(value) = update {
            pending.push(Work::Expression(value));
        }
        pending.push(Work::Statement(*body));
    }
}

fn step_functiondeclaration(node: FunctionDeclaration, pending: &mut Vec<Work>) {
    {
        let FunctionDeclaration {
            cv: _,
            id,
            params,
            body,
            generator: _,
            is_async: _,
        } = node;
        pending.push(Work::Identifier(id));
        pending.push(Work::FunctionParamList(params.into_iter()));
        pending.push(Work::BlockStatement(body));
    }
}

fn step_functionexpression(node: FunctionExpression, pending: &mut Vec<Work>) {
    {
        let FunctionExpression {
            cv: _,
            id,
            params,
            body,
            generator: _,
            is_async: _,
        } = node;
        if let Some(value) = id {
            pending.push(Work::Identifier(value));
        }
        pending.push(Work::FunctionParamList(params.into_iter()));
        pending.push(Work::BlockStatement(body));
    }
}

fn step_functionparam(node: FunctionParam, pending: &mut Vec<Work>) {
    match node {
        FunctionParam::Identifier(node) => {
            pending.push(Work::Identifier(node));
        }
        FunctionParam::RestElement(node) => {
            pending.push(Work::RestElement(node));
        }
        FunctionParam::AssignmentPattern(node) => {
            pending.push(Work::AssignmentPattern(node));
        }
    }
}

fn step_identifier(node: Identifier) {
    {
        let Identifier { cv: _, name: _ } = node;
    }
}

fn step_ifstatement(node: IfStatement, pending: &mut Vec<Work>) {
    {
        let IfStatement {
            cv: _,
            test,
            consequent,
            alternate,
        } = node;
        pending.push(Work::Expression(test));
        pending.push(Work::Statement(*consequent));
        if let Some(value) = alternate {
            pending.push(Work::Statement(*value));
        }
    }
}

fn step_importdeclaration(node: ImportDeclaration, pending: &mut Vec<Work>) {
    {
        let ImportDeclaration {
            cv: _,
            specifiers,
            source,
        } = node;
        pending.push(Work::ImportSpecifierList(specifiers.into_iter()));
        pending.push(Work::StringLiteral(source));
    }
}

fn step_importexpression(node: ImportExpression, pending: &mut Vec<Work>) {
    {
        let ImportExpression { cv: _, source } = node;
        pending.push(Work::Expression(*source));
    }
}

fn step_importmeta(node: ImportMeta) {
    {
        let ImportMeta { cv: _ } = node;
    }
}

fn step_importspecifier(node: ImportSpecifier, pending: &mut Vec<Work>) {
    match node {
        ImportSpecifier::Default(node) => {
            pending.push(Work::Identifier(node));
        }
        ImportSpecifier::Namespace(node) => {
            pending.push(Work::Identifier(node));
        }
        ImportSpecifier::Named { imported, local } => {
            pending.push(Work::Identifier(imported));
            pending.push(Work::Identifier(local));
        }
    }
}

fn step_labeledstatement(node: LabeledStatement, pending: &mut Vec<Work>) {
    {
        let LabeledStatement { cv: _, label, body } = node;
        pending.push(Work::Identifier(label));
        pending.push(Work::Statement(*body));
    }
}

fn step_logicalexpression(node: LogicalExpression, pending: &mut Vec<Work>) {
    {
        let LogicalExpression {
            cv: _,
            operator: _,
            left,
            right,
        } = node;
        pending.push(Work::Expression(*left));
        pending.push(Work::Expression(*right));
    }
}

fn step_memberexpression(node: MemberExpression, pending: &mut Vec<Work>) {
    {
        let MemberExpression {
            cv: _,
            object,
            property,
            computed: _,
        } = node;
        pending.push(Work::Expression(*object));
        pending.push(Work::Expression(*property));
    }
}

fn step_methoddefinition(node: MethodDefinition, pending: &mut Vec<Work>) {
    {
        let MethodDefinition {
            cv: _,
            key,
            kind: _,
            value,
            computed: _,
            is_static: _,
        } = node;
        pending.push(Work::PropertyKey(key));
        pending.push(Work::FunctionExpression(value));
    }
}

fn step_newexpression(node: NewExpression, pending: &mut Vec<Work>) {
    {
        let NewExpression {
            cv: _,
            callee,
            arguments,
        } = node;
        pending.push(Work::Expression(*callee));
        pending.push(Work::ExpressionList(arguments.into_iter()));
    }
}

fn step_newtarget(node: NewTarget) {
    {
        let NewTarget { cv: _ } = node;
    }
}

fn step_nullliteral(node: NullLiteral) {
    {
        let NullLiteral { cv: _ } = node;
    }
}

fn step_numericliteral(node: NumericLiteral) {
    {
        let NumericLiteral {
            cv: _,
            value: _,
            raw: _,
        } = node;
    }
}

fn step_objectexpression(node: ObjectExpression, pending: &mut Vec<Work>) {
    {
        let ObjectExpression { cv: _, properties } = node;
        pending.push(Work::ObjectMemberList(properties.into_iter()));
    }
}

fn step_objectmember(node: ObjectMember, pending: &mut Vec<Work>) {
    match node {
        ObjectMember::Property(node) => {
            pending.push(Work::Property(node));
        }
        ObjectMember::Spread(node) => {
            pending.push(Work::SpreadElement(node));
        }
    }
}

fn step_optionalcallexpression(node: OptionalCallExpression, pending: &mut Vec<Work>) {
    {
        let OptionalCallExpression {
            cv: _,
            callee,
            arguments,
        } = node;
        pending.push(Work::Expression(*callee));
        pending.push(Work::ExpressionList(arguments.into_iter()));
    }
}

fn step_optionalmemberexpression(node: OptionalMemberExpression, pending: &mut Vec<Work>) {
    {
        let OptionalMemberExpression {
            cv: _,
            object,
            property,
            computed: _,
        } = node;
        pending.push(Work::Expression(*object));
        pending.push(Work::Expression(*property));
    }
}

fn step_privatename(node: PrivateName) {
    {
        let PrivateName { cv: _, name: _ } = node;
    }
}

fn step_program(node: Program, pending: &mut Vec<Work>) {
    {
        let Program {
            cv: _,
            version: _,
            source_type: _,
            body,
        } = node;
        pending.push(Work::ProgramItemList(body.into_iter()));
    }
}

fn step_programitem(node: ProgramItem, pending: &mut Vec<Work>) {
    match node {
        ProgramItem::Statement(node) => {
            pending.push(Work::Statement(node));
        }
        ProgramItem::Declaration(node) => {
            pending.push(Work::Declaration(node));
        }
    }
}

fn step_property(node: Property, pending: &mut Vec<Work>) {
    {
        let Property {
            cv: _,
            kind: _,
            key,
            value,
            computed: _,
            shorthand: _,
            method: _,
        } = node;
        pending.push(Work::PropertyKey(key));
        pending.push(Work::Expression(*value));
    }
}

fn step_propertydefinition(node: PropertyDefinition, pending: &mut Vec<Work>) {
    {
        let PropertyDefinition {
            cv: _,
            key,
            value,
            computed: _,
            is_static: _,
        } = node;
        pending.push(Work::PropertyKey(key));
        if let Some(value) = value {
            pending.push(Work::Expression(value));
        }
    }
}

fn step_propertykey(node: PropertyKey, pending: &mut Vec<Work>) {
    match node {
        PropertyKey::Expression(node) => {
            pending.push(Work::Expression(*node));
        }
        PropertyKey::Identifier(node) => {
            pending.push(Work::Identifier(node));
        }
        PropertyKey::PrivateName(node) => {
            pending.push(Work::PrivateName(node));
        }
        PropertyKey::StringLiteral(node) => {
            pending.push(Work::StringLiteral(node));
        }
        PropertyKey::NumericLiteral(node) => {
            pending.push(Work::NumericLiteral(node));
        }
    }
}

fn step_regexpliteral(node: RegExpLiteral) {
    {
        let RegExpLiteral {
            cv: _,
            pattern: _,
            flags: _,
        } = node;
    }
}

fn step_restelement(node: RestElement, pending: &mut Vec<Work>) {
    {
        let RestElement { cv: _, argument } = node;
        pending.push(Work::Identifier(argument));
    }
}

fn step_returnstatement(node: ReturnStatement, pending: &mut Vec<Work>) {
    {
        let ReturnStatement { cv: _, argument } = node;
        if let Some(value) = argument {
            pending.push(Work::Expression(value));
        }
    }
}

fn step_sequenceexpression(node: SequenceExpression, pending: &mut Vec<Work>) {
    {
        let SequenceExpression { cv: _, expressions } = node;
        pending.push(Work::ExpressionList(expressions.into_iter()));
    }
}

fn step_spreadelement(node: SpreadElement, pending: &mut Vec<Work>) {
    {
        let SpreadElement { cv: _, argument } = node;
        pending.push(Work::Expression(*argument));
    }
}

fn step_statement(node: Statement, pending: &mut Vec<Work>) {
    match node {
        Statement::Tagged(node) => {
            pending.push(Work::TaggedStatement(node));
        }
        Statement::Declaration(node) => {
            pending.push(Work::Declaration(node));
        }
    }
}

fn step_stringliteral(node: StringLiteral) {
    {
        let StringLiteral {
            cv: _,
            value: _,
            raw: _,
        } = node;
    }
}

fn step_super(node: Super) {
    {
        let Super { cv: _ } = node;
    }
}

fn step_switchcase(node: SwitchCase, pending: &mut Vec<Work>) {
    {
        let SwitchCase {
            cv: _,
            test,
            consequent,
        } = node;
        if let Some(value) = test {
            pending.push(Work::Expression(value));
        }
        pending.push(Work::StatementList(consequent.into_iter()));
    }
}

fn step_switchstatement(node: SwitchStatement, pending: &mut Vec<Work>) {
    {
        let SwitchStatement {
            cv: _,
            discriminant,
            cases,
        } = node;
        pending.push(Work::Expression(discriminant));
        pending.push(Work::SwitchCaseList(cases.into_iter()));
    }
}

fn step_taggedstatement(node: TaggedStatement, pending: &mut Vec<Work>) {
    match node {
        TaggedStatement::ExpressionStatement(node) => {
            pending.push(Work::ExpressionStatement(node));
        }
        TaggedStatement::BlockStatement(node) => {
            pending.push(Work::BlockStatement(node));
        }
        TaggedStatement::IfStatement(node) => {
            pending.push(Work::IfStatement(node));
        }
        TaggedStatement::WhileStatement(node) => {
            pending.push(Work::WhileStatement(node));
        }
        TaggedStatement::DoWhileStatement(node) => {
            pending.push(Work::DoWhileStatement(node));
        }
        TaggedStatement::ForStatement(node) => {
            pending.push(Work::ForStatement(node));
        }
        TaggedStatement::ForInStatement(node) => {
            pending.push(Work::ForInStatement(node));
        }
        TaggedStatement::ForOfStatement(node) => {
            pending.push(Work::ForOfStatement(node));
        }
        TaggedStatement::ReturnStatement(node) => {
            pending.push(Work::ReturnStatement(node));
        }
        TaggedStatement::BreakStatement(node) => {
            pending.push(Work::BreakStatement(node));
        }
        TaggedStatement::ContinueStatement(node) => {
            pending.push(Work::ContinueStatement(node));
        }
        TaggedStatement::LabeledStatement(node) => {
            pending.push(Work::LabeledStatement(node));
        }
        TaggedStatement::ThrowStatement(node) => {
            pending.push(Work::ThrowStatement(node));
        }
        TaggedStatement::SwitchStatement(node) => {
            pending.push(Work::SwitchStatement(node));
        }
        TaggedStatement::TryStatement(node) => {
            pending.push(Work::TryStatement(node));
        }
        TaggedStatement::EmptyStatement(node) => {
            pending.push(Work::EmptyStatement(node));
        }
        TaggedStatement::DebuggerStatement(node) => {
            pending.push(Work::DebuggerStatement(node));
        }
        TaggedStatement::WithStatement(node) => {
            pending.push(Work::WithStatement(node));
        }
    }
}

fn step_taggedtemplateexpression(node: TaggedTemplateExpression, pending: &mut Vec<Work>) {
    {
        let TaggedTemplateExpression { cv: _, tag, quasi } = node;
        pending.push(Work::Expression(*tag));
        pending.push(Work::TemplateLiteral(quasi));
    }
}

fn step_templateelement(node: TemplateElement) {
    {
        let TemplateElement {
            cv: _,
            raw: _,
            cooked: _,
            tail: _,
        } = node;
    }
}

fn step_templateliteral(node: TemplateLiteral, pending: &mut Vec<Work>) {
    {
        let TemplateLiteral {
            cv: _,
            quasis,
            expressions,
        } = node;
        pending.push(Work::TemplateElementList(quasis.into_iter()));
        pending.push(Work::ExpressionList(expressions.into_iter()));
    }
}

fn step_thisexpression(node: ThisExpression) {
    {
        let ThisExpression { cv: _ } = node;
    }
}

fn step_throwstatement(node: ThrowStatement, pending: &mut Vec<Work>) {
    {
        let ThrowStatement { cv: _, argument } = node;
        pending.push(Work::Expression(argument));
    }
}

fn step_trystatement(node: TryStatement, pending: &mut Vec<Work>) {
    {
        let TryStatement {
            cv: _,
            block,
            handler,
            finalizer,
        } = node;
        pending.push(Work::BlockStatement(block));
        if let Some(value) = handler {
            pending.push(Work::CatchClause(value));
        }
        if let Some(value) = finalizer {
            pending.push(Work::BlockStatement(value));
        }
    }
}

fn step_unaryexpression(node: UnaryExpression, pending: &mut Vec<Work>) {
    {
        let UnaryExpression {
            cv: _,
            operator: _,
            prefix: _,
            argument,
        } = node;
        pending.push(Work::Expression(*argument));
    }
}

fn step_undefinedliteral(node: UndefinedLiteral) {
    {
        let UndefinedLiteral { cv: _ } = node;
    }
}

fn step_updateexpression(node: UpdateExpression, pending: &mut Vec<Work>) {
    {
        let UpdateExpression {
            cv: _,
            operator: _,
            prefix: _,
            argument,
        } = node;
        pending.push(Work::Expression(*argument));
    }
}

fn step_variabledeclaration(node: VariableDeclaration, pending: &mut Vec<Work>) {
    {
        let VariableDeclaration {
            cv: _,
            kind: _,
            declarations,
        } = node;
        pending.push(Work::VariableDeclaratorList(declarations.into_iter()));
    }
}

fn step_variabledeclarator(node: VariableDeclarator, pending: &mut Vec<Work>) {
    {
        let VariableDeclarator { cv: _, id, init } = node;
        pending.push(Work::BindingTarget(id));
        if let Some(value) = init {
            pending.push(Work::Expression(value));
        }
    }
}

fn step_whilestatement(node: WhileStatement, pending: &mut Vec<Work>) {
    {
        let WhileStatement { cv: _, test, body } = node;
        pending.push(Work::Expression(test));
        pending.push(Work::Statement(*body));
    }
}

fn step_withstatement(node: WithStatement, pending: &mut Vec<Work>) {
    {
        let WithStatement {
            cv: _,
            object,
            body,
        } = node;
        pending.push(Work::Expression(object));
        pending.push(Work::Statement(*body));
    }
}

fn step_yieldexpression(node: YieldExpression, pending: &mut Vec<Work>) {
    {
        let YieldExpression {
            cv: _,
            delegate: _,
            argument,
        } = node;
        if let Some(value) = argument {
            pending.push(Work::Expression(*value));
        }
    }
}

fn step_classmemberlist(mut items: std::vec::IntoIter<ClassMember>, pending: &mut Vec<Work>) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::ClassMemberList(items));
            pending.push(Work::ClassMember(item));
        }
    }
}

fn step_exportspecifierlist(
    mut items: std::vec::IntoIter<ExportSpecifier>,
    pending: &mut Vec<Work>,
) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::ExportSpecifierList(items));
            pending.push(Work::ExportSpecifier(item));
        }
    }
}

fn step_expressionlist(mut items: std::vec::IntoIter<Expression>, pending: &mut Vec<Work>) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::ExpressionList(items));
            pending.push(Work::Expression(item));
        }
    }
}

fn step_expressionpresentlist(
    mut items: std::iter::Flatten<std::vec::IntoIter<Option<Expression>>>,
    pending: &mut Vec<Work>,
) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::ExpressionPresentList(items));
            pending.push(Work::Expression(item));
        }
    }
}

fn step_functionparamlist(mut items: std::vec::IntoIter<FunctionParam>, pending: &mut Vec<Work>) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::FunctionParamList(items));
            pending.push(Work::FunctionParam(item));
        }
    }
}

fn step_importspecifierlist(
    mut items: std::vec::IntoIter<ImportSpecifier>,
    pending: &mut Vec<Work>,
) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::ImportSpecifierList(items));
            pending.push(Work::ImportSpecifier(item));
        }
    }
}

fn step_objectmemberlist(mut items: std::vec::IntoIter<ObjectMember>, pending: &mut Vec<Work>) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::ObjectMemberList(items));
            pending.push(Work::ObjectMember(item));
        }
    }
}

fn step_programitemlist(mut items: std::vec::IntoIter<ProgramItem>, pending: &mut Vec<Work>) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::ProgramItemList(items));
            pending.push(Work::ProgramItem(item));
        }
    }
}

fn step_statementlist(mut items: std::vec::IntoIter<Statement>, pending: &mut Vec<Work>) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::StatementList(items));
            pending.push(Work::Statement(item));
        }
    }
}

fn step_switchcaselist(mut items: std::vec::IntoIter<SwitchCase>, pending: &mut Vec<Work>) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::SwitchCaseList(items));
            pending.push(Work::SwitchCase(item));
        }
    }
}

fn step_templateelementlist(
    mut items: std::vec::IntoIter<TemplateElement>,
    pending: &mut Vec<Work>,
) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::TemplateElementList(items));
            pending.push(Work::TemplateElement(item));
        }
    }
}

fn step_variabledeclaratorlist(
    mut items: std::vec::IntoIter<VariableDeclarator>,
    pending: &mut Vec<Work>,
) {
    {
        if let Some(item) = items.next() {
            pending.push(Work::VariableDeclaratorList(items));
            pending.push(Work::VariableDeclarator(item));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_javascript_tokens::EsVersion;

    fn leaf() -> Expression {
        Expression::Identifier(Identifier {
            cv: None,
            name: "x".into(),
        })
    }
    fn block(statement: Statement) -> BlockStatement {
        BlockStatement {
            cv: None,
            body: vec![statement],
        }
    }

    #[test]
    fn dispose_deep_mixed_families_and_wide_lists_on_small_stack() {
        const CHILD: &str = "CV02_AST_DISPOSAL_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "disposal::tests::dispose_deep_mixed_families_and_wide_lists_on_small_stack",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "isolated disposal failed: {:?}\n{}",
                result.status,
                String::from_utf8_lossy(&result.stderr)
            );
            return;
        }
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                // The deep child alternates among parameter defaults, computed
                // class keys, templates, optional-call arguments and array slots.
                let mut expression = leaf();
                for index in 0..4096 {
                    expression = match index % 5 {
                        0 => Expression::ArrowFunctionExpression(ArrowFunctionExpression {
                            cv: None,
                            params: vec![FunctionParam::AssignmentPattern(AssignmentPattern {
                                cv: None,
                                left: Identifier {
                                    cv: None,
                                    name: "p".into(),
                                },
                                right: expression,
                            })],
                            body: ArrowBody::Expression(Box::new(leaf())),
                            is_async: false,
                        }),
                        1 => Expression::ClassExpression(ClassExpression {
                            cv: None,
                            id: None,
                            super_class: None,
                            body: vec![ClassMember::Field(PropertyDefinition {
                                cv: None,
                                key: PropertyKey::Expression(Box::new(expression)),
                                value: None,
                                computed: true,
                                is_static: false,
                            })],
                        }),
                        2 => Expression::TaggedTemplateExpression(TaggedTemplateExpression {
                            cv: None,
                            tag: Box::new(leaf()),
                            quasi: TemplateLiteral {
                                cv: None,
                                quasis: vec![TemplateElement {
                                    cv: None,
                                    raw: "".into(),
                                    cooked: Some("".into()),
                                    tail: true,
                                }],
                                expressions: vec![expression],
                            },
                        }),
                        3 => Expression::OptionalCallExpression(OptionalCallExpression {
                            cv: None,
                            callee: Box::new(leaf()),
                            arguments: vec![expression],
                        }),
                        _ => Expression::ArrayExpression(ArrayExpression {
                            cv: None,
                            elements: vec![None, Some(expression), None],
                        }),
                    };
                }
                let mut statement = Statement::expression_statement(ExpressionStatement {
                    cv: None,
                    expression,
                });
                for index in 0..4096 {
                    statement = match index % 3 {
                        0 => Statement::Tagged(TaggedStatement::TryStatement(TryStatement {
                            cv: None,
                            block: BlockStatement {
                                cv: None,
                                body: vec![],
                            },
                            handler: Some(CatchClause {
                                cv: None,
                                param: None,
                                body: block(statement),
                            }),
                            finalizer: None,
                        })),
                        1 => Statement::Tagged(TaggedStatement::SwitchStatement(SwitchStatement {
                            cv: None,
                            discriminant: leaf(),
                            cases: vec![SwitchCase {
                                cv: None,
                                test: None,
                                consequent: vec![statement],
                            }],
                        })),
                        _ => Statement::Tagged(TaggedStatement::ForStatement(ForStatement {
                            cv: None,
                            init: Some(ForInit::Expression(leaf())),
                            test: None,
                            update: None,
                            body: Box::new(statement),
                        })),
                    };
                }
                let item = ProgramItem::Declaration(Declaration::ExportNamedDeclaration(
                    ExportNamedDeclaration {
                        cv: None,
                        declaration: Some(Box::new(Declaration::ExportDefaultDeclaration(
                            ExportDefaultDeclaration {
                                cv: None,
                                declaration: ExportDefaultKind::FunctionDeclaration(
                                    FunctionDeclaration {
                                        cv: None,
                                        id: Identifier {
                                            cv: None,
                                            name: "f".into(),
                                        },
                                        params: vec![],
                                        body: block(statement),
                                        generator: false,
                                        is_async: false,
                                    },
                                ),
                            },
                        ))),
                        specifiers: vec![],
                        source: None,
                    },
                ));
                let program = Program::new("root".into(), EsVersion::Es2025, SourceType::Module)
                    .with_body(vec![item]);
                dispose_program(program);
                // IntoIter frames reuse sibling storage. Holes exercise flatten's
                // skipping path without recursive drop of present expression slots.
                let elements = (0..100_000)
                    .map(|i| if i % 2 == 0 { Some(leaf()) } else { None })
                    .collect();
                let program = Program::new("wide".into(), EsVersion::Es2025, SourceType::Module)
                    .with_body(vec![ProgramItem::Statement(
                        Statement::expression_statement(ExpressionStatement {
                            cv: None,
                            expression: Expression::ArrayExpression(ArrayExpression {
                                cv: None,
                                elements,
                            }),
                        }),
                    )]);
                dispose_program(program);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
