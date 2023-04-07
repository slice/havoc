//! CSS class mapping dumping.

use crate::{
    artifact::Artifact,
    discord::{AssetCache, AssetsExt, RootScript},
    dump::{DumpError, DumpResult},
    parse::{ModuleId, ParseError},
    scrape::ScrapeError,
};

use super::Dump;

use std::collections::HashMap;

extern crate swc_ecma_ast as ast;
use swc_ecma_visit::{Visit, VisitWith};

pub type ClassMappingMap = HashMap<String, String>;

// NOTE(slice): More like `ClassMappingModulesMap`, but that's too long.
pub type ClassModuleMap = HashMap<ModuleId, ClassMappingMap>;

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
    fn visit_key_value_prop(&mut self, n: &ast::KeyValueProp) {
        let key: &str = match &n.key {
            // { a: ... }
            ast::PropName::Ident(ast::Ident { sym: atom, .. }) => atom,
            // { "some key": ... }
            ast::PropName::Str(ast::Str { value: atom, .. }) => atom,
            _ => return,
        };

        let value: &str = match &*n.value {
            ast::Expr::Lit(ast::Lit::Str(ast::Str { value: atom, .. })) => atom,
            _ => return,
        };

        self.classes.insert(key.to_string(), value.to_string());
    }
}

struct ClassModuleVisitor {
    modules: ClassModuleMap,
}

impl Visit for ClassModuleVisitor {
    fn visit_key_value_prop(&mut self, n: &ast::KeyValueProp) {
        let module_id = match &n.key {
            // wow, i sure do hope webpack doesn't start using floating-point
            // numbers for module ids
            ast::PropName::Num(ast::Number { value, .. }) => *value as ModuleId,
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

/// Walk a Webpack chunk script containing classname mappings.
pub fn walk_classes_chunk(script: &ast::Script) -> Result<ClassModuleMap, ParseError> {
    let mut visitor = ClassModuleVisitor {
        modules: HashMap::new(),
    };

    tracing::info!("visiting class modules and mapping");
    script.visit_children_with(&mut visitor);

    let total_mappings: usize = visitor
        .modules
        .values()
        .map(|mappings| mappings.len())
        .sum();

    tracing::info!(
        "visited {} class module(s), totalling to {} class mappings",
        visitor.modules.len(),
        total_mappings
    );

    Ok(visitor.modules)
}

pub struct CSSClasses;

#[async_trait::async_trait]
impl Dump for CSSClasses {
    async fn dump(
        &mut self,
        artifact: &(dyn Artifact + Sync),
        cache: &mut AssetCache,
    ) -> Result<DumpResult, DumpError> {
        let classes_asset = artifact
            .assets()
            .iter()
            .find_root_script(RootScript::Classes)
            .ok_or(ScrapeError::MissingBranchPageAssets(
                "failed to locate root classes script; discord has updated their /channels/@me",
            ))?;

        let content = cache.raw_content(classes_asset).await?;
        let classes_js = std::str::from_utf8(content)
            .map_err(ScrapeError::Decoding)?
            .to_owned();
        let script = crate::parse::parse_script(classes_js)?;
        let mapping = walk_classes_chunk(&script)?;

        Ok(DumpResult::from_serializable(&mapping, "classes")?)
    }
}
