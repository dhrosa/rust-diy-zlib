use std::io;

enum InflateError {
    IoError(io::Error),
    InvalidCompressionInfo(u8),
    InvalidCompressionMethod(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct CompressionInfo(u8);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CompressionMethod {
    Deflate = 8,
}

impl TryFrom<u8> for CompressionMethod {
    type Error = InflateError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value != CompressionMethod::Deflate as u8 {
            return Err(InflateError::InvalidCompressionMethod(value));
        }
        Ok(CompressionMethod::Deflate)
    }
}

impl CompressionInfo {
    fn window_size(&self) -> u16 {
        let exponent = self.0 + 8;
        return 1 << exponent;
    }
}

impl TryFrom<u8> for CompressionInfo {
    type Error = InflateError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= 8 {
            return Err(InflateError::InvalidCompressionInfo(value));
        }
        Ok(CompressionInfo(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Flags {
    check: u8,
    preset_dictionary: bool,
    compression_level: u8,
}

trait BitRange {
    fn bits(&self, range: std::ops::RangeInclusive<u8>) -> Self;
}

impl BitRange for u8 {
    fn bits(&self, range: std::ops::RangeInclusive<u8>) -> Self {
        let mask = 0xFF >> (7 - range.end());
        (self & mask) >> range.start()
    }
}

impl From<u8> for Flags {
    fn from(value: u8) -> Self {
        Self {
            check: value.bits(0..=4),
            preset_dictionary: value.bits(5..=5) != 0,
            compression_level: value.bits(6..=7),
        }
    }
}

struct StreamHeader {
    method: CompressionMethod,
    info: CompressionInfo,
    flags: Flags,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bits() {
        let val: u8 = 0b1100;
        assert_eq!(val.bits(0..=1), 0b00);
        assert_eq!(val.bits(1..=2), 0b10);
        assert_eq!(val.bits(2..=3), 0b11);
        assert_eq!(val.bits(3..=4), 0b01);
        assert_eq!(val.bits(4..=5), 0b00);
    }

    #[test]
    fn hello() {
        println!("{:?}", CompressionInfo(2u8));
    }
}
