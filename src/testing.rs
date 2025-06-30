use std::path::PathBuf;
use std::str::FromStr;

use oxc::allocator::Allocator;
use oxc::codegen::{Codegen, CodegenReturn};
use oxc_traverse::{Traverse, traverse_mut};

use crate::loader::Loader;

pub(crate) fn transform<'a>(
    allocator: &'a Allocator,
    source_text: &str,
    mut traverser: impl Traverse<'a, ()>,
) -> String {
    let source_path = PathBuf::from_str("/path/to/source.js").unwrap();

    let source_text = allocator.alloc_str(source_text);
    let (mut program, scoping) = Loader
        .load_str(&allocator, source_text, &source_path)
        .unwrap();

    traverse_mut(&mut traverser, &allocator, &mut program, scoping, ());

    let CodegenReturn { code, .. } = Codegen::new()
        .with_options(Default::default())
        .build(&program);

    code
}
