pub mod webpack;
pub use webpack::*;

use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_parser::{error::Error as SwcError, lexer::Lexer, Parser, StringInput, Syntax};
extern crate swc_ecma_ast as ast;
use thiserror::Error;

/// Parses a script.
pub fn parse_script(js: String) -> Result<ast::Script, ParseError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("script.js".into()), js);

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
