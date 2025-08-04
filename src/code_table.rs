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

#[derive(PartialEq, Eq, Hash)]
struct Code {
    bits: u32,
    length: u8,
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:0width$b}", self.bits, width = self.length as usize)
    }
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
}
