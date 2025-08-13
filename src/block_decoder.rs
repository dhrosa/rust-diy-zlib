use crate::bit_reader::BitRead;
use crate::code_table::CodeToSymbolTable;
use crate::error::{InflateError, InflateResult};
use crate::lz77::Instruction;

struct BlockDecoder<'a, R: BitRead> {
    reader: &'a mut R,
    ll_table: CodeToSymbolTable,
    distance_table: CodeToSymbolTable,
}

impl<'a, R: BitRead> BlockDecoder<'a, R> {
    pub fn new_fixed(reader: &'a mut R) -> Self {
        Self {
            reader,
            ll_table: CodeToSymbolTable::fixed_ll(),
            distance_table: CodeToSymbolTable::fixed_distance(),
        }
    }

    pub fn next(&mut self) -> InflateResult<Instruction> {
        let symbol = self.ll_table.read_symbol(self.reader)? as u16;
        if symbol < 256 {
            return Ok(Instruction::Literal(symbol as u8));
        }
        if symbol == 256 {
            return Ok(Instruction::EndOfBlock);
        }
        let length = self.read_length(symbol)?;
        let distance = self.read_distance()?;
        Ok(Instruction::BackReference { length, distance })
    }

    fn read_length(&mut self, symbol: u16) -> InflateResult<u16> {
        // Borrowed from
        // https://github.com/nayuki/Simple-DEFLATE-decompressor/blob/2586b459a84f8918851a1078c2c0482b1b383fba/python/deflatedecompress.py#L439
        if symbol <= 264 {
            return Ok(symbol - 254);
        }
        if symbol <= 284 {
            let extra_bit_count = (symbol - 261) / 4;
            let extra_bits = self.reader.read_bits::<u16>(extra_bit_count as u8)?;
            let base = ((symbol - 265) % 4 + 4) << extra_bit_count;
            return Ok(3 + base + extra_bits);
        }
        if symbol == 285 {
            return Ok(258);
        }
        Err(InflateError::InvalidLengthSymbol(symbol))
    }

    fn read_distance(&mut self) -> InflateResult<u16> {
        // Borrowed from https://github.com/nayuki/Simple-DEFLATE-decompressor/blob/2586b459a84f8918851a1078c2c0482b1b383fba/python/deflatedecompress.py#L456
        let symbol = self.distance_table.read_symbol(self.reader)? as u16;
        if symbol <= 3 {
            return Ok(symbol + 1);
        }
        if symbol <= 29 {
            let extra_bit_count = symbol / 2 + 1;
            let extra_bits = self.reader.read_bits::<u16>(extra_bit_count as u8)?;
            let base = (symbol % 2 + 2) << extra_bit_count;
            return Ok(1 + base + extra_bits);
        }
        Err(InflateError::InvalidDistanceSymbol(symbol as u8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bit_reader::BitReader;
    use crate::bit_string::bit_string;

    #[test]
    fn test_read_literal() -> InflateResult<()> {
        // 0 is 8-bit code: 00110000
        // 144 is 9-bit code: 110010000
        let raw = bit_string("0000 1100 0001 0011 0");
        let mut reader = BitReader::new(raw.as_slice());
        let mut decoder = BlockDecoder::new_fixed(&mut reader);
        Ok(())
    }

    #[test]
    fn test_read_end_of_block() -> InflateResult<()> {
        // end of block is 7-bit code: 000 0000.
        let raw = bit_string("1000 0000");
        let mut reader = BitReader::new(raw.as_slice());
        let mut decoder = BlockDecoder::new_fixed(&mut reader);
        assert_eq!(decoder.next()?, Instruction::EndOfBlock);
        Ok(())
    }

    #[test]
    fn test_back_reference() -> InflateResult<()> {
        let raw = bit_string("00110000 00000000 00000000");
        let mut reader = BitReader::new(raw.as_slice());
        let mut decoder = BlockDecoder::new_fixed(&mut reader);
        assert_eq!(
            decoder.next()?,
            Instruction::BackReference {
                length: 8,
                distance: 1
            }
        );

        Ok(())
    }
}
