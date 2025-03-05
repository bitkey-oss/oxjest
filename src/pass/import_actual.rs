use oxc::allocator::{Box, Vec};
use oxc::ast::AstBuilder;
use oxc::ast::ast::{Argument, Expression, Program, VariableDeclarationKind};
use oxc::ast_visit::VisitMut;
use oxc::ast_visit::walk_mut::{walk_expression, walk_program};
use oxc::span::{Atom, Span};

use crate::jest::is_jest_require_actual_call;
use crate::pass::Pass;

fn make_import_name(ast: AstBuilder, index: usize) -> &str {
    ast.str(&format!("__oxjest_actual_{index}__"))
}

struct ImportActualVisitor<'a> {
    ast: AstBuilder<'a>,
    modules: Vec<'a, Atom<'a>>,
}

impl<'a> VisitMut<'a> for ImportActualVisitor<'a> {
    fn visit_program(&mut self, it: &mut Program<'a>) {
        // Visit every statement in the program first.
        walk_program(self, it);

        if self.modules.is_empty() {
            return;
        }

        // Create `const __oxjest_actual_{}__ = await import(...), ...;` declaration
        let decl = self.ast.declaration_variable(
            Span::default(),
            VariableDeclarationKind::Const,
            self.ast
                .vec_from_iter(self.modules.iter().enumerate().map(|(index, id)| {
                    let await_import = self.ast.expression_await(
                        Span::default(),
                        self.ast.expression_import(
                            Span::default(),
                            self.ast
                                .expression_string_literal(Span::default(), id, None),
                            self.ast.vec(),
                            None,
                        ),
                    );

                    self.ast.variable_declarator(
                        Span::default(),
                        VariableDeclarationKind::Const,
                        self.ast.binding_pattern(
                            self.ast.binding_pattern_kind_binding_identifier(
                                Span::default(),
                                make_import_name(self.ast, index),
                            ),
                            Option::<Box<'a, _>>::None,
                            false,
                        ),
                        Some(await_import),
                        false,
                    )
                })),
            false,
        );

        // Inject the declaration at the top of the module
        it.body.insert(0, decl.into());
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_expression(self, it);

        let Expression::CallExpression(call) = it else {
            return;
        };

        if !is_jest_require_actual_call(call) {
            return;
        }

        // jest.requireActual("<id>");
        let Some(Argument::StringLiteral(lit)) = call.arguments.first() else {
            return;
        };

        let index = self.modules.len();
        self.modules.push(lit.value);

        *it = self
            .ast
            .expression_identifier(call.span, make_import_name(self.ast, index))
    }
}

/// Turn `jest.requireActual()` calls into dynamic imports, then hoists to the top of the module.
pub struct ImportActual;

impl Pass for ImportActual {
    fn process<'a>(&mut self, program: &mut Program<'a>, ast: AstBuilder<'a>) {
        ImportActualVisitor {
            ast,
            modules: ast.vec(),
        }
        .visit_program(program);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transform;

    #[test]
    fn test_import_actual() {
        let source_text = r#"
        jest.unstable_mockModule("./greeter.js", () => ({
            ...jest.requireActual("./greeter.js"),
            greet: () => "Hello, world!",
        }));
        "#;

        let code = transform(source_text, ImportActual);

        insta::assert_snapshot!(code, @r#"
        const __oxjest_actual_0__ = await import("./greeter.js");
        jest.unstable_mockModule("./greeter.js", () => ({
        	...__oxjest_actual_0__,
        	greet: () => "Hello, world!"
        }));
        "#);
    }
}
