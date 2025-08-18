use std::ops::Index;

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    Literal(u8),
    EndOfBlock,
    BackReference { length: u16, distance: u16 },
}

#[derive(Debug)]
struct History {
    buffer: Vec<u8>,
    start: usize,
    length: usize,
}

impl History {
    fn max_length(&self) -> usize {
        self.buffer.len()
    }

    pub fn new(max_length: usize) -> Self {
        Self {
            buffer: vec![0; max_length],
            start: 0,
            length: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    // Translate a history index to an internal buffer index.
    fn buffer_index(&self, index: usize) -> usize {
        (self.start + index) % self.max_length()
    }

    pub fn append(&mut self, byte: u8) {
        let end = self.buffer_index(self.length);
        self.buffer[end] = byte;

        if self.length < self.max_length() {
            self.length += 1;
        } else {
            self.start += 1;
        }
    }

    pub fn extend(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.append(byte);
        }
    }
}

impl Index<isize> for History {
    type Output = u8;

    fn index(&self, index: isize) -> &u8 {
        if index >= 0 {
            let index = index as usize;
            if index >= self.length {
                panic!("Index out of bounds: {} vs {}", index, self.length);
            }
            return &self.buffer[self.buffer_index(index as usize)];
        }
        // Negative index
        if index < -(self.length as isize) {
            panic!("Index out of bounds: {} vs {}", index, self.length);
        }
        return &self.buffer[self.buffer_index((self.length as isize + index) as usize)];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eviction() {
        let mut history = History::new(3);

        history.append(0);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], 0);

        history.append(1);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], 0);
        assert_eq!(history[1], 1);

        history.append(2);
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], 0);
        assert_eq!(history[1], 1);
        assert_eq!(history[2], 2);

        history.append(3);
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], 1);
        assert_eq!(history[1], 2);
        assert_eq!(history[2], 3);
    }

    #[test]
    fn test_negative_index() {
        let mut history = History::new(3);
        history.extend(&[0, 1, 2]);

        assert_eq!(history[-1], 2);
        assert_eq!(history[-2], 1);
        assert_eq!(history[-3], 0);
    }

    #[test]
    fn test_negative_index_underfull() {
        // One slot is not yet filled.
        let mut history = History::new(3);
        history.extend(&[0, 1]);

        assert_eq!(history[-1], 1);
        assert_eq!(history[-2], 0);
    }
}
