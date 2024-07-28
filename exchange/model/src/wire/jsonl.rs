use std::io::{BufRead, Read, Write};

pub struct JsonLinesEncoder<W: Write> {
    writer: W,
}
impl<W: Write> JsonLinesEncoder<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
    pub fn encode<T: serde::Serialize>(&mut self, value: &T) -> Result<(), std::io::Error> {
        serde_json::to_writer(&mut self.writer, value)?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }
}
pub struct JsonLinesDecoder<R: Read> {
    reader: R,
}
impl<R: Read + BufRead> JsonLinesDecoder<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
    pub fn decode<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, std::io::Error> {
        let mut buffer = String::new();
        self.reader.read_line(&mut buffer)?;
        serde_json::from_str(&buffer).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse JSON: {}", e),
            )
        })
    }
}
