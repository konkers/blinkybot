#![no_std]

use postcard::experimental::schema::Schema;
use postcard_rpc::endpoint;
use serde::{Deserialize, Serialize};

endpoint!(PingEndpoint, u32, u32, "ping");
endpoint!(SetExpressionEndpoint, SetExpression, (), "expression/set");
endpoint!(GetExpressionEndpoint, u16, Expression, "expression/get");

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct Expression {
    pub pixels: [u16; 5],
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct SetExpression {
    pub index: u16,
    pub expression: Expression,
}
