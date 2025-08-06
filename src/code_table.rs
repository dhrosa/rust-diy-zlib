use std::fmt;

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

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Code {
    bits: u32,
    length: u8,
}

// Test-only covenience method for constructing a Code from a string.
impl From<&str> for Code {
    fn from(s: &str) -> Self {
        Code {
            bits: u32::from_str_radix(s, 2).unwrap(),
            length: s.len() as u8,
        }
    }
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.length == 0 {
            return Ok(());
        }
        write!(f, "{:0width$b}", self.bits, width = self.length as usize)
    }
}

impl Default for Code {
    fn default() -> Self {
        Code { bits: 0, length: 0 }
    }
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
pub struct CodeTable(Vec<Code>);

impl CodeTable {
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
        CodeTable(codes)
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
    fn test_code() {
        assert_eq!(format!("{:?}", Code { bits: 0, length: 0 }), "");
        assert_eq!(format!("{:?}", Code { bits: 2, length: 3 }), "010");
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
            CodeTable::from_code_lengths(code_lengths),
            CodeTable(vec![
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
}
