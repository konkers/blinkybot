use std::convert::Infallible;

use blinkybot_rpc::{
    self, ExpressionIndex, GetExpressionEndpoint, PingEndpoint, SetExpression,
    SetExpressionEndpoint,
};
use postcard_rpc::{
    host_client::{HostClient, HostErr},
    standard_icd::{WireError, ERROR_PATH},
};
use wasm_bindgen::prelude::*;

mod utils;

#[derive(Debug)]
pub enum Error<E: std::error::Error> {
    Comms(HostErr<WireError>),
    Endpoint(E),
}

impl<E: std::error::Error> From<Error<E>> for JsValue {
    fn from(e: Error<E>) -> Self {
        match e {
            Error::Comms(e) => format!("comms error: {e:?}").into(),
            Error::Endpoint(e) => format!("endpoint error: {e}").into(),
        }
    }
}

impl<E: std::error::Error> From<HostErr<WireError>> for Error<E> {
    fn from(value: HostErr<WireError>) -> Self {
        Self::Comms(value)
    }
}

#[wasm_bindgen]
pub struct BlinkyBotClient {
    client: HostClient<WireError>,
}

#[wasm_bindgen]
pub struct Expression {
    inner: blinkybot_rpc::Expression,
}

#[wasm_bindgen]
impl Expression {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: blinkybot_rpc::Expression { pixels: [0u16; 7] },
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, state: bool) {
        self.inner.set_pixel(x, y, state)
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> bool {
        self.inner.get_pixel(x, y)
    }
}

#[wasm_bindgen]
impl BlinkyBotClient {
    #[wasm_bindgen(constructor)]
    pub async fn new() -> Result<Self, String> {
        let client = HostClient::try_new_webusb(
            /* vendor */ 0xf569, /* interface */ 1, /* max_transfer_len */ 64,
            /* ep_in */ 1, /* ep_out */ 1, /* err_uri_path */ ERROR_PATH,
            /*outgoing_depth */ 64,
        )
        .await
        .map_err(|e| format!("{e}"))?;

        Ok(Self { client })
    }

    pub fn close(&self) {
        self.client.close();
    }

    pub async fn wait_closed(&self) {
        self.client.wait_closed().await;
    }

    pub async fn ping(&self, id: u32) -> Result<u32, Error<Infallible>> {
        let val = self.client.send_resp::<PingEndpoint>(&id).await?;
        Ok(val)
    }

    pub async fn set_expression(
        &self,
        index: ExpressionIndex,
        expression: Expression,
    ) -> Result<(), Error<Infallible>> {
        self.client
            .send_resp::<SetExpressionEndpoint>(&SetExpression {
                index,
                expression: expression.inner,
            })
            .await?;
        Ok(())
    }

    pub async fn get_expression(
        &self,
        index: ExpressionIndex,
    ) -> Result<Expression, Error<Infallible>> {
        let expression = self
            .client
            .send_resp::<GetExpressionEndpoint>(&index)
            .await?;
        Ok(Expression { inner: expression })
    }
}
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, blinkbot-ui-wasm!");
}
