use std::path::Path;

use oxc::allocator::Allocator;
use oxc::ast::ast::Program;
use oxc::diagnostics::OxcDiagnostic;
use oxc::parser::{Parser, ParserReturn};
use oxc::semantic::{SemanticBuilder, SemanticBuilderReturn};
use oxc::span::SourceType;
use oxc::transformer::{DecoratorOptions, TransformOptions, Transformer, TransformerReturn};

pub struct Loader;

impl Loader {
    pub fn load_str<'a>(
        &self,
        allocator: &'a Allocator,
        source_text: &'a str,
        source_path: impl AsRef<Path>,
    ) -> Result<Program<'a>, Vec<OxcDiagnostic>> {
        let source_path = source_path.as_ref();
        let source_type = SourceType::from_path(source_path).unwrap();

        let ParserReturn {
            mut program,
            errors,
            ..
        } = Parser::new(allocator, source_text, source_type).parse();
        if !errors.is_empty() {
            return Err(errors);
        }

        let SemanticBuilderReturn { semantic, errors } = SemanticBuilder::new()
            .with_excess_capacity(2.0)
            .build(&program);
        if !errors.is_empty() {
            return Err(errors);
        }

        let (symbols, scopes) = semantic.into_symbol_table_and_scope_tree();
        let options = TransformOptions {
            decorator: DecoratorOptions {
                legacy: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let TransformerReturn { errors, .. } = Transformer::new(allocator, source_path, &options)
            .build_with_symbols_and_scopes(symbols, scopes, &mut program);
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(program)
    }
}
