use crate::bit_reader::BitRead;
use crate::code_table::{CodeLength, CodeToSymbolTable};
use crate::error::{InflateError, InflateResult};
use crate::lz77::Instruction;

struct BlockDecoder<'a, R: BitRead> {
    reader: &'a mut R,
    ll_table: CodeToSymbolTable,
    distance_table: CodeToSymbolTable,
}

fn push_repeated<T: Copy>(v: &mut Vec<T>, value: T, count: usize) {
    for _ in 0..count {
        v.push(value);
    }
}

impl<'a, R: BitRead> BlockDecoder<'a, R> {
    // Decoder for block type 1 (fixed codes).
    pub fn new_fixed(reader: &'a mut R) -> Self {
        Self {
            reader,
            ll_table: CodeToSymbolTable::fixed_ll(),
            distance_table: CodeToSymbolTable::fixed_distance(),
        }
    }

    // Decoder for block type 2 (dynamic codes).
    pub fn new_dynamic(reader: &'a mut R) -> InflateResult<Self> {
        let ll_count = reader.read_bits::<usize>(5)? + 257;
        let distance_count = reader.read_bits::<usize>(5)? + 1;
        let cl_count = reader.read_bits::<usize>(4)? + 4;

        // Construct CL table.
        let cl_table: CodeToSymbolTable;
        {
            let mut cl_code_lengths = [0; 19];
            let cl_indexes = [
                16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
            ];
            for i in 0..cl_count {
                let cl_code_length = reader.read_bits::<u8>(3)?;
                let index = cl_indexes[i];
                cl_code_lengths[index] = cl_code_length;
            }
            cl_table = CodeToSymbolTable::from_code_lengths(&cl_code_lengths);
        }

        // Use CL table to decode LL and distance code lengths.
        let mut code_lengths = Vec::<CodeLength>::new();
        while code_lengths.len() < ll_count + distance_count {
            let symbol = cl_table.read_symbol(reader)?;
            if symbol <= 15 {
                // Verbatim length
                code_lengths.push(symbol as CodeLength);
            } else if symbol == 16 {
                // Repeat previous length
                let count = 3 + reader.read_bits::<usize>(2)?;
                if let Some(&length) = code_lengths.last() {
                    push_repeated(&mut code_lengths, length, count);
                } else {
                    return Err(InflateError::DynamicCodeMalformed);
                }
            } else if symbol == 17 {
                let count = 3 + reader.read_bits::<usize>(3)?;
                push_repeated(&mut code_lengths, 0, count);
            } else if symbol == 18 {
                let count = 11 + reader.read_bits::<usize>(7)?;
                push_repeated(&mut code_lengths, 0, count);
            }
        }

        let mut ll_lengths = [0; 288];
        for i in 0..ll_count {
            ll_lengths[i] = code_lengths[i];
        }
        let mut distance_lengths = [0; 32];
        for i in 0..distance_count {
            distance_lengths[i] = code_lengths[ll_count + i];
        }
        Ok(Self {
            reader,
            ll_table: CodeToSymbolTable::from_code_lengths(&ll_lengths),
            distance_table: CodeToSymbolTable::from_code_lengths(&distance_lengths),
        })
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
