//! Each constructor creates distinct nonempty CVs in native declaration order.
//! These typed fixtures exercise every reachable variant and every AST field
//! populated, including normally opaque statement/declaration/class wrappers.
//! Expected IDs come from construction, independently of production dispatch.
use super::*;

#[derive(Default)]
struct Seed {
    ids: Vec<String>,
}
impl Seed {
    fn id(&mut self) -> String {
        let id = format!("fixture-{}", self.ids.len());
        self.ids.push(id.clone());
        id
    }
}

fn check(frame: Frame<'_>, expected: &[String]) {
    assert!(!expected.is_empty(), "every case has real native CVs");
    let mut budget = Budget {
        work: 1_000_000,
        bytes: 1_000_000,
    };
    let mut stack = Stack::new();
    stack.push(frame, &mut budget, &mut |_| Ok(())).unwrap();
    let mut actual = Vec::new();
    while !stack.frames.is_empty() {
        budget.take(2).unwrap();
        let frame = stack.frames.pop().unwrap();
        step(
            frame,
            &mut stack,
            &mut budget,
            &mut |_| Ok(()),
            &mut |id, _| {
                actual.push(id.to_owned());
                Ok(())
            },
        )
        .unwrap();
    }
    assert_eq!(
        actual, expected,
        "complete native field declaration preorder"
    );
}

fn make_arrayexpression(seed: &mut Seed, full: bool) -> ArrayExpression {
    ArrayExpression {
        cv: Some(seed.id()),
        elements: if full {
            vec![
                Some(make_expression(seed, false)),
                Some(make_expression(seed, false)),
            ]
        } else {
            vec![]
        },
    }
}

fn make_arrowbody(seed: &mut Seed, _full: bool) -> ArrowBody {
    ArrowBody::Expression(Box::new(make_expression(seed, false)))
}

fn make_arrowfunctionexpression(seed: &mut Seed, full: bool) -> ArrowFunctionExpression {
    ArrowFunctionExpression {
        cv: Some(seed.id()),
        params: if full {
            vec![
                make_functionparam(seed, false),
                make_functionparam(seed, false),
            ]
        } else {
            vec![]
        },
        body: make_arrowbody(seed, false),
        is_async: false,
    }
}

fn make_assignmentexpression(seed: &mut Seed, _full: bool) -> AssignmentExpression {
    AssignmentExpression {
        cv: Some(seed.id()),
        operator: AssignmentOperator::Eq,
        left: make_assignmenttarget(seed, false),
        right: Box::new(make_expression(seed, false)),
    }
}

fn make_assignmentpattern(seed: &mut Seed, _full: bool) -> AssignmentPattern {
    AssignmentPattern {
        cv: Some(seed.id()),
        left: make_identifier(seed, false),
        right: make_expression(seed, false),
    }
}

fn make_assignmenttarget(seed: &mut Seed, _full: bool) -> AssignmentTarget {
    AssignmentTarget::Identifier(make_identifier(seed, false))
}

fn make_awaitexpression(seed: &mut Seed, _full: bool) -> AwaitExpression {
    AwaitExpression {
        cv: Some(seed.id()),
        argument: Box::new(make_expression(seed, false)),
    }
}

fn make_bigintliteral(seed: &mut Seed, _full: bool) -> BigIntLiteral {
    BigIntLiteral {
        cv: Some(seed.id()),
        value: "payload".into(),
        raw: "payload".into(),
    }
}

fn make_binaryexpression(seed: &mut Seed, _full: bool) -> BinaryExpression {
    BinaryExpression {
        cv: Some(seed.id()),
        operator: BinaryOperator::Add,
        left: Box::new(make_expression(seed, false)),
        right: Box::new(make_expression(seed, false)),
    }
}

fn make_bindingtarget(seed: &mut Seed, _full: bool) -> BindingTarget {
    BindingTarget::Identifier(make_identifier(seed, false))
}

fn make_blockstatement(seed: &mut Seed, full: bool) -> BlockStatement {
    BlockStatement {
        cv: Some(seed.id()),
        body: if full {
            vec![make_statement(seed, false), make_statement(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_booleanliteral(seed: &mut Seed, _full: bool) -> BooleanLiteral {
    BooleanLiteral {
        cv: Some(seed.id()),
        value: false,
    }
}

fn make_breakstatement(seed: &mut Seed, full: bool) -> BreakStatement {
    BreakStatement {
        cv: Some(seed.id()),
        label: if full {
            Some(make_identifier(seed, false))
        } else {
            None
        },
    }
}

fn make_callexpression(seed: &mut Seed, full: bool) -> CallExpression {
    CallExpression {
        cv: Some(seed.id()),
        callee: Box::new(make_expression(seed, false)),
        arguments: if full {
            vec![make_expression(seed, false), make_expression(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_catchclause(seed: &mut Seed, full: bool) -> CatchClause {
    CatchClause {
        cv: Some(seed.id()),
        param: if full {
            Some(make_identifier(seed, false))
        } else {
            None
        },
        body: make_blockstatement(seed, false),
    }
}

fn make_chainexpression(seed: &mut Seed, _full: bool) -> ChainExpression {
    ChainExpression {
        cv: Some(seed.id()),
        expression: Box::new(make_expression(seed, false)),
    }
}

fn make_classdeclaration(seed: &mut Seed, full: bool) -> ClassDeclaration {
    ClassDeclaration {
        cv: Some(seed.id()),
        id: make_identifier(seed, false),
        super_class: if full {
            Some(Box::new(make_expression(seed, false)))
        } else {
            None
        },
        body: if full {
            vec![make_classmember(seed, false), make_classmember(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_classexpression(seed: &mut Seed, full: bool) -> ClassExpression {
    ClassExpression {
        cv: Some(seed.id()),
        id: if full {
            Some(make_identifier(seed, false))
        } else {
            None
        },
        super_class: if full {
            Some(Box::new(make_expression(seed, false)))
        } else {
            None
        },
        body: if full {
            vec![make_classmember(seed, false), make_classmember(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_classmember(seed: &mut Seed, _full: bool) -> ClassMember {
    ClassMember::Method(make_methoddefinition(seed, false))
}

fn make_conditionalexpression(seed: &mut Seed, _full: bool) -> ConditionalExpression {
    ConditionalExpression {
        cv: Some(seed.id()),
        test: Box::new(make_expression(seed, false)),
        consequent: Box::new(make_expression(seed, false)),
        alternate: Box::new(make_expression(seed, false)),
    }
}

fn make_continuestatement(seed: &mut Seed, full: bool) -> ContinueStatement {
    ContinueStatement {
        cv: Some(seed.id()),
        label: if full {
            Some(make_identifier(seed, false))
        } else {
            None
        },
    }
}

fn make_debuggerstatement(seed: &mut Seed, _full: bool) -> DebuggerStatement {
    DebuggerStatement {
        cv: Some(seed.id()),
    }
}

fn make_declaration(seed: &mut Seed, _full: bool) -> Declaration {
    Declaration::VariableDeclaration(make_variabledeclaration(seed, false))
}

fn make_dowhilestatement(seed: &mut Seed, _full: bool) -> DoWhileStatement {
    DoWhileStatement {
        cv: Some(seed.id()),
        body: Box::new(make_statement(seed, false)),
        test: make_expression(seed, false),
    }
}

fn make_emptystatement(seed: &mut Seed, _full: bool) -> EmptyStatement {
    EmptyStatement {
        cv: Some(seed.id()),
    }
}

fn make_exportalldeclaration(seed: &mut Seed, full: bool) -> ExportAllDeclaration {
    ExportAllDeclaration {
        cv: Some(seed.id()),
        exported: if full {
            Some(make_identifier(seed, false))
        } else {
            None
        },
        source: make_stringliteral(seed, false),
    }
}

fn make_exportdefaultdeclaration(seed: &mut Seed, _full: bool) -> ExportDefaultDeclaration {
    ExportDefaultDeclaration {
        cv: Some(seed.id()),
        declaration: make_exportdefaultkind(seed, false),
    }
}

fn make_exportdefaultkind(seed: &mut Seed, _full: bool) -> ExportDefaultKind {
    ExportDefaultKind::Expression(Box::new(make_expression(seed, false)))
}

fn make_exportnameddeclaration(seed: &mut Seed, full: bool) -> ExportNamedDeclaration {
    ExportNamedDeclaration {
        cv: Some(seed.id()),
        declaration: if full {
            Some(Box::new(make_declaration(seed, false)))
        } else {
            None
        },
        specifiers: if full {
            vec![
                make_exportspecifier(seed, false),
                make_exportspecifier(seed, false),
            ]
        } else {
            vec![]
        },
        source: if full {
            Some(make_stringliteral(seed, false))
        } else {
            None
        },
    }
}

fn make_exportspecifier(seed: &mut Seed, _full: bool) -> ExportSpecifier {
    ExportSpecifier {
        local: make_identifier(seed, false),
        exported: make_identifier(seed, false),
    }
}

fn make_expression(seed: &mut Seed, _full: bool) -> Expression {
    Expression::Identifier(make_identifier(seed, false))
}

fn make_expressionstatement(seed: &mut Seed, _full: bool) -> ExpressionStatement {
    ExpressionStatement {
        cv: Some(seed.id()),
        expression: make_expression(seed, false),
    }
}

fn make_forinstatement(seed: &mut Seed, _full: bool) -> ForInStatement {
    ForInStatement {
        cv: Some(seed.id()),
        left: make_forinit(seed, false),
        right: make_expression(seed, false),
        body: Box::new(make_statement(seed, false)),
    }
}

fn make_forinit(seed: &mut Seed, _full: bool) -> ForInit {
    ForInit::VariableDeclaration(make_variabledeclaration(seed, false))
}

fn make_forofstatement(seed: &mut Seed, _full: bool) -> ForOfStatement {
    ForOfStatement {
        cv: Some(seed.id()),
        left: make_forinit(seed, false),
        right: make_expression(seed, false),
        body: Box::new(make_statement(seed, false)),
    }
}

fn make_forstatement(seed: &mut Seed, full: bool) -> ForStatement {
    ForStatement {
        cv: Some(seed.id()),
        init: if full {
            Some(make_forinit(seed, false))
        } else {
            None
        },
        test: if full {
            Some(make_expression(seed, false))
        } else {
            None
        },
        update: if full {
            Some(make_expression(seed, false))
        } else {
            None
        },
        body: Box::new(make_statement(seed, false)),
    }
}

fn make_functiondeclaration(seed: &mut Seed, full: bool) -> FunctionDeclaration {
    FunctionDeclaration {
        cv: Some(seed.id()),
        id: make_identifier(seed, false),
        params: if full {
            vec![
                make_functionparam(seed, false),
                make_functionparam(seed, false),
            ]
        } else {
            vec![]
        },
        body: make_blockstatement(seed, false),
        generator: false,
        is_async: false,
    }
}

fn make_functionexpression(seed: &mut Seed, full: bool) -> FunctionExpression {
    FunctionExpression {
        cv: Some(seed.id()),
        id: if full {
            Some(make_identifier(seed, false))
        } else {
            None
        },
        params: if full {
            vec![
                make_functionparam(seed, false),
                make_functionparam(seed, false),
            ]
        } else {
            vec![]
        },
        body: make_blockstatement(seed, false),
        generator: false,
        is_async: false,
    }
}

fn make_functionparam(seed: &mut Seed, _full: bool) -> FunctionParam {
    FunctionParam::Identifier(make_identifier(seed, false))
}

fn make_identifier(seed: &mut Seed, _full: bool) -> Identifier {
    Identifier {
        cv: Some(seed.id()),
        name: "payload".into(),
    }
}

fn make_ifstatement(seed: &mut Seed, full: bool) -> IfStatement {
    IfStatement {
        cv: Some(seed.id()),
        test: make_expression(seed, false),
        consequent: Box::new(make_statement(seed, false)),
        alternate: if full {
            Some(Box::new(make_statement(seed, false)))
        } else {
            None
        },
    }
}

fn make_importdeclaration(seed: &mut Seed, full: bool) -> ImportDeclaration {
    ImportDeclaration {
        cv: Some(seed.id()),
        specifiers: if full {
            vec![
                make_importspecifier(seed, false),
                make_importspecifier(seed, false),
            ]
        } else {
            vec![]
        },
        source: make_stringliteral(seed, false),
    }
}

fn make_importexpression(seed: &mut Seed, _full: bool) -> ImportExpression {
    ImportExpression {
        cv: Some(seed.id()),
        source: Box::new(make_expression(seed, false)),
    }
}

fn make_importmeta(seed: &mut Seed, _full: bool) -> ImportMeta {
    ImportMeta {
        cv: Some(seed.id()),
    }
}

fn make_importspecifier(seed: &mut Seed, _full: bool) -> ImportSpecifier {
    ImportSpecifier::Default(make_identifier(seed, false))
}

fn make_labeledstatement(seed: &mut Seed, _full: bool) -> LabeledStatement {
    LabeledStatement {
        cv: Some(seed.id()),
        label: make_identifier(seed, false),
        body: Box::new(make_statement(seed, false)),
    }
}

fn make_logicalexpression(seed: &mut Seed, _full: bool) -> LogicalExpression {
    LogicalExpression {
        cv: Some(seed.id()),
        operator: LogicalOperator::And,
        left: Box::new(make_expression(seed, false)),
        right: Box::new(make_expression(seed, false)),
    }
}

fn make_memberexpression(seed: &mut Seed, _full: bool) -> MemberExpression {
    MemberExpression {
        cv: Some(seed.id()),
        object: Box::new(make_expression(seed, false)),
        property: Box::new(make_expression(seed, false)),
        computed: false,
    }
}

fn make_methoddefinition(seed: &mut Seed, _full: bool) -> MethodDefinition {
    MethodDefinition {
        cv: Some(seed.id()),
        key: make_propertykey(seed, false),
        kind: MethodKind::Method,
        value: make_functionexpression(seed, false),
        computed: false,
        is_static: false,
    }
}

fn make_newexpression(seed: &mut Seed, full: bool) -> NewExpression {
    NewExpression {
        cv: Some(seed.id()),
        callee: Box::new(make_expression(seed, false)),
        arguments: if full {
            vec![make_expression(seed, false), make_expression(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_newtarget(seed: &mut Seed, _full: bool) -> NewTarget {
    NewTarget {
        cv: Some(seed.id()),
    }
}

fn make_nullliteral(seed: &mut Seed, _full: bool) -> NullLiteral {
    NullLiteral {
        cv: Some(seed.id()),
    }
}

fn make_numericliteral(seed: &mut Seed, _full: bool) -> NumericLiteral {
    NumericLiteral {
        cv: Some(seed.id()),
        value: 1.25,
        raw: "payload".into(),
    }
}

fn make_objectexpression(seed: &mut Seed, full: bool) -> ObjectExpression {
    ObjectExpression {
        cv: Some(seed.id()),
        properties: if full {
            vec![
                make_objectmember(seed, false),
                make_objectmember(seed, false),
            ]
        } else {
            vec![]
        },
    }
}

fn make_objectmember(seed: &mut Seed, _full: bool) -> ObjectMember {
    ObjectMember::Property(make_property(seed, false))
}

fn make_optionalcallexpression(seed: &mut Seed, full: bool) -> OptionalCallExpression {
    OptionalCallExpression {
        cv: Some(seed.id()),
        callee: Box::new(make_expression(seed, false)),
        arguments: if full {
            vec![make_expression(seed, false), make_expression(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_optionalmemberexpression(seed: &mut Seed, _full: bool) -> OptionalMemberExpression {
    OptionalMemberExpression {
        cv: Some(seed.id()),
        object: Box::new(make_expression(seed, false)),
        property: Box::new(make_expression(seed, false)),
        computed: false,
    }
}

fn make_privatename(seed: &mut Seed, _full: bool) -> PrivateName {
    PrivateName {
        cv: Some(seed.id()),
        name: "payload".into(),
    }
}

fn make_property(seed: &mut Seed, _full: bool) -> Property {
    Property {
        cv: Some(seed.id()),
        kind: PropertyKind::Init,
        key: make_propertykey(seed, false),
        value: Box::new(make_expression(seed, false)),
        computed: false,
        shorthand: false,
        method: false,
    }
}

fn make_propertydefinition(seed: &mut Seed, full: bool) -> PropertyDefinition {
    PropertyDefinition {
        cv: Some(seed.id()),
        key: make_propertykey(seed, false),
        value: if full {
            Some(make_expression(seed, false))
        } else {
            None
        },
        computed: false,
        is_static: false,
    }
}

fn make_propertykey(seed: &mut Seed, _full: bool) -> PropertyKey {
    PropertyKey::Identifier(make_identifier(seed, false))
}

fn make_regexpliteral(seed: &mut Seed, _full: bool) -> RegExpLiteral {
    RegExpLiteral {
        cv: Some(seed.id()),
        pattern: "payload".into(),
        flags: "payload".into(),
    }
}

fn make_restelement(seed: &mut Seed, _full: bool) -> RestElement {
    RestElement {
        cv: Some(seed.id()),
        argument: make_identifier(seed, false),
    }
}

fn make_returnstatement(seed: &mut Seed, full: bool) -> ReturnStatement {
    ReturnStatement {
        cv: Some(seed.id()),
        argument: if full {
            Some(make_expression(seed, false))
        } else {
            None
        },
    }
}

fn make_sequenceexpression(seed: &mut Seed, full: bool) -> SequenceExpression {
    SequenceExpression {
        cv: Some(seed.id()),
        expressions: if full {
            vec![make_expression(seed, false), make_expression(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_spreadelement(seed: &mut Seed, _full: bool) -> SpreadElement {
    SpreadElement {
        cv: Some(seed.id()),
        argument: Box::new(make_expression(seed, false)),
    }
}

fn make_statement(seed: &mut Seed, _full: bool) -> Statement {
    Statement::Tagged(make_taggedstatement(seed, false))
}

fn make_stringliteral(seed: &mut Seed, _full: bool) -> StringLiteral {
    StringLiteral {
        cv: Some(seed.id()),
        value: "payload".into(),
        raw: "payload".into(),
    }
}

fn make_super(seed: &mut Seed, _full: bool) -> Super {
    Super {
        cv: Some(seed.id()),
    }
}

fn make_switchcase(seed: &mut Seed, full: bool) -> SwitchCase {
    SwitchCase {
        cv: Some(seed.id()),
        test: if full {
            Some(make_expression(seed, false))
        } else {
            None
        },
        consequent: if full {
            vec![make_statement(seed, false), make_statement(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_switchstatement(seed: &mut Seed, full: bool) -> SwitchStatement {
    SwitchStatement {
        cv: Some(seed.id()),
        discriminant: make_expression(seed, false),
        cases: if full {
            vec![make_switchcase(seed, false), make_switchcase(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_taggedstatement(seed: &mut Seed, _full: bool) -> TaggedStatement {
    TaggedStatement::ExpressionStatement(make_expressionstatement(seed, false))
}

fn make_taggedtemplateexpression(seed: &mut Seed, _full: bool) -> TaggedTemplateExpression {
    TaggedTemplateExpression {
        cv: Some(seed.id()),
        tag: Box::new(make_expression(seed, false)),
        quasi: make_templateliteral(seed, false),
    }
}

fn make_templateelement(seed: &mut Seed, full: bool) -> TemplateElement {
    TemplateElement {
        cv: Some(seed.id()),
        raw: "payload".into(),
        cooked: if full { Some("payload".into()) } else { None },
        tail: false,
    }
}

fn make_templateliteral(seed: &mut Seed, full: bool) -> TemplateLiteral {
    TemplateLiteral {
        cv: Some(seed.id()),
        quasis: if full {
            vec![
                make_templateelement(seed, false),
                make_templateelement(seed, false),
            ]
        } else {
            vec![]
        },
        expressions: if full {
            vec![make_expression(seed, false), make_expression(seed, false)]
        } else {
            vec![]
        },
    }
}

fn make_thisexpression(seed: &mut Seed, _full: bool) -> ThisExpression {
    ThisExpression {
        cv: Some(seed.id()),
    }
}

fn make_throwstatement(seed: &mut Seed, _full: bool) -> ThrowStatement {
    ThrowStatement {
        cv: Some(seed.id()),
        argument: make_expression(seed, false),
    }
}

fn make_trystatement(seed: &mut Seed, full: bool) -> TryStatement {
    TryStatement {
        cv: Some(seed.id()),
        block: make_blockstatement(seed, false),
        handler: if full {
            Some(make_catchclause(seed, false))
        } else {
            None
        },
        finalizer: if full {
            Some(make_blockstatement(seed, false))
        } else {
            None
        },
    }
}

fn make_unaryexpression(seed: &mut Seed, _full: bool) -> UnaryExpression {
    UnaryExpression {
        cv: Some(seed.id()),
        operator: UnaryOperator::Not,
        prefix: false,
        argument: Box::new(make_expression(seed, false)),
    }
}

fn make_undefinedliteral(seed: &mut Seed, _full: bool) -> UndefinedLiteral {
    UndefinedLiteral {
        cv: Some(seed.id()),
    }
}

fn make_updateexpression(seed: &mut Seed, _full: bool) -> UpdateExpression {
    UpdateExpression {
        cv: Some(seed.id()),
        operator: UpdateOperator::Increment,
        prefix: false,
        argument: Box::new(make_expression(seed, false)),
    }
}

fn make_variabledeclaration(seed: &mut Seed, full: bool) -> VariableDeclaration {
    VariableDeclaration {
        cv: Some(seed.id()),
        kind: VarKind::Let,
        declarations: if full {
            vec![
                make_variabledeclarator(seed, false),
                make_variabledeclarator(seed, false),
            ]
        } else {
            vec![]
        },
    }
}

fn make_variabledeclarator(seed: &mut Seed, full: bool) -> VariableDeclarator {
    VariableDeclarator {
        cv: Some(seed.id()),
        id: make_bindingtarget(seed, false),
        init: if full {
            Some(make_expression(seed, false))
        } else {
            None
        },
    }
}

fn make_whilestatement(seed: &mut Seed, _full: bool) -> WhileStatement {
    WhileStatement {
        cv: Some(seed.id()),
        test: make_expression(seed, false),
        body: Box::new(make_statement(seed, false)),
    }
}

fn make_withstatement(seed: &mut Seed, _full: bool) -> WithStatement {
    WithStatement {
        cv: Some(seed.id()),
        object: make_expression(seed, false),
        body: Box::new(make_statement(seed, false)),
    }
}

fn make_yieldexpression(seed: &mut Seed, full: bool) -> YieldExpression {
    YieldExpression {
        cv: Some(seed.id()),
        delegate: false,
        argument: if full {
            Some(Box::new(make_expression(seed, false)))
        } else {
            None
        },
    }
}

#[test]

fn c37_all_reachable_native_fields_and_variants_have_nonempty_ordered_cvs() {
    let mut cases = 0usize;

    {
        let mut seed = Seed::default();
        let node = make_arrayexpression(&mut seed, true);
        check(Frame::ArrayExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ArrowBody::Expression(Box::new(make_expression(&mut seed, true)));
        check(Frame::ArrowBody(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ArrowBody::Block(make_blockstatement(&mut seed, true));
        check(Frame::ArrowBody(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_arrowfunctionexpression(&mut seed, true);
        check(Frame::ArrowFunctionExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_assignmentexpression(&mut seed, true);
        check(Frame::AssignmentExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_assignmentpattern(&mut seed, true);
        check(Frame::AssignmentPattern(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = AssignmentTarget::Identifier(make_identifier(&mut seed, true));
        check(Frame::AssignmentTarget(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node =
            AssignmentTarget::MemberExpression(Box::new(make_memberexpression(&mut seed, true)));
        check(Frame::AssignmentTarget(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_awaitexpression(&mut seed, true);
        check(Frame::AwaitExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_bigintliteral(&mut seed, true);
        check(Frame::BigIntLiteral(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_binaryexpression(&mut seed, true);
        check(Frame::BinaryExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = BindingTarget::Identifier(make_identifier(&mut seed, true));
        check(Frame::BindingTarget(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_blockstatement(&mut seed, true);
        check(Frame::BlockStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_booleanliteral(&mut seed, true);
        check(Frame::BooleanLiteral(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_breakstatement(&mut seed, true);
        check(Frame::BreakStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_callexpression(&mut seed, true);
        check(Frame::CallExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_catchclause(&mut seed, true);
        check(Frame::CatchClause(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_chainexpression(&mut seed, true);
        check(Frame::ChainExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_classdeclaration(&mut seed, true);
        check(Frame::ClassDeclaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_classexpression(&mut seed, true);
        check(Frame::ClassExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ClassMember::Method(make_methoddefinition(&mut seed, true));
        check(Frame::ClassMember(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ClassMember::Field(make_propertydefinition(&mut seed, true));
        check(Frame::ClassMember(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ClassMember::StaticBlock(make_blockstatement(&mut seed, true));
        check(Frame::ClassMember(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_conditionalexpression(&mut seed, true);
        check(Frame::ConditionalExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_continuestatement(&mut seed, true);
        check(Frame::ContinueStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_debuggerstatement(&mut seed, true);
        check(Frame::DebuggerStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Declaration::VariableDeclaration(make_variabledeclaration(&mut seed, true));
        check(Frame::Declaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Declaration::FunctionDeclaration(make_functiondeclaration(&mut seed, true));
        check(Frame::Declaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Declaration::ClassDeclaration(make_classdeclaration(&mut seed, true));
        check(Frame::Declaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Declaration::ImportDeclaration(make_importdeclaration(&mut seed, true));
        check(Frame::Declaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node =
            Declaration::ExportNamedDeclaration(make_exportnameddeclaration(&mut seed, true));
        check(Frame::Declaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node =
            Declaration::ExportDefaultDeclaration(make_exportdefaultdeclaration(&mut seed, true));
        check(Frame::Declaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Declaration::ExportAllDeclaration(make_exportalldeclaration(&mut seed, true));
        check(Frame::Declaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_dowhilestatement(&mut seed, true);
        check(Frame::DoWhileStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_emptystatement(&mut seed, true);
        check(Frame::EmptyStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_exportalldeclaration(&mut seed, true);
        check(Frame::ExportAllDeclaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_exportdefaultdeclaration(&mut seed, true);
        check(Frame::ExportDefaultDeclaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ExportDefaultKind::Expression(Box::new(make_expression(&mut seed, true)));
        check(Frame::ExportDefaultKind(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node =
            ExportDefaultKind::FunctionDeclaration(make_functiondeclaration(&mut seed, true));
        check(Frame::ExportDefaultKind(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ExportDefaultKind::ClassDeclaration(make_classdeclaration(&mut seed, true));
        check(Frame::ExportDefaultKind(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_exportnameddeclaration(&mut seed, true);
        check(Frame::ExportNamedDeclaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_exportspecifier(&mut seed, true);
        check(Frame::ExportSpecifier(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::Identifier(make_identifier(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::NumericLiteral(make_numericliteral(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::StringLiteral(make_stringliteral(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::BooleanLiteral(make_booleanliteral(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::NullLiteral(make_nullliteral(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::BigIntLiteral(make_bigintliteral(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::UndefinedLiteral(make_undefinedliteral(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::RegExpLiteral(make_regexpliteral(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::BinaryExpression(make_binaryexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::LogicalExpression(make_logicalexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::UnaryExpression(make_unaryexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::AssignmentExpression(make_assignmentexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::ConditionalExpression(make_conditionalexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::CallExpression(make_callexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::MemberExpression(make_memberexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node =
            Expression::OptionalMemberExpression(make_optionalmemberexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::OptionalCallExpression(make_optionalcallexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::ChainExpression(make_chainexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::ArrayExpression(make_arrayexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::ObjectExpression(make_objectexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::FunctionExpression(make_functionexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node =
            Expression::ArrowFunctionExpression(make_arrowfunctionexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::ClassExpression(make_classexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::TemplateLiteral(make_templateliteral(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::UpdateExpression(make_updateexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::NewExpression(make_newexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::SequenceExpression(make_sequenceexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node =
            Expression::TaggedTemplateExpression(make_taggedtemplateexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::SpreadElement(make_spreadelement(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::YieldExpression(make_yieldexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::AwaitExpression(make_awaitexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::ThisExpression(make_thisexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::Super(make_super(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::NewTarget(make_newtarget(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::ImportMeta(make_importmeta(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Expression::ImportExpression(make_importexpression(&mut seed, true));
        check(Frame::Expression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_expressionstatement(&mut seed, true);
        check(Frame::ExpressionStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_forinstatement(&mut seed, true);
        check(Frame::ForInStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ForInit::VariableDeclaration(make_variabledeclaration(&mut seed, true));
        check(Frame::ForInit(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ForInit::Expression(make_expression(&mut seed, true));
        check(Frame::ForInit(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_forofstatement(&mut seed, true);
        check(Frame::ForOfStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_forstatement(&mut seed, true);
        check(Frame::ForStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_functiondeclaration(&mut seed, true);
        check(Frame::FunctionDeclaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_functionexpression(&mut seed, true);
        check(Frame::FunctionExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = FunctionParam::Identifier(make_identifier(&mut seed, true));
        check(Frame::FunctionParam(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = FunctionParam::RestElement(make_restelement(&mut seed, true));
        check(Frame::FunctionParam(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = FunctionParam::AssignmentPattern(make_assignmentpattern(&mut seed, true));
        check(Frame::FunctionParam(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_identifier(&mut seed, true);
        check(Frame::Identifier(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_ifstatement(&mut seed, true);
        check(Frame::IfStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_importdeclaration(&mut seed, true);
        check(Frame::ImportDeclaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_importexpression(&mut seed, true);
        check(Frame::ImportExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_importmeta(&mut seed, true);
        check(Frame::ImportMeta(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ImportSpecifier::Default(make_identifier(&mut seed, true));
        check(Frame::ImportSpecifier(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ImportSpecifier::Namespace(make_identifier(&mut seed, true));
        check(Frame::ImportSpecifier(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ImportSpecifier::Named {
            imported: make_identifier(&mut seed, true),
            local: make_identifier(&mut seed, true),
        };
        check(Frame::ImportSpecifier(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_labeledstatement(&mut seed, true);
        check(Frame::LabeledStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_logicalexpression(&mut seed, true);
        check(Frame::LogicalExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_memberexpression(&mut seed, true);
        check(Frame::MemberExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_methoddefinition(&mut seed, true);
        check(Frame::MethodDefinition(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_newexpression(&mut seed, true);
        check(Frame::NewExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_newtarget(&mut seed, true);
        check(Frame::NewTarget(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_nullliteral(&mut seed, true);
        check(Frame::NullLiteral(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_numericliteral(&mut seed, true);
        check(Frame::NumericLiteral(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_objectexpression(&mut seed, true);
        check(Frame::ObjectExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ObjectMember::Property(make_property(&mut seed, true));
        check(Frame::ObjectMember(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = ObjectMember::Spread(make_spreadelement(&mut seed, true));
        check(Frame::ObjectMember(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_optionalcallexpression(&mut seed, true);
        check(Frame::OptionalCallExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_optionalmemberexpression(&mut seed, true);
        check(Frame::OptionalMemberExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_privatename(&mut seed, true);
        check(Frame::PrivateName(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_property(&mut seed, true);
        check(Frame::Property(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_propertydefinition(&mut seed, true);
        check(Frame::PropertyDefinition(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = PropertyKey::Identifier(make_identifier(&mut seed, true));
        check(Frame::PropertyKey(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = PropertyKey::PrivateName(make_privatename(&mut seed, true));
        check(Frame::PropertyKey(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = PropertyKey::StringLiteral(make_stringliteral(&mut seed, true));
        check(Frame::PropertyKey(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = PropertyKey::NumericLiteral(make_numericliteral(&mut seed, true));
        check(Frame::PropertyKey(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = PropertyKey::Expression(Box::new(make_expression(&mut seed, true)));
        check(Frame::PropertyKey(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_regexpliteral(&mut seed, true);
        check(Frame::RegExpLiteral(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_restelement(&mut seed, true);
        check(Frame::RestElement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_returnstatement(&mut seed, true);
        check(Frame::ReturnStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_sequenceexpression(&mut seed, true);
        check(Frame::SequenceExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_spreadelement(&mut seed, true);
        check(Frame::SpreadElement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Statement::Tagged(make_taggedstatement(&mut seed, true));
        check(Frame::Statement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = Statement::Declaration(make_declaration(&mut seed, true));
        check(Frame::Statement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_stringliteral(&mut seed, true);
        check(Frame::StringLiteral(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_super(&mut seed, true);
        check(Frame::Super(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_switchcase(&mut seed, true);
        check(Frame::SwitchCase(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_switchstatement(&mut seed, true);
        check(Frame::SwitchStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::ExpressionStatement(make_expressionstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::BlockStatement(make_blockstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::IfStatement(make_ifstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::WhileStatement(make_whilestatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::DoWhileStatement(make_dowhilestatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::ForStatement(make_forstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::ForInStatement(make_forinstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::ForOfStatement(make_forofstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::ReturnStatement(make_returnstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::BreakStatement(make_breakstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::ContinueStatement(make_continuestatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::LabeledStatement(make_labeledstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::ThrowStatement(make_throwstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::SwitchStatement(make_switchstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::TryStatement(make_trystatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::EmptyStatement(make_emptystatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::DebuggerStatement(make_debuggerstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = TaggedStatement::WithStatement(make_withstatement(&mut seed, true));
        check(Frame::TaggedStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_taggedtemplateexpression(&mut seed, true);
        check(Frame::TaggedTemplateExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_templateelement(&mut seed, true);
        check(Frame::TemplateElement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_templateliteral(&mut seed, true);
        check(Frame::TemplateLiteral(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_thisexpression(&mut seed, true);
        check(Frame::ThisExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_throwstatement(&mut seed, true);
        check(Frame::ThrowStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_trystatement(&mut seed, true);
        check(Frame::TryStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_unaryexpression(&mut seed, true);
        check(Frame::UnaryExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_undefinedliteral(&mut seed, true);
        check(Frame::UndefinedLiteral(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_updateexpression(&mut seed, true);
        check(Frame::UpdateExpression(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_variabledeclaration(&mut seed, true);
        check(Frame::VariableDeclaration(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_variabledeclarator(&mut seed, true);
        check(Frame::VariableDeclarator(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_whilestatement(&mut seed, true);
        check(Frame::WhileStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_withstatement(&mut seed, true);
        check(Frame::WithStatement(&node), &seed.ids);
        cases += 1;
    }

    {
        let mut seed = Seed::default();
        let node = make_yieldexpression(&mut seed, true);
        check(Frame::YieldExpression(&node), &seed.ids);
        cases += 1;
    }

    assert_eq!(cases, 161);
}
