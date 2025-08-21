#![feature(assert_matches)]

mod bit_reader;
pub mod bit_string;
pub mod block_decoder;
mod code;
pub mod code_table;
mod error;
mod header;
mod lz77;

use crate::bit_reader::{BitRead, BitReader};
use crate::error::{InflateError, InflateResult};
use crate::header::*;

use std::io::{self, Read};

#[derive(Debug)]
pub struct Inflator<R: io::Read> {
    input: BitReader<R>,
    pub header: StreamHeader,
}

impl<R: io::Read> Inflator<R> {
    pub fn try_new(input: R) -> InflateResult<Self> {
        let mut header = [0u8; 2];
        let mut input = BitReader::new(input);
        input.read_exact(&mut header)?;
        let header = StreamHeader::try_from(&header)?;
        Ok(Self { input, header })
    }

    pub fn next_block(&mut self) -> InflateResult<Vec<u8>> {
        let _is_final_block = self.input.read_bit()?;
        let block_type = self.input.read_bits::<u8>(2)?;
        if block_type != 0 {
            return Err(InflateError::UnimplementedBlockType(block_type));
        }
        self.read_uncompressed_block()
    }

    fn read_uncompressed_block(&mut self) -> InflateResult<Vec<u8>> {
        let length = self.input.read_u16()?;
        let inverse_length = self.input.read_u16()?;
        if inverse_length != (!length) {
            return Err(InflateError::LengthComplementMismatch(
                length,
                inverse_length,
            ));
        }
        let mut data = vec![0u8; length as usize];
        self.input.read_exact(&mut data)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::InflateError::*;
    use super::*;
    use std::assert_matches::assert_matches;

    #[test]
    fn test_truncated_header() {
        let mut raw: &[u8] = &[0];
        assert_matches!(Inflator::try_new(&mut raw), Err(IoError(_)));
    }

    #[test]
    fn test_begin_stream() -> InflateResult<()> {
        let mut raw: &[u8] = &[0x48, 0b1010_0000 + 8];
        let inflator = Inflator::try_new(&mut raw)?;
        assert_eq!(
            inflator.header,
            StreamHeader {
                method: CompressionMethod::Deflate,
                info: CompressionInfo::try_from(4)?,
                flags: Flags {
                    preset_dictionary: true,
                    compression_level: 2,
                }
            }
        );

        Ok(())
    }

    #[test]
    fn test_uncompressed_block() -> InflateResult<()> {
        let mut raw: &[u8] = &[
            0x48,
            0b1010_0000 + 8,
            // header
            0,
            // length
            5,
            0,
            // inverse length
            !5,
            !0,
            // data
            1,
            2,
            3,
            4,
            5,
        ];
        let mut inflator = Inflator::try_new(&mut raw)?;
        let block = inflator.next_block()?;
        assert_eq!(block, vec![1, 2, 3, 4, 5]);
        Ok(())
    }
}
