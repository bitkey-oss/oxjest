use oxc::allocator::{Box, CloneIn, Vec};
use oxc::ast::AstBuilder;
use oxc::ast::ast::{Argument, Expression, Program, Span, Statement};
use oxc::ast_visit::VisitMut;
use oxc::ast_visit::walk_mut::{walk_expression, walk_program, walk_statements};

use crate::jest::is_jest_mock_call;
use crate::pass::Pass;

fn make_create_mock_factory<'a>(ast: AstBuilder<'a>, id: &str) -> Expression<'a> {
    ast.expression_call(
        Span::default(),
        ast.member_expression_static(
            Span::default(),
            ast.expression_identifier(Span::default(), "__oxjest__"),
            ast.identifier_name(Span::default(), "createMockFactory"),
            false,
        )
        .into(),
        Option::<Box<'_, _>>::None,
        ast.vec1(
            ast.expression_await(
                Span::default(),
                ast.expression_import(
                    Span::default(),
                    ast.expression_string_literal(Span::default(), id, None),
                    ast.vec(),
                    None,
                ),
            )
            .into(),
        ),
        false,
    )
}

struct HoistMocksVisitor<'a> {
    ast: AstBuilder<'a>,
    mocks: Vec<'a, Expression<'a>>,
}

impl<'a> VisitMut<'a> for HoistMocksVisitor<'a> {
    fn visit_program(&mut self, it: &mut Program<'a>) {
        walk_program(self, it);

        it.body.splice(
            0..0,
            self.mocks.iter().map(|mock_call| {
                self.ast
                    .statement_expression(Span::default(), mock_call.clone_in(self.ast.allocator))
            }),
        );
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_expression(self, it);

        let Expression::CallExpression(call) = it else {
            return;
        };

        if !is_jest_mock_call(call) {
            return;
        }

        let Expression::StaticMemberExpression(member) = &mut call.callee else {
            // SAFETY: Already checked above
            unreachable!();
        };

        member.property.name = self.ast.atom("unstable_mockModule");

        if call.arguments.len() < 2 {
            let Some(Argument::StringLiteral(lit)) = call.arguments.first() else {
                return;
            };

            let id = lit.value.as_str();

            call.arguments
                .push(make_create_mock_factory(self.ast, id).into())
        }

        self.mocks.push(self.ast.move_expression(it));
    }

    fn visit_statements(&mut self, it: &mut Vec<'a, Statement<'a>>) {
        walk_statements(self, it);

        it.retain(|stmt| !matches!(stmt, Statement::ExpressionStatement(stmt) if stmt.expression.is_null()));
    }
}

pub(crate) struct HoistMocks;

impl Pass for HoistMocks {
    fn process<'a>(&mut self, program: &mut Program<'a>, ast: AstBuilder<'a>) {
        HoistMocksVisitor {
            ast,
            mocks: Vec::new_in(ast.allocator),
        }
        .visit_program(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transform;

    #[test]
    fn test_hoist_mocks() {
        let source_text = r#"
        import { greet } from "./greeter.js";

        // this mocking needs to be hoisted to the top of this module
        jest.mock("./greeter.js", () => ({
          greet: () => "Hello, world!",
        }));
        "#;

        let code = transform(source_text, HoistMocks);

        insta::assert_snapshot!(code, @r#"
        jest.unstable_mockModule("./greeter.js", () => ({ greet: () => "Hello, world!" }));
        import { greet } from "./greeter.js";
        "#);
    }
}
