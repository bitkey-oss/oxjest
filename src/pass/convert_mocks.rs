use std::iter::once;
use std::sync::atomic::{AtomicUsize, Ordering};

use oxc::allocator::{Box, CloneIn, TakeIn, Vec as ArenaVec};
use oxc::ast::AstBuilder;
use oxc::ast::ast::{
    Argument, BindingPatternKind, Expression, ImportDeclaration, ImportDeclarationSpecifier,
    Program, Span, Statement, VariableDeclaration, VariableDeclarationKind,
};
use oxc::span::GetSpan;
use oxc_traverse::{Traverse, TraverseCtx};

use crate::jest::is_jest_do_mock_call;
use crate::jest::is_jest_mock_call;

fn make_create_mock_factory<'a>(ast: AstBuilder<'a>, id: &'a str) -> Expression<'a> {
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
                    None,
                    None,
                ),
            )
            .into(),
        ),
        false,
    )
}

fn make_dynamic_import<'a>(
    ast: AstBuilder<'a>,
    decl: &ImportDeclaration<'a>,
    import_name: &'a str,
) -> VariableDeclaration<'a> {
    // __oxjest_import_{}__ = await import("...")
    let await_import = ast.variable_declarator(
        Span::default(),
        VariableDeclarationKind::Const,
        ast.binding_pattern(
            ast.binding_pattern_kind_binding_identifier(Span::default(), import_name),
            Option::<Box<'a, _>>::None,
            false,
        ),
        Some(ast.expression_await(
            Span::default(),
            ast.expression_import(
                Span::default(),
                ast.expression_string_literal(decl.source.span, decl.source.value, decl.source.raw),
                None,
                None,
            ),
        )),
        false,
    );

    // foo = __oxjest_import_{}__.foo, bar = __oxjest_import_{}__.default, ...
    let declarations = decl.specifiers.iter().flatten().map(|specifier| {
        ast.variable_declarator(
            Span::default(),
            VariableDeclarationKind::Const,
            ast.binding_pattern(
                BindingPatternKind::BindingIdentifier(
                    ast.alloc(specifier.local().clone_in(ast.allocator)),
                ),
                Option::<Box<'a, _>>::None,
                false,
            ),
            Some(match specifier {
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                    ast.expression_identifier(Span::default(), import_name)
                }
                _ => ast
                    .member_expression_static(
                        Span::default(),
                        ast.expression_identifier(Span::default(), import_name),
                        match specifier {
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                                ast.identifier_name(specifier.span, "default")
                            }
                            ImportDeclarationSpecifier::ImportSpecifier(specifier) => ast
                                .identifier_name(
                                    specifier.imported.span(),
                                    specifier.imported.name(),
                                ),
                            _ => unreachable!(),
                        },
                        false,
                    )
                    .into(),
            }),
            false,
        )
    });

    // const __oxjest_import_{}__ = await import("..."), foo = __oxjest_import_{}__.foo, ...;
    ast.variable_declaration(
        decl.span,
        VariableDeclarationKind::Const,
        ast.vec_from_iter(once(await_import).chain(declarations)),
        false,
    )
}

pub(crate) struct ConvertMocks<'a> {
    mocks: Vec<Expression<'a>>,
}

impl ConvertMocks<'_> {
    pub(crate) fn new() -> Self {
        Self { mocks: Vec::new() }
    }
}

impl<'a, State> Traverse<'a, State> for ConvertMocks<'a> {
    fn exit_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a, State>) {
        // Insert hoisted mocks at the top of the body
        node.body.splice(
            0..0,
            self.mocks.iter().map(|mock_call| {
                ctx.ast
                    .statement_expression(Span::default(), mock_call.clone_in(ctx.ast.allocator))
            }),
        );

        // Imports don't need to be turned into dynamic imports if there are no mocks
        if !self.mocks.is_empty() {
            let import_id = AtomicUsize::new(0);

            node.body.iter_mut().for_each(|stmt| {
                let Statement::ImportDeclaration(decl) = stmt else {
                    return;
                };

                let import_id = import_id.fetch_add(1, Ordering::Relaxed);
                let import_name = format!("__oxjest_import_{import_id}__");

                *stmt = Statement::VariableDeclaration(ctx.ast.alloc(make_dynamic_import(
                    ctx.ast,
                    decl,
                    ctx.ast.str(&import_name),
                )));
            })
        }
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, ctx: &mut TraverseCtx<'a, State>) {
        let Expression::CallExpression(call) = node else {
            return;
        };

        let is_jest_mock_call = is_jest_mock_call(call);
        let is_jest_do_mock_call = is_jest_do_mock_call(call);
        if !is_jest_mock_call && !is_jest_do_mock_call {
            return;
        }

        let Expression::StaticMemberExpression(member) = &mut call.callee else {
            // SAFETY: Already checked above
            unreachable!();
        };

        member.property.name = ctx.ast.atom("unstable_mockModule");

        if call.arguments.len() < 2 {
            let Some(Argument::StringLiteral(lit)) = call.arguments.first() else {
                return;
            };

            let id = lit.value.as_str();

            call.arguments
                .push(make_create_mock_factory(ctx.ast, id).into())
        }

        // only jest.mock needs to be hoisted
        if is_jest_mock_call {
            self.mocks.push(node.take_in(ctx.ast.allocator));
        }
    }

    fn exit_statements(
        &mut self,
        node: &mut ArenaVec<'a, Statement<'a>>,
        _ctx: &mut TraverseCtx<'a, State>,
    ) {
        node.retain(|stmt| !matches!(stmt, Statement::ExpressionStatement(stmt) if stmt.expression.is_null()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transform;
    use oxc::allocator::Allocator;

    #[test]
    fn test_mock() {
        let source_text = r#"
        import { greet } from "./greeter.js";

        // this mocking needs to be hoisted to the top of this module
        jest.mock("./greeter.js", () => ({
          greet: () => "Hello, world!",
        }));
        "#;

        let allocator = Allocator::new();
        let code = transform(&allocator, source_text, ConvertMocks::new());

        insta::assert_snapshot!(code, @r#"
        jest.unstable_mockModule("./greeter.js", () => ({ greet: () => "Hello, world!" }));
        const __oxjest_import_0__ = await import("./greeter.js"), greet = __oxjest_import_0__.greet;
        "#);
    }

    #[test]
    fn test_mock_auto() {
        let source_text = r#"
        import { greet } from "./greeter.js";

        // this mocking needs to be hoisted to the top of this module
        jest.mock("./greeter.js");
        "#;

        let allocator = Allocator::new();
        let code = transform(&allocator, source_text, ConvertMocks::new());

        insta::assert_snapshot!(code, @r#"
        jest.unstable_mockModule("./greeter.js", __oxjest__.createMockFactory(await import("./greeter.js")));
        const __oxjest_import_0__ = await import("./greeter.js"), greet = __oxjest_import_0__.greet;
        "#);
    }

    #[test]
    fn test_do_mock() {
        let source_text = r#"
        import { greet } from "./greeter.js";

        // this mocking does not need to be hoisted
        jest.doMock("./greeter.js", () => ({
          greet: () => "Hello, world!",
        }));
        "#;

        let allocator = Allocator::new();
        let code = transform(&allocator, source_text, ConvertMocks::new());

        insta::assert_snapshot!(code, @r#"
        import { greet } from "./greeter.js";
        // this mocking does not need to be hoisted
        jest.unstable_mockModule("./greeter.js", () => ({ greet: () => "Hello, world!" }));
        "#);
    }

    #[test]
    fn test_do_mock_auto() {
        let source_text = r#"
        import { greet } from "./greeter.js";

        // this mocking does not need to be hoisted
        jest.doMock("./greeter.js");
        "#;

        let allocator = Allocator::new();
        let code = transform(&allocator, source_text, ConvertMocks::new());

        insta::assert_snapshot!(code, @r#"
        import { greet } from "./greeter.js";
        // this mocking does not need to be hoisted
        jest.unstable_mockModule("./greeter.js", __oxjest__.createMockFactory(await import("./greeter.js")));
        "#);
    }
}
