use std::io::{self, Read};

// Buffer of up to 8-bits for reading from a byte-based input at a sub-byte
// granularity.
#[derive(Debug, Clone, Copy)]
struct BitBuffer {
    byte: u8,
    bit_offset: u8,
}

impl BitBuffer {
    fn new(byte: u8) -> BitBuffer {
        BitBuffer {
            byte,
            bit_offset: 0,
        }
    }

    // Consume a single bit. The left return value contains the remaining bits
    // left to read, if there are any.
    fn read_bit(self) -> (Option<BitBuffer>, u8) {
        let Self { byte, bit_offset } = self;
        let bit = byte & 1;
        let byte = byte >> 1;
        let bit_offset = bit_offset + 1;
        let buffer = if bit_offset == 8 {
            None
        } else {
            Some(BitBuffer { byte, bit_offset })
        };
        (buffer, bit)
    }
}

// Extention to io::Read that allows reading individual bits from the input
// stream.
pub struct BitReader<R: io::Read> {
    input: R,
    bit_buffer: Option<BitBuffer>,
}

impl<R: io::Read> BitReader<R> {
    pub fn new(input: R) -> Self {
        BitReader {
            input,
            bit_buffer: None,
        }
    }

    fn read_byte(&mut self) -> io::Result<u8> {
        let mut bytes = [0u8];
        self.read_exact(&mut bytes)?;
        Ok(bytes[0])
    }

    pub fn read_bit(&mut self) -> io::Result<u8> {
        let buffer = match self.bit_buffer {
            None => BitBuffer::new(self.read_byte()?),
            Some(b) => b,
        };
        let bit: u8;
        (self.bit_buffer, bit) = buffer.read_bit();
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
        self.bit_buffer = None;
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
