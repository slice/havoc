use std::collections::HashMap;
use std::time::Instant;

use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};
extern crate swc_ecma_ast as ast;

#[derive(Debug)]
pub enum ParseError {
    SwcError(swc_ecma_parser::error::Error),
    WalkError(&'static str),
}

impl From<swc_ecma_parser::error::Error> for ParseError {
    fn from(error: swc_ecma_parser::error::Error) -> ParseError {
        ParseError::SwcError(error)
    }
}

struct ClassesClassMappingVisitor {
    classes: HashMap<String, String>,
}

// NOTE(slice): It's worth noting that these visitors will stop visiting deeper
// if you do not explicitly call `visit_children_with` within the visitor
// methods.
//
// However, we don't need to do that because there's only 2 levels of
// key-values. The outer level is handled by `ClassesModuleVisitor` and the
// inner by `ClassesClassMappingVisitor`.

impl Visit for ClassesClassMappingVisitor {
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

struct ClassesModuleVisitor {
    modules: HashMap<u64, HashMap<String, String>>,
}

impl Visit for ClassesModuleVisitor {
    fn visit_key_value_prop(&mut self, n: &ast::KeyValueProp, _parent: &dyn swc_ecma_visit::Node) {
        let module_id = match &n.key {
            // wow, i sure do hope webpack doesn't start using floating-point
            // numbers for module ids
            ast::PropName::Num(ast::Number { value, .. }) => *value as u64,
            _ => return,
        };

        let mut class_mapping_visitor = ClassesClassMappingVisitor {
            classes: HashMap::new(),
        };
        n.visit_children_with(&mut class_mapping_visitor);

        self.modules
            .insert(module_id, class_mapping_visitor.classes);
    }
}

pub fn parse_classes_file(js: &str) -> Result<(), ParseError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("classes.js".into()), js.into());

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        // JscTarget = es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    let instant = Instant::now();
    let script = parser.parse_script()?;
    log::debug!("took {:?} to parse classes script", instant.elapsed());

    let mut visitor = ClassesModuleVisitor {
        modules: HashMap::new(),
    };
    script.visit_children_with(&mut visitor);

    for (module_id, mappings) in &visitor.modules {
        println!("module {} has {} mapping(s)", module_id, mappings.len());
    }

    Ok(())
}
