use std::collections::HashMap;
use std::time::Instant;

use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_parser::Parser;
use swc_ecma_parser::{lexer::Lexer, StringInput, Syntax};
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

    use ParseError::WalkError;

    let expr = script.body.get(0).expect("1st element of body");

    let expr_stmt = if let ast::Stmt::Expr(expr_stmt @ ast::ExprStmt { .. }) = expr {
        expr_stmt
    } else {
        return Err(WalkError("first expr needs to be an expr stmt"));
    };

    let call = if let ast::Expr::Call(call @ ast::CallExpr { .. }) = &*expr_stmt.expr {
        call
    } else {
        return Err(WalkError("first expr stmt needs to be a call"));
    };

    let expr_or_spread = call.args.get(0).ok_or(WalkError("missing arg to call"))?;

    let array_lit =
        if let ast::Expr::Array(array_lit @ ast::ArrayLit { .. }) = &*expr_or_spread.expr {
            array_lit
        } else {
            return Err(WalkError("arg isn't an array literal"));
        };

    let object_of_modules = array_lit
        .elems
        .get(1)
        .ok_or(WalkError("need second elem of pushed array"))
        .map(|expr_or_spread_opt| {
            expr_or_spread_opt
                .as_ref()
                .ok_or(WalkError("second elem of pushed array is a hole"))
        })
        .and_then(std::convert::identity)?;

    let object_lit =
        if let ast::Expr::Object(object_lit @ ast::ObjectLit { .. }) = &*object_of_modules.expr {
            object_lit
        } else {
            return Err(WalkError("second array elem isn't an object literal"));
        };

    let mut mapping: HashMap<u64 /* module */, HashMap<String, String>> = HashMap::new();

    for prop_or_spread in &object_lit.props {
        let boxed_prop = if let ast::PropOrSpread::Prop(boxed_prop) = prop_or_spread {
            boxed_prop
        } else {
            return Err(WalkError("object literal prop or spread is a spread"));
        };

        let key_value_prop =
            if let ast::Prop::KeyValue(key_value_prop @ ast::KeyValueProp { .. }) = &**boxed_prop {
                key_value_prop
            } else {
                return Err(WalkError("object literal prop isn't keyvalue"));
            };

        let key_num = if let ast::PropName::Num(key_num) = key_value_prop.key {
            key_num
        } else {
            return Err(WalkError("prop key isn't a number"));
        };

        let fn_expr = if let ast::Expr::Fn(fn_expr) = &*key_value_prop.value {
            fn_expr
        } else {
            return Err(WalkError("prop value isn't a fn"));
        };

        let body = match fn_expr.function.body.as_ref() {
            Some(block_stmt) => block_stmt,
            None => continue,
        };

        let stmt = match body.stmts.get(0) {
            Some(stmt) => stmt,
            None => continue,
        };

        let expr_stmt = if let ast::Stmt::Expr(expr_stmt) = stmt {
            expr_stmt
        } else {
            return Err(WalkError("statement isn't an expr stmt"));
        };

        let assign = if let ast::Expr::Assign(assign) = &*expr_stmt.expr {
            assign
        } else {
            return Err(WalkError("expr isn't an assign"));
        };

        let mut classes: HashMap<String, String> = HashMap::new();

        let classes_mapping_object_lit =
            if let ast::Expr::Object(classes_mapping_object_lit @ ast::ObjectLit { .. }) =
                &*assign.right
            {
                classes_mapping_object_lit
            } else {
                return Err(WalkError("rhs of assign isn't an object"));
            };

        for mapping_prop_or_spread in &classes_mapping_object_lit.props {
            let boxed_prop = if let ast::PropOrSpread::Prop(boxed_prop) = mapping_prop_or_spread {
                boxed_prop
            } else {
                return Err(WalkError("mapping prop or spread isn't a prop"));
            };

            let kv_prop = if let ast::Prop::KeyValue(kv_prop) = &**boxed_prop {
                kv_prop
            } else {
                return Err(WalkError("mapping prop isn't a keyvalue"));
            };

            let key: &str = match &kv_prop.key {
                ast::PropName::Ident(ast::Ident { sym: atom, .. }) => &atom,
                ast::PropName::Str(ast::Str { value: atom, .. }) => &atom,
                _ => return Err(WalkError("failed to extract key from mapping prop")),
            };

            let value: &str = if let ast::Expr::Lit(ast::Lit::Str(ast::Str {
                value: atom, ..
            })) = &*kv_prop.value
            {
                &*atom
            } else {
                return Err(WalkError("failed to extract value from mapping prop"));
            };

            classes.insert(key.to_string(), value.to_string());
        }

        mapping.insert(key_num.value as u64, classes);
    }

    dbg!(&mapping);

    Ok(())
}
