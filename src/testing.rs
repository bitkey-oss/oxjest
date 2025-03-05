use std::path::PathBuf;
use std::str::FromStr;

use oxc::allocator::Allocator;
use oxc::ast::AstBuilder;
use oxc::codegen::{CodeGenerator, CodegenReturn};

use crate::loader::Loader;
use crate::pass::Pass;

pub(crate) fn transform<'a, P: Pass>(source_text: &str, mut pass: P) -> String {
    let source_path = PathBuf::from_str("/path/to/source.js").unwrap();

    let allocator = Allocator::new();
    let mut program = Loader
        .load_str(&allocator, source_text, &source_path)
        .unwrap();

    let ast = AstBuilder::new(&allocator);

    pass.process(&mut program, ast);

    let CodegenReturn { code, .. } = CodeGenerator::new()
        .with_options(Default::default())
        .build(&program);

    code
}
