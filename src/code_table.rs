use std::collections::HashMap;
use std::fmt;

type CodeLength = u8;

fn code_length_counts(code_lengths: &[CodeLength]) -> HashMap<CodeLength, u32> {
    let mut counts = HashMap::new();
    for length in code_lengths {
        if *length > 0 {
            let count = counts.entry(*length).or_insert(0);
            *count += 1;
        }
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
        write!(f, "{:0width$b}", self.bits, width = self.length as usize)
    }
}

// Step 2 of algorithm from https://datatracker.ietf.org/doc/html/rfc1951#page-9
fn min_codes_by_length(code_lengths: &[CodeLength]) -> HashMap<CodeLength, Code> {
    let mut min_codes = HashMap::new();
    let mut code_bits = 0;
    let counts = code_length_counts(code_lengths);
    let max_code_length = *code_lengths.iter().max().unwrap();
    for length in 1..=max_code_length {
        code_bits = (code_bits + counts.get(&(length - 1)).unwrap_or(&0)) << 1;
        min_codes.insert(
            length,
            Code {
                bits: code_bits,
                length,
            },
        );
    }
    min_codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_length_counts() {
        // Example from https://datatracker.ietf.org/doc/html/rfc1951#page-9
        let code_lengths = &[3, 3, 3, 3, 3, 2, 4, 4];
        assert_eq!(
            code_length_counts(code_lengths),
            HashMap::from([(2, 1), (3, 5), (4, 2)])
        );
    }

    #[test]
    fn test_code() {
        assert_eq!(format!("{:?}", Code { bits: 2, length: 3 }), "010")
    }

    #[test]
    fn test_min_codes() {
        let code_lengths = &[3, 3, 3, 3, 3, 2, 4, 4];
        assert_eq!(
            min_codes_by_length(code_lengths),
            HashMap::from([
                (1, Code::from("0")),
                (2, Code::from("00")),
                (3, Code::from("010")),
                (4, Code::from("1110")),
            ])
        );
    }
}
