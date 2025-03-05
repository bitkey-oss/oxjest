use std::iter::once;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use oxc::allocator::{Box, CloneIn};
use oxc::ast::AstBuilder;
use oxc::ast::ast::{
    BindingPatternKind, CallExpression, ImportDeclaration, ImportDeclarationSpecifier, Program,
    Statement, VariableDeclaration, VariableDeclarationKind,
};
use oxc::ast_visit::VisitMut;
use oxc::ast_visit::walk_mut::{walk_call_expression, walk_statement};
use oxc::span::{GetSpan, Span};

use crate::jest::{is_jest_mock_call, is_jest_mock_module_call};
use crate::pass::Pass;

fn make_dynamic_import<'a>(
    ast: AstBuilder<'a>,
    decl: &ImportDeclaration<'a>,
    import_name: &str,
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
                ast.vec(),
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

struct DynamicImportsVisitor<'a> {
    ast: AstBuilder<'a>,
    is_mock_found: AtomicBool,
    import_id: AtomicUsize,
}

impl<'a> VisitMut<'a> for DynamicImportsVisitor<'a> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        walk_call_expression(self, it);

        if is_jest_mock_call(it) || is_jest_mock_module_call(it) {
            self.is_mock_found.store(true, Ordering::Release);
        }
    }

    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        walk_statement(self, it);

        let Statement::ImportDeclaration(decl) = &it else {
            return;
        };

        // Imports before any mocking don't need to be turned into dynamic imports
        if !self.is_mock_found.load(Ordering::Acquire) {
            return;
        }

        let import_id = self.import_id.fetch_add(1, Ordering::Relaxed);
        let import_name = format!("__oxjest_import_{}__", import_id);

        *it = Statement::VariableDeclaration(self.ast.alloc(make_dynamic_import(
            self.ast,
            decl,
            &import_name,
        )));
    }
}

/// Convert import declarations after module mocking to dynamic imports.
pub(crate) struct DynamicImports;

impl Pass for DynamicImports {
    fn process<'a>(&mut self, program: &mut Program<'a>, ast: AstBuilder<'a>) {
        DynamicImportsVisitor {
            ast,
            is_mock_found: AtomicBool::new(false),
            import_id: AtomicUsize::new(0),
        }
        .visit_program(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transform;

    #[test]
    fn test_dynamic_imports() {
        let source_text = r#"
        // this import will kept as-is
        import { greeter } from "./greeter.js";

        jest.unstable_mockModule("./greeter.js", () => ({
          greet: () => "Hello, world!",
        }));

        // this import needs to be turned into dynamic one to be evaluated after the mocking above
        import { greet } from "./greeter.js";
        "#;

        let code = transform(source_text, DynamicImports);

        insta::assert_snapshot!(code, @r#"
        import { greeter } from "./greeter.js";
        jest.unstable_mockModule("./greeter.js", () => ({ greet: () => "Hello, world!" }));
        const __oxjest_import_0__ = await import("./greeter.js"), greet = __oxjest_import_0__.greet;
        "#);
    }
}
