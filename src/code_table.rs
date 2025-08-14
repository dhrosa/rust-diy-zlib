use crate::bit_reader::{BitRead, BitReader};
use crate::bit_string::bit_string;
use crate::code::Code;
use crate::error::{InflateError, InflateResult};
use std::collections::HashMap;
use std::io;

pub type CodeLength = u8;

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

    pub fn fixed_ll() -> Self {
        let mut code_lengths: [CodeLength; 288] = [8; 288];
        code_lengths[144..=255].fill(9);
        code_lengths[256..=279].fill(7);
        Self::from_code_lengths(&code_lengths)
    }

    pub fn fixed_distance() -> Self {
        Self::from_code_lengths(&[5; 32])
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
    pub fn fixed_ll() -> Self {
        SymbolToCodeTable::fixed_ll().inverse()
    }

    pub fn fixed_distance() -> Self {
        SymbolToCodeTable::fixed_distance().inverse()
    }

    pub fn from_code_lengths(code_lengths: &[CodeLength]) -> Self {
        SymbolToCodeTable::from_code_lengths(code_lengths).inverse()
    }

    pub fn read_symbol(&self, reader: &mut impl BitRead) -> InflateResult<u32> {
        let mut code = Code::default();
        loop {
            if let Some(&symbol) = self.0.get(&code) {
                return Ok(symbol);
            }
            code = code.append_bit(reader.read_bit()?);
        }
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
    fn test_empty_codes() {
        let code_lengths = &[2, 0, 1, 0, 3, 3];
        assert_eq!(
            SymbolToCodeTable::from_code_lengths(code_lengths),
            SymbolToCodeTable(vec![
                Code::from("10"),
                Code::default(),
                Code::from("0"),
                Code::default(),
                Code::from("110"),
                Code::from("111"),
            ])
        );
    }

    #[test]
    fn test_fixed_ll_table() {
        let SymbolToCodeTable(fixed) = SymbolToCodeTable::fixed_ll();
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
    fn test_fixed_distance_table() {
        let SymbolToCodeTable(fixed) = SymbolToCodeTable::fixed_distance();
        assert_eq!(fixed[0], Code::from("00000"));
        assert_eq!(fixed[3], Code::from("00011"));
        assert_eq!(fixed[9], Code::from("01001"));
        assert_eq!(fixed[27], Code::from("11011"));
        assert_eq!(fixed[31], Code::from("11111"));
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
}
