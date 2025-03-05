use oxc::ast::AstBuilder;
use oxc::ast::ast::Program;

pub(crate) mod dynamic_imports;
pub(crate) mod hoist_mocks;
pub(crate) mod import_actual;
pub(crate) mod inject_globals;

pub(crate) trait Pass {
    fn process<'a>(&mut self, program: &mut Program<'a>, ast: AstBuilder<'a>);
}
