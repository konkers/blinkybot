use core::ops::Range;

use defmt::{debug, error};
use embedded_storage_async::nor_flash::NorFlash;
use sequential_storage::{
    cache::NoCache,
    map::{fetch_item, store_item, Key, SerializationError, Value},
};
use serde::{Deserialize, Serialize};

use blinkybot_rpc::{Expression, ExpressionIndex};

const POSTCARD_BYTES_PER_WORD: usize = 5;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum ConfigKey {
    ExpressionV0(ExpressionIndex),
}

impl ConfigKey {
    const KEY_WORDS: usize = 2;
    const BUFFER_SIZE: usize = Self::KEY_WORDS * POSTCARD_BYTES_PER_WORD;
}

fn postcard_to_storage_err(e: postcard::Error) -> SerializationError {
    match e {
        postcard::Error::SerializeBufferFull => SerializationError::BufferTooSmall,
        _ => SerializationError::InvalidFormat,
    }
}

impl Key for ConfigKey {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        let key_buf = postcard::to_slice(&self, buffer).map_err(postcard_to_storage_err)?;
        Ok(key_buf.len())
    }

    fn deserialize_from(buffer: &[u8]) -> Result<(Self, usize), SerializationError> {
        let (key, value_buf) =
            postcard::take_from_bytes(buffer).map_err(postcard_to_storage_err)?;
        Ok((key, buffer.len() - value_buf.len()))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum ConfigValue {
    ExpressionV0(Expression),
}

impl ConfigValue {
    const EXPRESSION_WORDS: usize = 7;
    const PADDING_WORDS: usize = 0;
    const BUFFER_SIZE: usize =
        (Self::EXPRESSION_WORDS + Self::PADDING_WORDS) * POSTCARD_BYTES_PER_WORD;
}

impl<'a> Value<'a> for ConfigValue {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        let key_buf = postcard::to_slice(&self, buffer).map_err(postcard_to_storage_err)?;
        Ok(key_buf.len())
    }

    fn deserialize_from(buffer: &'a [u8]) -> Result<Self, SerializationError> {
        let (value, _) = postcard::take_from_bytes(buffer).map_err(postcard_to_storage_err)?;
        Ok(value)
    }
}

pub struct FlashConfigStore<Flash: NorFlash>
where
    Flash::Error: defmt::Format,
{
    flash: Flash,
    range: Range<u32>,
}

impl<Flash: NorFlash> FlashConfigStore<Flash>
where
    Flash::Error: defmt::Format,
{
    pub fn new(flash: Flash, range: Range<u32>) -> Self {
        Self { flash, range }
    }

    fn default_expression(index: ExpressionIndex) -> Expression {
        match index {
            ExpressionIndex::Default => Expression {
                pixels: [
                    0b000_0000_0000_0000,
                    0b001_1000_0000_1100,
                    0b010_0100_0001_0010,
                    0b010_0100_0001_0010,
                    0b001_1001_0100_1100,
                    0b000_0001_1100_0000,
                    0b000_0000_0000_0000,
                ],
            },
            ExpressionIndex::Blink => Expression {
                pixels: [
                    0b000_0000_0000_0000,
                    0b000_0000_0000_0000,
                    0b011_1100_0001_1110,
                    0b000_0000_0000_0000,
                    0b000_0001_0100_0000,
                    0b000_0001_1100_0000,
                    0b000_0000_0000_0000,
                ],
            },
        }
    }

    pub async fn get_expression(&mut self, index: ExpressionIndex) -> Expression {
        let mut buffer = [0u8; ConfigKey::BUFFER_SIZE + ConfigValue::BUFFER_SIZE];
        let key = ConfigKey::ExpressionV0(index);
        match fetch_item(
            &mut self.flash,
            self.range.clone(),
            &mut NoCache::new(),
            &mut buffer,
            &key,
        )
        .await
        {
            Ok(value) => {
                let Some(value) = value else {
                    return Self::default_expression(index);
                };

                // Protects against future additions to Config vaule.
                #[allow(irrefutable_let_patterns)]
                let ConfigValue::ExpressionV0(expression) = value
                else {
                    return Self::default_expression(index);
                };

                expression
            }
            Err(e) => {
                error!("Error fetching expression {}: {}", index, e);
                Self::default_expression(index)
            }
        }
    }
}