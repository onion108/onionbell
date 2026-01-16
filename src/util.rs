use std::io::Read;

use crate::error::AppError;

pub fn reader_to_string(mut reader: impl Read) -> Result<String, AppError> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    Ok(buf)
}
