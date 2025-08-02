use std::io::{self, Read};

// Extention to io::Read that allows reading individual bits from the input
// stream.
struct BitReader<R: io::Read> {
    input: R,
    current_byte: Option<u8>,
    bit_offset: u8,
}

impl<R: io::Read> BitReader<R> {
    pub fn new(input: R) -> Self {
        BitReader {
            input,
            current_byte: None,
            bit_offset: 0,
        }
    }

    fn reset_buffer(&mut self) {
        self.current_byte = None;
        self.bit_offset = 0;
    }

    fn refill_buffer(&mut self) -> io::Result<u8> {
        self.reset_buffer();
        let mut bytes = [0u8];
        self.read_exact(&mut bytes)?;
        let byte = bytes[0];
        self.current_byte = Some(byte);
        Ok(byte)
    }

    pub fn read_bit(&mut self) -> io::Result<u8> {
        let byte = match self.current_byte {
            Some(b) => b,
            None => self.refill_buffer()?,
        };
        let bit = byte & 1;
        self.bit_offset += 1;
        self.current_byte = Some(byte >> 1);
        if self.bit_offset == 8 {
            self.reset_buffer()
        }
        Ok(bit)
    }

    pub fn read_bits(&mut self, count: u8) -> io::Result<u32> {
        let mut value = 0;
        for i in 0..count {
            let bit = self.read_bit()? as u32;
            value |= bit << i;
        }
        Ok(value)
    }
}

// Pass-through implementation of io::Read that delegates to upstream reader.
// Any partially-read byte initially present is discarded.
impl<R: io::Read> io::Read for BitReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reset_buffer();
        self.input.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passthrough() -> io::Result<()> {
        let raw: &[u8] = &[1, 2];
        let mut reader = BitReader::new(raw);

        let mut out = Vec::<u8>::new();
        reader.read_to_end(&mut out)?;
        assert_eq!(out, vec![1, 2]);
        Ok(())
    }

    #[test]
    fn test_read_bit() -> io::Result<()> {
        // A pattern of 1x1, 0, 2x1, 0, ...
        let raw: &[u8] = &[0b11101101, 0b1101_1110];
        let mut reader = BitReader::new(raw);

        // 1st byte
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 0);
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 0);
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 1);
        // 2nd byte
        assert_eq!(reader.read_bit()?, 0);
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 1);
        assert_eq!(reader.read_bit()?, 0);
        Ok(())
    }

    #[test]
    fn test_read_bits() -> io::Result<()> {
        // A pattern of 1x1, 0, 2x1, 0, ...
        let raw: &[u8] = &[0b11101101, 0b1101_1110];
        let mut reader = BitReader::new(raw);

        assert_eq!(reader.read_bits(1)?, 0b1);
        assert_eq!(reader.read_bits(2)?, 0b10);
        assert_eq!(reader.read_bits(3)?, 0b101);
        // Cross byte boundary.
        assert_eq!(reader.read_bits(4)?, 0b1011);
        assert_eq!(reader.read_bits(5)?, 0b10111);

        Ok(())
    }

    #[test]
    fn test_passthrough_after_partial_read() -> io::Result<()> {
        let raw: &[u8] = &[0b1010_1010, 0b1100_1100, 0b1111_1110];
        let mut reader = BitReader::new(raw);

        assert_eq!(reader.read_bits(4)?, 0b1010);

        // Upper half of first-byte should be discarded.
        let mut out = [0u8];
        reader.read_exact(&mut out)?;
        assert_eq!(out, [0b1100_1100]);

        // Start another partial read.
        assert_eq!(reader.read_bits(4)?, 0b1110);

        Ok(())
    }
}
