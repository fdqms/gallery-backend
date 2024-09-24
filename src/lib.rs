use tract_onnx::prelude::{Graph, TypedFact, TypedOp};
use tract_onnx::tract_core;

pub type AiModel = tract_core::model::typed::RunnableModel<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

pub mod utils;
pub mod route;
pub mod model;
pub mod middleware;
pub mod ai;
pub mod db;