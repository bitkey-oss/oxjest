use oxc::allocator::Box;
use oxc::ast::AstBuilder;
use oxc::ast::ast::{Argument, Expression, Program, VariableDeclarationKind};
use oxc::span::{Atom, Span};
use oxc_traverse::{Traverse, TraverseCtx};

use crate::jest::is_jest_require_actual_call;

fn make_import_name(ast: AstBuilder, index: usize) -> &str {
    ast.str(&format!("__oxjest_actual_{index}__"))
}

/// Turn `jest.requireActual()` calls into dynamic imports, then hoists to the top of the module.
pub(crate) struct ImportActual<'a> {
    modules: Vec<Atom<'a>>,
}

impl ImportActual<'_> {
    pub(crate) fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }
}

impl<'a> Traverse<'a> for ImportActual<'a> {
    fn exit_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a>) {
        if self.modules.is_empty() {
            return;
        }

        // Create `const __oxjest_actual_{}__ = await import(...), ...;` declaration
        let decl = ctx.ast.declaration_variable(
            Span::default(),
            VariableDeclarationKind::Const,
            ctx.ast
                .vec_from_iter(self.modules.iter().enumerate().map(|(index, id)| {
                    let await_import = ctx.ast.expression_await(
                        Span::default(),
                        ctx.ast.expression_import(
                            Span::default(),
                            ctx.ast.expression_string_literal(Span::default(), id, None),
                            ctx.ast.vec(),
                            None,
                        ),
                    );

                    ctx.ast.variable_declarator(
                        Span::default(),
                        VariableDeclarationKind::Const,
                        ctx.ast.binding_pattern(
                            ctx.ast.binding_pattern_kind_binding_identifier(
                                Span::default(),
                                make_import_name(ctx.ast, index),
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
        node.body.insert(0, decl.into());
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, ctx: &mut TraverseCtx<'a>) {
        let Expression::CallExpression(call) = node else {
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

        *node = ctx
            .ast
            .expression_identifier(call.span, make_import_name(ctx.ast, index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transform;
    use oxc::allocator::Allocator;

    #[test]
    fn test_import_actual() {
        let source_text = r#"
        jest.unstable_mockModule("./greeter.js", () => ({
            ...jest.requireActual("./greeter.js"),
            greet: () => "Hello, world!",
        }));
        "#;

        let allocator = Allocator::new();
        let code = transform(&allocator, source_text, ImportActual::new());

        insta::assert_snapshot!(code, @r#"
        const __oxjest_actual_0__ = await import("./greeter.js");
        jest.unstable_mockModule("./greeter.js", () => ({
        	...__oxjest_actual_0__,
        	greet: () => "Hello, world!"
        }));
        "#);
    }
}
