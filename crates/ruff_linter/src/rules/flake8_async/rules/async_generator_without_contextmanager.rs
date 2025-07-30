use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::{self as ast, Decorator};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for async generators that are not decorated with `@asynccontextmanager`
/// or other configured safe decorators.
///
/// ## Why is this bad?
/// Async generators are inherently unsafe due to potential control-flow problems
/// (see [PEP 789]) and delayed cleanup problems (see [PEP 533]). Using
/// `@asynccontextmanager` or similar decorators provides safer cleanup semantics.
///
/// ## Example
/// ```python
/// async def get_data():
///     resource = acquire_resource()
///     yield resource
///     release_resource(resource)
/// ```
///
/// Use instead:
/// ```python
/// from contextlib import asynccontextmanager
///
/// @asynccontextmanager
/// async def get_data():
///     resource = acquire_resource()
///     yield resource
///     release_resource(resource)
/// ```
///
/// ## Options
/// - `flake8-async.transform-async-generator-decorators`: A list of additional
///   decorators that should be treated as safe for async generators (e.g.,
///   `["trio_util.trio_async_generator"]`).
///
/// [PEP 533]: https://peps.python.org/pep-0533/
/// [PEP 789]: https://peps.python.org/pep-0789/
#[derive(ViolationMetadata)]
pub(crate) struct AsyncGeneratorWithoutContextmanager;

impl crate::Violation for AsyncGeneratorWithoutContextmanager {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Async generator without `@asynccontextmanager` not allowed".to_string()
    }
}

/// ASYNC900
pub(crate) fn async_generator_without_contextmanager(
    checker: &Checker,
    function_def: &ast::StmtFunctionDef,
) {
    // Only check async functions
    if !function_def.is_async {
        return;
    }

    // Check if the function is an async generator (contains yield)
    if !is_async_generator(function_def) {
        return;
    }

    // Check if the function has a safe decorator
    if has_safe_decorator(function_def, checker) {
        return;
    }

    // Report the violation
    checker.report_diagnostic(
        AsyncGeneratorWithoutContextmanager,
        function_def.name.range(),
    );
}

/// Check if a function is an async generator (contains yield statements)
fn is_async_generator(function_def: &ast::StmtFunctionDef) -> bool {
    use ruff_python_ast::visitor::Visitor;
    use ruff_python_ast::{self as ast};
    
    struct YieldVisitor {
        has_yield: bool,
    }
    
    impl<'a> Visitor<'a> for YieldVisitor {
        fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
            // Don't descend into nested functions
            match stmt {
                ast::Stmt::FunctionDef(_) => return,
                _ => {
                    ruff_python_ast::visitor::walk_stmt(self, stmt);
                }
            }
        }
        
        fn visit_expr(&mut self, expr: &'a ast::Expr) {
            match expr {
                ast::Expr::Yield(_) | ast::Expr::YieldFrom(_) => {
                    self.has_yield = true;
                }
                _ => {
                    ruff_python_ast::visitor::walk_expr(self, expr);
                }
            }
        }
    }
    
    let mut visitor = YieldVisitor { has_yield: false };
    for stmt in &function_def.body {
        visitor.visit_stmt(stmt);
        if visitor.has_yield {
            return true;
        }
    }
    false
}

/// Check if a function has a safe decorator for async generators
fn has_safe_decorator(function_def: &ast::StmtFunctionDef, checker: &Checker) -> bool {
    // Default safe decorators
    let safe_decorators = vec![
        vec!["contextlib", "asynccontextmanager"],
        vec!["pytest", "fixture"],
    ];

    for decorator in &function_def.decorator_list {
        if is_safe_decorator(decorator, &safe_decorators, checker) {
            return true;
        }
    }

    false
}

/// Check if a decorator is in the list of safe decorators
fn is_safe_decorator(
    decorator: &Decorator,
    safe_decorators: &[Vec<&str>],
    checker: &Checker,
) -> bool {
    // Get the base expression - if it's a call, get the function being called
    let base_expr = match &decorator.expression {
        ast::Expr::Call(call) => &*call.func,
        other => other,
    };
    
    let Some(qualified_name) = checker
        .semantic()
        .resolve_qualified_name(base_expr)
    else {
        return false;
    };

    for safe_decorator in safe_decorators {
        if qualified_name.segments() == safe_decorator.as_slice() {
            return true;
        }
    }

    false
}