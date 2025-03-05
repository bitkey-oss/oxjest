use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use napi::bindgen_prelude::*;
use oxc::allocator::Allocator;
use oxc::codegen::{CodeGenerator, CodegenOptions, CodegenReturn};
use oxc_sourcemap::SourceMap;
use oxc_traverse::traverse_mut;

use crate::loader::Loader;
use crate::pass::Transformer;

pub(crate) fn _transform(
    source_text: String,
    source_path: String,
) -> Result<crate::TransformedSource> {
    let source_path = PathBuf::from(source_path);
    let allocator = Allocator::new();
    let (mut program, symbols, scopes) = Loader
        .load_str(&allocator, &source_text, &source_path)
        .map_err(|_| Error::from_reason("Could not load a source file. Invalid syntax?"))?;

    let mut transformer = Transformer::new();

    traverse_mut(&mut transformer, &allocator, &mut program, symbols, scopes);

    let CodegenReturn { mut code, map, .. } = CodeGenerator::new()
        .with_options(CodegenOptions {
            source_map_path: Some(source_path.clone()),
            ..Default::default()
        })
        .build(&program);

    // SAFETY: map is always Some(_) if CodegenOptions.source_map_path is Some(_)
    let map = map.unwrap();

    // Remove sourcesContent from the sourcemap as larger sources can lead OOM on Node.js
    let map = SourceMap::new(
        map.get_file().map(Arc::from),
        map.get_names().map(Arc::from).collect(),
        map.get_source_root().map(String::from),
        map.get_sources().map(Arc::from).collect(),
        None,
        map.get_tokens().cloned().collect(),
        None,
    );

    let map = map.to_json_string();

    // Append the source map to the code for better compatibility
    // https://github.com/swc-project/swc/blob/b22d7ee3ab8ee0a6dd521298237c42633137c633/crates/swc_compiler_base/src/lib.rs#L264
    code.push_str("\n//# sourceMappingURL=data:application/json;base64,");
    BASE64_STANDARD.encode_string(map.as_bytes(), &mut code);

    Ok(crate::TransformedSource { code, map })
}
