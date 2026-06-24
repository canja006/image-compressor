use crate::error::EngineError;
use image::DynamicImage;
use std::io::Cursor;

/// Decode an image from an in-memory byte buffer, guessing the format from its content
/// (not the file extension). A corrupt or unsupported buffer returns a typed error rather
/// than panicking.
pub fn decode(bytes: &[u8]) -> Result<DynamicImage, EngineError> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| EngineError::Decode(e.to_string()))?;
    reader
        .decode()
        .map_err(|e| EngineError::Decode(e.to_string()))
}
