#![feature(assert_matches)]

use std::error::Error;
use std::fmt;
use std::io;

trait BitRange {
    fn bits(&self, range: std::ops::RangeInclusive<u8>) -> Self;
}

impl BitRange for u8 {
    fn bits(&self, range: std::ops::RangeInclusive<u8>) -> Self {
        let mask = 0xFF >> (7 - range.end());
        (self & mask) >> range.start()
    }
}

#[derive(Debug)]
enum InflateError {
    IoError(io::Error),
    InvalidCompressionInfo(u8),
    InvalidCompressionMethod(u8),
    FlagCheckMismatch(u16),
}

impl From<io::Error> for InflateError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}

impl fmt::Display for InflateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use InflateError::*;

        match self {
            IoError(e) => write!(f, "I/O error: {}", e),
            InvalidCompressionInfo(i) => write!(f, "Invalid compression info value: {}", i),
            InvalidCompressionMethod(m) => write!(f, "Invalid compression method: {}", m),
            FlagCheckMismatch(c) => write!(f, "Flag checksum is not a multiple of 31: {}", c),
        }
    }
}

impl Error for InflateError {}

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
    preset_dictionary: bool,
    compression_level: u8,
}

impl From<u8> for Flags {
    fn from(value: u8) -> Self {
        Self {
            preset_dictionary: value.bits(5..=5) != 0,
            compression_level: value.bits(6..=7),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StreamHeader {
    method: CompressionMethod,
    info: CompressionInfo,
    flags: Flags,
}

impl TryFrom<&[u8; 2]> for StreamHeader {
    type Error = InflateError;

    fn try_from(bytes: &[u8; 2]) -> Result<Self, Self::Error> {
        let [cmf, flg] = *bytes;
        let method = CompressionMethod::try_from(cmf.bits(0..=3))?;
        let info = CompressionInfo::try_from(cmf.bits(4..=7))?;
        let flags = Flags::from(flg);

        let checksum = ((cmf as u16) << 8) + (flg as u16);
        if checksum % 31 != 0 {
            return Err(InflateError::FlagCheckMismatch(checksum));
        }
        Ok(StreamHeader {
            method,
            info,
            flags,
        })
    }
}

#[derive(Debug)]
struct Inflator<R: io::Read> {
    input: R,
    header: StreamHeader,
}

impl<R: io::Read> Inflator<R> {
    fn try_new(mut input: R) -> Result<Self, InflateError> {
        let mut header = [0u8; 2];
        input.read_exact(&mut header)?;
        let header = StreamHeader::try_from(&header)?;
        Ok(Self { input, header })
    }
}

#[cfg(test)]
mod tests {
    use super::InflateError::*;
    use super::*;
    use std::assert_matches::assert_matches;

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
    fn test_invalid_compression_method() {
        assert_matches!(
            StreamHeader::try_from(&[1, 0]),
            Err(InvalidCompressionMethod(1))
        );
    }

    #[test]
    fn test_invalid_compression_info() {
        assert_matches!(
            StreamHeader::try_from(&[0x88, 0]),
            Err(InvalidCompressionInfo(8))
        );
    }

    #[test]
    fn test_flag_check_mismatch() {
        assert_matches!(
            StreamHeader::try_from(&[0x08, 0]),
            Err(FlagCheckMismatch(0x800)),
        );
    }

    #[test]
    fn test_valid_stream_header() -> Result<(), InflateError> {
        let header = StreamHeader::try_from(&[0x48, 0b1010_0000 + 8])?;
        assert_eq!(
            header,
            StreamHeader {
                method: CompressionMethod::Deflate,
                info: CompressionInfo(4),
                flags: Flags {
                    preset_dictionary: true,
                    compression_level: 2,
                }
            }
        );
        Ok(())
    }

    #[test]
    fn test_window_size() {
        assert_eq!(CompressionInfo(7).window_size(), 32768);
    }

    #[test]
    fn test_truncated_header() {
        let mut raw: &[u8] = &[0];
        assert_matches!(Inflator::try_new(&mut raw), Err(IoError(_)));
    }

    #[test]
    fn test_begin_stream() -> Result<(), InflateError> {
        let mut raw: &[u8] = &[0x48, 0b1010_0000 + 8];
        let inflator = Inflator::try_new(&mut raw)?;
        assert_eq!(
            inflator.header,
            StreamHeader {
                method: CompressionMethod::Deflate,
                info: CompressionInfo(4),
                flags: Flags {
                    preset_dictionary: true,
                    compression_level: 2,
                }
            }
        );

        Ok(())
    }
}
