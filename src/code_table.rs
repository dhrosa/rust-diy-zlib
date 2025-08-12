use crate::bit_reader::BitReader;
use crate::bit_string::bit_string;
use crate::code::Code;
use crate::error::{InflateError, InflateResult};
use crate::lz77::Instruction;
use std::collections::HashMap;
use std::io;

type CodeLength = u8;

// Each index is a code length, each value is the number of code lengths of that
// value. The [0] value is always 0.
fn code_length_counts(code_lengths: &[CodeLength]) -> Vec<u32> {
    let max_code_length = *code_lengths.iter().max().unwrap();
    let mut counts = vec![0; (max_code_length as usize) + 1];
    for &length in code_lengths {
        if length == 0 {
            continue;
        }
        counts[length as usize] += 1;
    }
    counts
}

// Step 2 of algorithm from https://datatracker.ietf.org/doc/html/rfc1951#page-9
fn min_codes_by_length(code_lengths: &[CodeLength]) -> Vec<Code> {
    let mut min_codes = vec![Code { bits: 0, length: 0 }];
    let mut code_bits = 0;
    let counts = code_length_counts(code_lengths);
    let max_code_length = *code_lengths.iter().max().unwrap();
    for length in 1..=max_code_length {
        code_bits = (code_bits + counts.get((length - 1) as usize).unwrap()) << 1;
        min_codes.push(Code {
            bits: code_bits,
            length,
        });
    }
    min_codes
}

#[derive(Debug, PartialEq, Eq)]
pub struct SymbolToCodeTable(Vec<Code>);

impl SymbolToCodeTable {
    pub fn from_code_lengths(code_lengths: &[CodeLength]) -> Self {
        let mut codes = Vec::new();
        for &length in code_lengths {
            codes.push(Code {
                bits: 0,
                length: length,
            });
        }
        let mut next_codes = min_codes_by_length(code_lengths);
        // Step 3 of algorithm from https://datatracker.ietf.org/doc/html/rfc1951#page-9
        for code in codes.iter_mut() {
            if code.length == 0 {
                continue;
            }
            let next_code = next_codes.get_mut(code.length as usize).unwrap();
            code.bits = next_code.bits;
            next_code.bits += 1;
        }
        SymbolToCodeTable(codes)
    }

    pub fn fixed() -> Self {
        let mut code_lengths: [CodeLength; 288] = [8; 288];
        code_lengths[144..=255].fill(9);
        code_lengths[256..=279].fill(7);
        Self::from_code_lengths(&code_lengths)
    }

    pub fn inverse(&self) -> CodeToSymbolTable {
        let mut inverse = HashMap::new();
        for (symbol, code) in self.0.iter().enumerate() {
            inverse.insert(*code, symbol as u32);
        }
        CodeToSymbolTable(inverse)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CodeToSymbolTable(HashMap<Code, u32>);

impl CodeToSymbolTable {
    fn fixed() -> Self {
        SymbolToCodeTable::fixed().inverse()
    }

    fn read_symbol<R: io::Read>(&self, reader: &mut BitReader<R>) -> InflateResult<u32> {
        let mut code = Code::default();
        loop {
            if let Some(&symbol) = self.0.get(&code) {
                return Ok(symbol);
            }
            code = code.append_bit(reader.read_bit()?);
        }
    }

    fn read_instruction<R: io::Read>(
        &self,
        reader: &mut BitReader<R>,
    ) -> InflateResult<Instruction> {
        let symbol = self.read_symbol(reader)? as u16;
        if symbol < 256 {
            return Ok(Instruction::Literal(symbol as u8));
        }
        if symbol == 256 {
            return Ok(Instruction::EndOfBlock);
        }
        let length = self.read_length(symbol, reader)?;
        let distance = self.read_distance(reader)?;
        Ok(Instruction::BackReference { length, distance })
    }

    fn read_length<R: io::Read>(
        &self,
        symbol: u16,
        reader: &mut BitReader<R>,
    ) -> InflateResult<u16> {
        // Borrowed from
        // https://github.com/nayuki/Simple-DEFLATE-decompressor/blob/2586b459a84f8918851a1078c2c0482b1b383fba/python/deflatedecompress.py#L439
        if symbol <= 264 {
            return Ok(symbol - 254);
        }
        if symbol <= 284 {
            let extra_bit_count = (symbol - 261) / 4;
            let extra_bits = reader.read_bits::<u16>(extra_bit_count as u8)?;
            let base = ((symbol - 265) % 4 + 4) << extra_bit_count;
            return Ok(3 + base + extra_bits);
        }
        if symbol == 285 {
            return Ok(258);
        }
        Err(InflateError::InvalidLengthSymbol(symbol))
    }

    fn read_distance<R: io::Read>(&self, reader: &mut BitReader<R>) -> InflateResult<u16> {
        // Borrowed from https://github.com/nayuki/Simple-DEFLATE-decompressor/blob/2586b459a84f8918851a1078c2c0482b1b383fba/python/deflatedecompress.py#L456
        let symbol = reader.read_bits::<u16>(5)?;
        if symbol <= 3 {
            return Ok(symbol + 1);
        }
        if symbol <= 29 {
            let extra_bit_count = symbol / 2 + 1;
            let extra_bits = reader.read_bits::<u16>(extra_bit_count as u8)?;
            let base = (symbol % 2 + 2) << extra_bit_count;
            return Ok(1 + base + extra_bits);
        }
        Err(InflateError::InvalidDistanceSymbol(symbol as u8))
    }
}

impl<const N: usize> From<[(Code, u32); N]> for CodeToSymbolTable {
    fn from(pairs: [(Code, u32); N]) -> Self {
        Self(HashMap::from(pairs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_length_counts() {
        // Example from https://datatracker.ietf.org/doc/html/rfc1951#page-9
        let code_lengths = &[3, 3, 3, 3, 3, 2, 4, 4];
        assert_eq!(code_length_counts(code_lengths), vec![0, 0, 1, 5, 2]);
    }

    #[test]
    fn test_min_codes() {
        let code_lengths = &[3, 3, 3, 3, 3, 2, 4, 4];
        assert_eq!(
            min_codes_by_length(code_lengths),
            vec![
                Code::default(),
                Code::from("0"),
                Code::from("00"),
                Code::from("010"),
                Code::from("1110")
            ]
        );
    }

    #[test]
    fn test_from_code_lengths() {
        let code_lengths = &[3, 3, 3, 3, 3, 2, 4, 4];
        assert_eq!(
            SymbolToCodeTable::from_code_lengths(code_lengths),
            SymbolToCodeTable(vec![
                Code::from("010"),
                Code::from("011"),
                Code::from("100"),
                Code::from("101"),
                Code::from("110"),
                Code::from("00"),
                Code::from("1110"),
                Code::from("1111"),
            ])
        );
    }

    #[test]
    fn test_fixed_table() {
        let SymbolToCodeTable(fixed) = SymbolToCodeTable::fixed();
        assert_eq!(fixed[0], Code::from("00110000"));
        assert_eq!(fixed[143], Code::from("10111111"));
        assert_eq!(fixed[144], Code::from("110010000"));
        assert_eq!(fixed[255], Code::from("111111111"));
        assert_eq!(fixed[256], Code::from("0000000"));
        assert_eq!(fixed[279], Code::from("0010111"));
        assert_eq!(fixed[280], Code::from("11000000"));
        assert_eq!(fixed[287], Code::from("11000111"));
    }

    #[test]
    fn test_inverse() {
        let code_lengths = &[1, 2, 2];
        let table = SymbolToCodeTable::from_code_lengths(code_lengths);
        assert_eq!(
            table.inverse(),
            CodeToSymbolTable::from([
                (Code::from("0"), 0),
                (Code::from("10"), 1),
                (Code::from("11"), 2),
            ])
        )
    }

    #[test]
    fn test_read_code() -> InflateResult<()> {
        let table = CodeToSymbolTable::from([
            (Code::from("0"), 0),
            (Code::from("10"), 1),
            (Code::from("11"), 2),
        ]);
        let raw: &[u8] = &[0b010_11_01_0];
        let mut reader = BitReader::new(raw);
        assert_eq!(table.read_symbol(&mut reader)?, 0);
        assert_eq!(table.read_symbol(&mut reader)?, 1);
        assert_eq!(table.read_symbol(&mut reader)?, 2);
        assert_eq!(reader.read_bits::<u8>(3)?, 0b010);
        Ok(())
    }

    #[test]
    fn test_read_literal() -> InflateResult<()> {
        // 0 is 8-bit code: 00110000
        // 144 is 9-bit code: 110010000
        let raw = bit_string("0000 1100 0001 0011 0");
        let mut reader = BitReader::new(raw.as_slice());
        let table = CodeToSymbolTable::fixed();
        assert_eq!(
            table.read_instruction(&mut reader)?,
            Instruction::Literal(0)
        );
        assert_eq!(
            table.read_instruction(&mut reader)?,
            Instruction::Literal(144)
        );
        Ok(())
    }

    #[test]
    fn test_end_of_block() -> InflateResult<()> {
        // end of block is 7-bit code: 000 0000.
        let raw = bit_string("1000 0000");
        let mut reader = BitReader::new(raw.as_slice());
        let table = CodeToSymbolTable::fixed();
        assert_eq!(
            table.read_instruction(&mut reader)?,
            Instruction::EndOfBlock,
        );
        Ok(())
    }

    #[test]
    fn test_back_reference() -> InflateResult<()> {
        use super::Instruction::*;

        let raw = bit_string("00110000 00000000 00000000");
        let mut reader = BitReader::new(raw.as_slice());
        let table = CodeToSymbolTable::fixed();
        assert_eq!(
            table.read_instruction(&mut reader)?,
            BackReference {
                length: 8,
                distance: 1
            }
        );

        Ok(())
    }
}
