use std::collections::{HashMap, VecDeque};
use crate::tcp::flow;
#[derive(Default)]
pub struct f_t{
    pub(crate) connections: HashMap<flow::Quad, flow::flow>,
}