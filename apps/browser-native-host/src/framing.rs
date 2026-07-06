use serde_json::Value;
use std::io::{self, Read, Write};

pub const MAX_NATIVE_MESSAGE_BYTES: usize = 256 * 1024;

pub fn read_message(reader: &mut impl Read) -> io::Result<Option<Value>> {
    let mut length_bytes = [0_u8; 4];
    match reader.read_exact(&mut length_bytes) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }

    let length = u32::from_le_bytes(length_bytes) as usize;
    if length == 0 || length > MAX_NATIVE_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("native message length {length} is outside the allowed range"),
        ));
    }

    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn write_message(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if body.is_empty() || body.len() > MAX_NATIVE_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("native response length {} is outside the allowed range", body.len()),
        ));
    }
    writer.write_all(&(body.len() as u32).to_le_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn native_message_round_trip_preserves_unicode_byte_length() {
        let value = json!({"message": "Deyana - नमस्ते"});
        let mut encoded = Vec::new();
        write_message(&mut encoded, &value).expect("write native message");

        let decoded = read_message(&mut Cursor::new(encoded))
            .expect("read native message")
            .expect("message exists");
        assert_eq!(decoded, value);
    }

    #[test]
    fn oversized_native_message_is_rejected() {
        let mut encoded = ((MAX_NATIVE_MESSAGE_BYTES + 1) as u32)
            .to_le_bytes()
            .to_vec();
        encoded.extend_from_slice(b"{}");
        let error = read_message(&mut Cursor::new(encoded)).expect_err("message must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
