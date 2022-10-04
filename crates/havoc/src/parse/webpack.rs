use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use serde::Serialize;
extern crate swc_ecma_ast as ast;

use super::ParseError;

/// A webpack chunk ID.
pub type ChunkId = u32;

/// A webpack module ID.
///
/// Webpack module IDs are, in practice, globally unique.
pub type ModuleId = u32;

/// A webpack chunk.
///
/// A chunk is a file that encapsulates multiple modules. It can be loaded
/// asynchronously through the chunk loader.
pub struct WebpackChunk<'a> {
    /// The chunks IDs included in this script.
    pub chunks: Vec<ChunkId>,

    /// The modules within this chunk.
    pub modules: HashMap<ChunkId, WebpackModule<'a>>,

    /// The entrypoint modules within this chunk.
    pub entrypoints: Vec<ChunkId>,
}

/// A function-like AST node.
///
/// This is needed because functions and arrow expressions have distinct
/// representations under swt's model of the ECMAScript AST.
#[derive(Debug, Serialize)]
pub enum FunctionLike<'a> {
    Function(&'a ast::Function),
    Arrow(&'a ast::ArrowExpr),
}

impl FunctionLike<'_> {
    /// Returns the span for this function-like AST node.
    pub fn span(&self) -> swc_common::Span {
        match self {
            FunctionLike::Function(function) => function.span,
            FunctionLike::Arrow(arrow_expr) => arrow_expr.span,
        }
    }
}

/// A fallible conversion from an AST expression into a `FunctionLike`.
impl<'a> TryFrom<&'a ast::Expr> for FunctionLike<'a> {
    type Error = ();

    fn try_from(expr: &'a ast::Expr) -> Result<Self, Self::Error> {
        match expr {
            ast::Expr::Fn(ast::FnExpr { function, .. }) => Ok(FunctionLike::Function(function)),
            ast::Expr::Arrow(arrow_expr) => Ok(FunctionLike::Arrow(arrow_expr)),
            _ => Err(()),
        }
    }
}

/// A webpack module.
#[derive(Serialize)]
pub struct WebpackModule<'a> {
    /// The module's ID.
    pub id: ChunkId,

    /// The module's function.
    pub func: FunctionLike<'a>,
}

/// Walks a generic Webpack chunk that contains modules.
pub fn walk_webpack_chunk(script: &ast::Script) -> Result<WebpackChunk, ParseError> {
    use ParseError::MissingNode;

    let span = tracing::info_span!("webpack_chunk_walking");
    let _enter = span.enter();

    // NOTE: This is the format for `webpackJsonp`/`webpackChunk`:
    //
    // webpackJsonp.push([
    //
    //   // chunk IDs:
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
        // the first expression is the webpackJsonp.push call
        if let ast::Stmt::Expr(ast::ExprStmt { expr: boxed_expr, .. }) = body;
        if let ast::Expr::Call(ast::CallExpr { args: call_args, .. }) = &**boxed_expr;

        // the first argument is an array
        if let [ast::ExprOrSpread { expr: boxed_array_expr, .. }, ..] = call_args.as_slice();
        if let ast::Expr::Array(array_lit) = &**boxed_array_expr;

        // the elements of the array
        if let [chunk_ids_eos, modules_eos, ..] = array_lit.elems.as_slice();
        if let (
            Some(ast::ExprOrSpread { expr: _boxed_chunk_ids_expr, .. }),
            Some(ast::ExprOrSpread { expr: boxed_modules_expr, .. })
        ) = (chunk_ids_eos, modules_eos);
        let modules_expr = boxed_modules_expr;

        then {
            for (module_id, func) in walk_module_listing(modules_expr) {
                let span = func.span();
                let module = WebpackModule { id: module_id, func };
                tracing::trace!("found module {} (span: {} to {}, len: {})", module_id, span.lo.0, span.hi.0, span.hi.0 - span.lo.0);
                webpack_chunk.modules.insert(module_id, module);
            }

            tracing::info!("walked {} modules", webpack_chunk.modules.len());
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

/// Walks a module listing expression.
///
/// The expression can either be an array or an object. If it is neither, then
/// this function panics. Modules' identifiers come from their array indices
/// or their object keys.
fn walk_module_listing<'script>(
    modules: &'script ast::Expr,
) -> Box<dyn Iterator<Item = (ModuleId, FunctionLike<'script>)> + 'script> {
    match modules {
        ast::Expr::Array(ast::ArrayLit { elems, .. }) => {
            Box::new(elems.iter().enumerate().filter_map(|(module_id, optional_expr_or_spread)| {
                if_chain::if_chain! {
                    if let Some(ast::ExprOrSpread { expr: boxed_expr, .. }) = optional_expr_or_spread;
                    if let Ok(function_like) = (&**boxed_expr).try_into();

                    then {
                        let module_id: u32 = module_id.try_into().expect("module ID couldn't fit into u32");
                        Some((module_id, function_like))
                    } else {
                        None
                    }
                }
            }))
        }
        ast::Expr::Object(ast::ObjectLit { props, .. }) => {
            Box::new(props.iter().filter_map(|prop_or_spread| {
              if_chain::if_chain! {
                    if let ast::PropOrSpread::Prop(boxed_prop) = prop_or_spread;
                    if let ast::Prop::KeyValue(ast::KeyValueProp { key, value: boxed_value }) = &**boxed_prop;
                    if let Ok(function_like) = (&**boxed_value).try_into();

                    then {
                        match key {
                            ast::PropName::Num(ast::Number { value: key_floating_point, .. }) =>  {
                                let module_id: u32 = *key_floating_point as u32;
                                Some((module_id, function_like))
                            }
                            _ => panic!("key in object module listing wasn't a number")
                        }
                    } else {
                        None
                    }
                }
            }))
        }
        _ => {
            panic!("unexpected module listing representation: wasn't an array nor an object")
        }
    }
}
