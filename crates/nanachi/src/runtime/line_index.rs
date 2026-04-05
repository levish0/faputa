/// Precomputed line start offsets for O(log n) byte-offset → line:col conversion.
///
/// Build once with `LineIndex::new(source)`, then query with `line_col(offset)`.
#[derive(Clone, Debug)]
pub struct LineIndex {
    /// Byte offset of each line's first character. Always starts with `[0]`.
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Build a line index from source text. O(n), SIMD-accelerated via memchr.
    pub fn new(text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut line_starts = vec![0];
        let mut i = 0;
        while let Some(pos) = memchr::memchr(b'\n', &bytes[i..]) {
            i += pos + 1;
            line_starts.push(i);
        }
        Self { line_starts }
    }

    /// Convert a byte offset to 1-based (line, column).
    ///
    /// Column counts bytes from the start of the line (1-based).
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let line_idx = self.line_starts.partition_point(|&start| start <= offset);
        let line_idx = line_idx.saturating_sub(1);
        let line_start = self.line_starts[line_idx];
        (line_idx + 1, offset - line_start + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line() {
        let idx = LineIndex::new("hello");
        assert_eq!(idx.line_col(0), (1, 1));
        assert_eq!(idx.line_col(4), (1, 5));
    }

    #[test]
    fn multi_line() {
        let idx = LineIndex::new("aaa\nbbb\nccc");
        assert_eq!(idx.line_col(0), (1, 1)); // 'a'
        assert_eq!(idx.line_col(3), (1, 4)); // '\n'
        assert_eq!(idx.line_col(4), (2, 1)); // 'b'
        assert_eq!(idx.line_col(8), (3, 1)); // 'c'
    }

    #[test]
    fn empty() {
        let idx = LineIndex::new("");
        assert_eq!(idx.line_col(0), (1, 1));
    }

    #[test]
    fn trailing_newline() {
        let idx = LineIndex::new("abc\n");
        assert_eq!(idx.line_col(3), (1, 4));
        assert_eq!(idx.line_col(4), (2, 1));
    }

    #[test]
    fn unicode_columns_are_byte_based() {
        let idx = LineIndex::new("a한\nz");

        assert_eq!(idx.line_col(1), (1, 2));
        assert_eq!(idx.line_col(4), (1, 5));
        assert_eq!(idx.line_col(5), (2, 1));
    }
}
