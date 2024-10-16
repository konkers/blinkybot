#![no_std]

use postcard::experimental::schema::Schema;
use postcard_rpc::endpoint;
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm-bindgen")]
use wasm_bindgen::prelude::*;

endpoint!(PingEndpoint, u32, u32, "ping");
endpoint!(SetExpressionEndpoint, SetExpression, (), "expression/set");
endpoint!(GetExpressionEndpoint, u16, Expression, "expression/get");

#[derive(Serialize, Deserialize, Schema, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "wasm-bindgen", wasm_bindgen)]
pub enum ExpressionIndex {
    Default = 0,
    Blink = 1,
}

#[derive(Serialize, Deserialize, Schema, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Expression {
    pub pixels: [u16; 7],
}

impl Expression {
    pub fn set_pixel(&mut self, x: u32, y: u32, state: bool) {
        if x < 15 && y < 7 {
            if state {
                self.pixels[y as usize] |= 1 << x;
            } else {
                self.pixels[y as usize] &= !(1 << x);
            }
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> bool {
        if x < 15 && y < 7 {
            (self.pixels[y as usize] & (1 << x)) != 0
        } else {
            false
        }
    }
}
#[derive(Serialize, Deserialize, Schema, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetExpression {
    pub index: ExpressionIndex,
    pub expression: Expression,
}
