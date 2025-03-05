mod jest;
mod loader;
mod pass;
mod transform;

#[cfg(test)]
mod testing;

use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
pub struct TransformOptions<'scope> {
    pub resolver: Function<'scope, (String, String), String>,
}

#[napi(object)]
pub struct TransformedSource {
    pub code: String,
    pub map: String,
}

#[napi]
pub fn transform(source_text: String, source_path: String) -> Result<TransformedSource> {
    transform::_transform(source_text, source_path)
}
