//! Lowering methods that emit formatting [`Token`] from AST [`Node`].
//!
//! [`Token`]: crate::token::Token
//! [`Node`]: tree_sitter::Node

mod comment;
mod module;
mod node;

use comment::*;
use module::*;
pub(crate) use node::*;
