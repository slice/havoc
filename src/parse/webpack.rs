use std::collections::HashMap;

extern crate swc_ecma_ast as ast;

use super::ParseError;

pub type ChunkId = u32;
pub type ModuleId = u32;

pub struct WebpackChunk {
    /// The chunks that are included in this chunk script.
    pub chunks: Vec<ChunkId>,
    pub modules: HashMap<ChunkId, ()>,
    pub entrypoints: Vec<ChunkId>,
}

/// Walks a generic Webpack chunk that contains modules.
pub fn walk_webpack_chunk(script: &ast::Script) -> Result<WebpackChunk, ParseError> {
    use ParseError::MissingNode;

    // NOTE: This is the format for `webpackJsonp`/`webpackChunk`:
    //
    // webpackJsonp.push([
    //
    //   // chunk ids:
    //   [1],
    //
    //   // modules (can also be an object; indexes/keys are global IDs):
    //   [function(module, exports, require) { }, ...],
    //
    //   // entrypoints (optional; module IDs):
    //   [0]
    //
    // ]);

    let body = script.body.get(0).ok_or(MissingNode("script body"))?;

    let mut webpack_chunk = WebpackChunk {
        chunks: vec![],
        modules: HashMap::new(),
        // TODO: Handle this.
        entrypoints: vec![],
    };

    if_chain::if_chain! {
        if let ast::Stmt::Expr(ast::ExprStmt { expr: boxed_expr, .. }) = body;
        if let ast::Expr::Call(ast::CallExpr { args: call_args, .. }) = &**boxed_expr;
        if let [ast::ExprOrSpread { expr: boxed_array_expr, .. }, ..] = call_args.as_slice();
        if let ast::Expr::Array(array_lit) = &**boxed_array_expr;
        if let [chunk_ids_eos, modules_eos, ..] = array_lit.elems.as_slice();
        if let (
            Some(ast::ExprOrSpread { expr: boxed_chunk_ids_expr, .. }),
            Some(ast::ExprOrSpread { expr: boxed_modules_expr, .. })
        ) = (chunk_ids_eos, modules_eos);
        let modules_expr = &*boxed_modules_expr;

        then {
            // let mut last_module_id: ModuleId = 0;

            walk_module_listing(modules_expr, |module_id, func| {
                // if module_id - last_module_id > 1 {
                //     log::debug!("detected gap in module ids: from {} to {}", last_module_id, module_id);
                // }
                // last_module_id = module_id;
                webpack_chunk.modules.insert(module_id, ());
            });

            log::debug!("walked {} modules", webpack_chunk.modules.len());
        } else {
            // NOTE(slice): This error message isn't ideal, but the code needed
            // to achieve good error messages isn't quite ergonomic. Also, we
            // likely wouldn't get many benefits from good error messages in
            // the first place seeing as the AST structure is quite arbitrary
            // and can break at any time.
            //
            // When the AST does break, a manual reinspection is required.
            // Error messages would help a bit, but they likely wouldn't help
            // much especially after major changes.
            return Err(MissingNode("failed to walk ast"));
        }
    }

    Ok(webpack_chunk)
}

/// Walks a module listing. It can either be an array (with indexes as the
/// IDs), or an object (where the keys are the IDs).
fn walk_module_listing(modules: &ast::Expr, mut callback: impl FnMut(ModuleId, &ast::Function)) {
    match modules {
        ast::Expr::Array(ast::ArrayLit { elems, .. }) => {
            for (module_id, optional_expr_or_spread) in elems.iter().enumerate() {
                if let Some(ast::ExprOrSpread {
                    expr: boxed_expr, ..
                }) = optional_expr_or_spread
                {
                    if let ast::Expr::Fn(ast::FnExpr { function, .. }) = &**boxed_expr {
                        use std::convert::TryInto;
                        callback(
                            module_id
                                .try_into()
                                .expect("module ID couldn't fit into u32"),
                            function,
                        );
                    }
                }
            }
        }
        ast::Expr::Object(ast::ObjectLit { props, .. }) => {
            todo!("walking object module listing");
        }
        _ => {}
    }
}
