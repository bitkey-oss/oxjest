use oxc::allocator::Box;
use oxc::ast::AstBuilder;
use oxc::ast::ast::{
    Expression, IdentifierReference, ImportOrExportKind, MemberExpression, Program, Statement,
    StaticMemberExpression,
};
use oxc::span::Span;
use oxc_traverse::{Traverse, TraverseCtx};

const OXJEST_RUNTIME_ID: &str = "oxjest/runtime";
const OXJEST_RUNTIME_NAME: &str = "__oxjest__";
const JEST_OBJECT_NAME: &str = "jest";

fn make_runtime_import_stmt<'a>(ast: AstBuilder<'a>) -> Statement<'a> {
    Statement::ImportDeclaration(ast.alloc_import_declaration::<Option<Box<'a, _>>>(
        Span::default(),
        Some(
            ast.vec1(ast.import_declaration_specifier_import_namespace_specifier(
                Span::default(),
                ast.binding_identifier(Span::default(), OXJEST_RUNTIME_NAME),
            )),
        ),
        ast.string_literal(Span::default(), OXJEST_RUNTIME_ID, None),
        None,
        None,
        ImportOrExportKind::Value,
    ))
}

fn make_import_meta_jest<'a>(
    ast: AstBuilder<'a>,
    reference: &IdentifierReference<'a>,
) -> StaticMemberExpression<'a> {
    ast.static_member_expression(
        Span::default(),
        MemberExpression::StaticMemberExpression(ast.alloc_static_member_expression(
            Span::default(),
            ast.expression_identifier(Span::default(), "import"),
            ast.identifier_name(Span::default(), "meta"),
            false,
        ))
        .into(),
        ast.identifier_name(reference.span, reference.name),
        false,
    )
}

pub(crate) struct InjectGlobals {}

impl InjectGlobals {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl<'a, State> Traverse<'a, State> for InjectGlobals {
    fn exit_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a, State>) {
        node.body.insert(0, make_runtime_import_stmt(ctx.ast));
    }

    fn enter_member_expression(
        &mut self,
        node: &mut MemberExpression<'a>,
        ctx: &mut TraverseCtx<'a, State>,
    ) {
        let MemberExpression::StaticMemberExpression(expr) = node else {
            return;
        };

        let Expression::Identifier(ident) = &expr.object else {
            return;
        };

        if ident.name != JEST_OBJECT_NAME {
            return;
        }

        expr.object = MemberExpression::StaticMemberExpression(
            ctx.ast.alloc(make_import_meta_jest(ctx.ast, ident)),
        )
        .into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transform;
    use oxc::allocator::Allocator;

    #[test]
    fn test_inject_globals() {
        let source_text = r#"
        // needs to be import.meta.jest
        jest.mock("./greeter.js");
        "#;

        let allocator = Allocator::new();
        let code = transform(&allocator, source_text, InjectGlobals::new());

        insta::assert_snapshot!(code, @r#"
        import * as __oxjest__ from "oxjest/runtime";
        // needs to be import.meta.jest
        import.meta.jest.mock("./greeter.js");
        "#);
    }
}
