#![no_std]

use postcard::experimental::schema::Schema;
use postcard_rpc::endpoint;
use serde::{Deserialize, Serialize};

endpoint!(PingEndpoint, u32, u32, "ping");
endpoint!(SetExpressionEndpoint, SetExpression, (), "expression/set");
endpoint!(GetExpressionEndpoint, u16, Expression, "expression/get");

#[derive(Serialize, Deserialize, Schema, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ExpressionIndex {
    Default = 0,
    Blink = 1,
}

#[derive(Serialize, Deserialize, Schema, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Expression {
    pub pixels: [u16; 7],
}

#[derive(Serialize, Deserialize, Schema, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetExpression {
    pub index: ExpressionIndex,
    pub expression: Expression,
}
