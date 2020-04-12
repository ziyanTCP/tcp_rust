use crate::tcp::flow;
use std::collections::{HashMap, VecDeque};
#[derive(Default)]
pub struct f_t {
    pub(crate) connections: HashMap<flow::Quad, flow::flow>,
}
