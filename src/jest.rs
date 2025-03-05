use oxc::ast::ast::{CallExpression, Expression};

/// Checks that the expression is `jest.meta`.
fn is_import_meta(expr: &Expression) -> bool {
    const IMPORT: &str = "import";
    const META: &str = "meta";

    let Expression::StaticMemberExpression(member) = expr else {
        return false;
    };

    matches!(&member.object, Expression::Identifier(ident) if ident.name == IMPORT)
        && member.property.name == META
}

/// Checks that the expression is `jest` or `import.meta.jest`.
fn is_jest_object(expr: &Expression) -> bool {
    const JEST_OBJECT_NAME: &str = "jest";

    match expr {
        Expression::Identifier(ident) => ident.name == JEST_OBJECT_NAME,
        Expression::StaticMemberExpression(member) if is_import_meta(&member.object) => {
            member.property.name == JEST_OBJECT_NAME
        }
        _ => false,
    }
}

/// Checks that the call expression is `jest.requireActual(...)`.
pub fn is_jest_require_actual_call(expr: &CallExpression) -> bool {
    const REQUIRE_ACTUAL: &str = "requireActual";

    matches!(
        &expr.callee,
        Expression::StaticMemberExpression(callee)
            if is_jest_object(&callee.object) && callee.property.name == REQUIRE_ACTUAL
    )
}

/// Checks that the call expression is `jest.mock(...)`.
pub fn is_jest_mock_call(expr: &CallExpression) -> bool {
    const MOCK: &str = "mock";

    matches!(
        &expr.callee,
        Expression::StaticMemberExpression(callee)
            if is_jest_object(&callee.object) && callee.property.name == MOCK
    )
}

/// Checks that the call expression is `jest.unstable_mockModule(...)`.
pub fn is_jest_mock_module_call(expr: &CallExpression) -> bool {
    const MOCK_MODULE: &str = "unstable_mockModule";

    matches!(
        &expr.callee,
        Expression::StaticMemberExpression(callee)
            if is_jest_object(&callee.object) && callee.property.name == MOCK_MODULE
    )
}
