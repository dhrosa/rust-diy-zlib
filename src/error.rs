use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum InflateError {
    IoError(io::Error),
    InvalidCompressionInfo(u8),
    InvalidCompressionMethod(u8),
    FlagCheckMismatch(u16),
    UnimplementedBlockType(u8),
    LengthComplementMismatch(u16, u16),
    InvalidLengthSymbol(u16),
    InvalidDistanceSymbol(u8),
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
            UnimplementedBlockType(b) => write!(f, "Unimplemented block type: {}", b),
            LengthComplementMismatch(length, inverse_length) => write!(
                f,
                "Corrupted block length. Length: {}, Inverse length: {}",
                length, inverse_length
            ),
            InvalidLengthSymbol(s) => write!(f, "Invalid run length symbol: {}", s),
            InvalidDistanceSymbol(s) => write!(f, "Invaid distance symbol: {}", s),
        }
    }
}

impl Error for InflateError {}

pub type InflateResult<T> = Result<T, InflateError>;
