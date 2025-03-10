use oxc::allocator::Vec;
use oxc::ast::ast::{Expression, MemberExpression, Program, Statement};
use oxc_traverse::{Traverse, TraverseCtx};

pub(crate) mod convert_mocks;
pub(crate) mod import_actual;
pub(crate) mod inject_globals;

/// The facade of all transforms combined into one.
/// Be careful with calling order when adding a new transform.
pub(crate) struct Transformer<'a> {
    convert_mocks: convert_mocks::ConvertMocks<'a>,
    import_actual: import_actual::ImportActual<'a>,
    inject_globals: inject_globals::InjectGlobals,
}

impl Transformer<'_> {
    pub(crate) fn new() -> Self {
        Self {
            convert_mocks: convert_mocks::ConvertMocks::new(),
            import_actual: import_actual::ImportActual::new(),
            inject_globals: inject_globals::InjectGlobals::new(),
        }
    }
}

impl<'a> Traverse<'a> for Transformer<'a> {
    fn exit_program(&mut self, node: &mut Program<'a>, ctx: &mut TraverseCtx<'a>) {
        self.convert_mocks.exit_program(node, ctx);
        self.import_actual.exit_program(node, ctx);
        self.inject_globals.exit_program(node, ctx);
    }

    fn exit_expression(&mut self, node: &mut Expression<'a>, ctx: &mut TraverseCtx<'a>) {
        self.convert_mocks.exit_expression(node, ctx);
        self.import_actual.exit_expression(node, ctx);
    }

    fn enter_member_expression(
        &mut self,
        node: &mut MemberExpression<'a>,
        ctx: &mut TraverseCtx<'a>,
    ) {
        self.inject_globals.enter_member_expression(node, ctx);
    }

    fn exit_statements(&mut self, node: &mut Vec<'a, Statement<'a>>, ctx: &mut TraverseCtx<'a>) {
        self.convert_mocks.exit_statements(node, ctx);
    }
}
