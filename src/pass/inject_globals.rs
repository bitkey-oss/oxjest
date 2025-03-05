use oxc::allocator::Box;
use oxc::ast::AstBuilder;
use oxc::ast::ast::*;
use oxc::ast_visit::VisitMut;
use oxc::ast_visit::walk_mut::{walk_member_expression, walk_program};
use oxc::span::Span;

use crate::pass::Pass;

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

struct InjectGlobalsVisitor<'a> {
    ast: AstBuilder<'a>,
}

impl<'a> VisitMut<'a> for InjectGlobalsVisitor<'a> {
    fn visit_program(&mut self, it: &mut Program<'a>) {
        walk_program(self, it);

        it.body.insert(0, make_runtime_import_stmt(self.ast));
    }

    fn visit_member_expression(&mut self, it: &mut MemberExpression<'a>) {
        walk_member_expression(self, it);

        let MemberExpression::StaticMemberExpression(expr) = it else {
            return;
        };

        let Expression::Identifier(ident) = &expr.object else {
            return;
        };

        if ident.name != JEST_OBJECT_NAME {
            return;
        }

        expr.object = MemberExpression::StaticMemberExpression(
            self.ast.alloc(make_import_meta_jest(self.ast, ident)),
        )
        .into();
    }
}

pub(crate) struct InjectGlobals;

impl Pass for InjectGlobals {
    fn process<'a>(&mut self, program: &mut Program<'a>, ast: AstBuilder<'a>) {
        let mut visitor = InjectGlobalsVisitor { ast };

        visitor.visit_program(program);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transform;

    #[test]
    fn test_inject_globals() {
        let source_text = r#"
        // needs to be import.meta.jest
        jest.mock("./greeter.js");
        "#;

        let code = transform(source_text, InjectGlobals);

        insta::assert_snapshot!(code, @r#"
        import * as __oxjest__ from "oxjest/runtime";
        import.meta.jest.mock("./greeter.js");
        "#);
    }
}
