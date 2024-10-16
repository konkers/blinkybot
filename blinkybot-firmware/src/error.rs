use defmt::Format;

#[derive(Format, Clone, Copy)]
pub enum Error {
    Storage,
    Unknown,
}

pub type Result<T> = core::result::Result<T, Error>;
