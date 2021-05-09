use std::collections::HashMap;

use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_parser::{error::Error as SwcError, lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};
extern crate swc_ecma_ast as ast;
use thiserror::Error;

pub type ClassMappingMap = HashMap<String, String>;

// NOTE(slice): More like `ClassMappingModulesMap`, but that's too long.
pub type ClassModuleMap = HashMap<u64 /* webpack module id */, ClassMappingMap>;

struct ClassMappingVisitor {
    classes: ClassMappingMap,
}

// NOTE(slice): It's worth noting that these visitors will stop visiting deeper
// if you do not explicitly call `visit_children_with` within the visitor
// methods.
//
// However, we don't need to do that because there's only 2 levels of
// key-values. The outer level is handled by `ClassesModuleVisitor` and the
// inner by `ClassesClassMappingVisitor`.

impl Visit for ClassMappingVisitor {
    fn visit_key_value_prop(&mut self, n: &ast::KeyValueProp, _parent: &dyn swc_ecma_visit::Node) {
        let key: &str = match &n.key {
            // { a: ... }
            ast::PropName::Ident(ast::Ident { sym: atom, .. }) => &atom,
            // { "some key": ... }
            ast::PropName::Str(ast::Str { value: atom, .. }) => &atom,
            _ => return,
        };

        let value: &str = match &*n.value {
            ast::Expr::Lit(ast::Lit::Str(ast::Str { value: atom, .. })) => &atom,
            _ => return,
        };

        self.classes.insert(key.to_string(), value.to_string());
    }
}

struct ClassModuleVisitor {
    modules: ClassModuleMap,
}

impl Visit for ClassModuleVisitor {
    fn visit_key_value_prop(&mut self, n: &ast::KeyValueProp, _parent: &dyn swc_ecma_visit::Node) {
        let module_id = match &n.key {
            // wow, i sure do hope webpack doesn't start using floating-point
            // numbers for module ids
            ast::PropName::Num(ast::Number { value, .. }) => *value as u64,
            _ => return,
        };

        let mut class_mapping_visitor = ClassMappingVisitor {
            classes: HashMap::new(),
        };
        n.visit_children_with(&mut class_mapping_visitor);

        self.modules
            .insert(module_id, class_mapping_visitor.classes);
    }
}

/// Parses a script.
pub fn parse_script(js: &str) -> Result<ast::Script, ParseError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("script.js".into()), js.into());

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        // JscTarget = es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);
    Ok(parser.parse_script()?)
}

/// Walk a Webpack chunk script containing classname mappings.
pub fn walk_classes_chunk(script: &ast::Script) -> Result<ClassModuleMap, ParseError> {
    let mut visitor = ClassModuleVisitor {
        modules: HashMap::new(),
    };
    crate::util::measure("visiting class modules and mapping", || {
        script.visit_children_with(&mut visitor)
    });

    let total_mappings: usize = visitor
        .modules
        .iter()
        .map(|(_, mappings)| mappings.len())
        .sum();

    log::debug!(
        "visited {} class module(s), totalling to {} class mappings",
        visitor.modules.len(),
        total_mappings
    );

    Ok(visitor.modules)
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("missing ast node: {0}")]
    MissingNode(&'static str),

    #[error("parsing error")]
    Swc(SwcError),
}

impl From<SwcError> for ParseError {
    fn from(err: SwcError) -> Self {
        ParseError::Swc(err)
    }
}

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
