use winnow::stream::AsBStr;
use winnow::stream::AsBytes;
use winnow::stream::Compare;
use winnow::stream::CompareResult;
use winnow::stream::FindSlice;
use winnow::stream::Location;
use winnow::stream::Needed;
use winnow::stream::Offset;
use winnow::stream::SliceLen;
use winnow::stream::Stream;
use winnow::stream::StreamIsPartial;
use winnow::stream::UpdateSlice;

pub use winnow::stream::{LocatingSlice, Stateful};

/// Parser input that snapshots parser state together with the stream position.
///
/// Unlike winnow's `Stateful`, `checkpoint()` clones the user state so
/// backtracking restores flags/counters alongside the input position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Input<'i, S> {
    pub input: LocatingSlice<&'i str>,
    pub state: S,
    furthest_pos: usize,
}

#[derive(Clone, Debug)]
pub struct InputCheckpoint<'i, S> {
    input: <LocatingSlice<&'i str> as Stream>::Checkpoint,
    state: S,
}

impl<'i, S> InputCheckpoint<'i, S> {
    fn new(input: <LocatingSlice<&'i str> as Stream>::Checkpoint, state: S) -> Self {
        Self { input, state }
    }
}

impl<'i, S> Input<'i, S> {
    pub fn new(source: &'i str, state: S) -> Self {
        Self {
            input: LocatingSlice::new(source),
            state,
            furthest_pos: 0,
        }
    }

    pub fn track_pos(&mut self, pos: usize) {
        if pos > self.furthest_pos {
            self.furthest_pos = pos;
        }
    }

    pub fn furthest_pos(&self) -> usize {
        self.furthest_pos
    }
}

impl<'i, S> AsRef<LocatingSlice<&'i str>> for Input<'i, S> {
    #[inline(always)]
    fn as_ref(&self) -> &LocatingSlice<&'i str> {
        &self.input
    }
}

impl<'i, S> core::ops::Deref for Input<'i, S> {
    type Target = LocatingSlice<&'i str>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'i, S> core::fmt::Display for Input<'i, S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.input.fmt(f)
    }
}

impl<'i, S> SliceLen for Input<'i, S> {
    #[inline(always)]
    fn slice_len(&self) -> usize {
        self.input.slice_len()
    }
}

impl<'i, S> Stream for Input<'i, S>
where
    S: Clone + core::fmt::Debug,
{
    type Token = <LocatingSlice<&'i str> as Stream>::Token;
    type Slice = <LocatingSlice<&'i str> as Stream>::Slice;
    type IterOffsets = <LocatingSlice<&'i str> as Stream>::IterOffsets;
    type Checkpoint = InputCheckpoint<'i, S>;

    #[inline(always)]
    fn iter_offsets(&self) -> Self::IterOffsets {
        self.input.iter_offsets()
    }

    #[inline(always)]
    fn eof_offset(&self) -> usize {
        self.input.eof_offset()
    }

    #[inline(always)]
    fn next_token(&mut self) -> Option<Self::Token> {
        self.input.next_token()
    }

    #[inline(always)]
    fn peek_token(&self) -> Option<Self::Token> {
        self.input.peek_token()
    }

    #[inline(always)]
    fn offset_for<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Token) -> bool,
    {
        self.input.offset_for(predicate)
    }

    #[inline(always)]
    fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
        self.input.offset_at(tokens)
    }

    #[inline(always)]
    fn next_slice(&mut self, offset: usize) -> Self::Slice {
        self.input.next_slice(offset)
    }

    #[inline(always)]
    unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
        unsafe { self.input.next_slice_unchecked(offset) }
    }

    #[inline(always)]
    fn peek_slice(&self, offset: usize) -> Self::Slice {
        self.input.peek_slice(offset)
    }

    #[inline(always)]
    unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
        unsafe { self.input.peek_slice_unchecked(offset) }
    }

    #[inline(always)]
    fn checkpoint(&self) -> Self::Checkpoint {
        InputCheckpoint::new(self.input.checkpoint(), self.state.clone())
    }

    #[inline(always)]
    fn reset(&mut self, checkpoint: &Self::Checkpoint) {
        self.input.reset(&checkpoint.input);
        self.state = checkpoint.state.clone();
    }

    fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.input.trace(f)
    }
}

impl<'i, S> Offset for Input<'i, S>
where
    S: Clone + core::fmt::Debug,
{
    #[inline(always)]
    fn offset_from(&self, start: &Self) -> usize {
        self.offset_from(&start.checkpoint())
    }
}

impl<'i, S> Offset<<Input<'i, S> as Stream>::Checkpoint> for Input<'i, S>
where
    S: Clone + core::fmt::Debug,
{
    #[inline(always)]
    fn offset_from(&self, other: &<Input<'i, S> as Stream>::Checkpoint) -> usize {
        self.input.offset_from(&other.input)
    }
}

impl<'i, S> Offset for InputCheckpoint<'i, S> {
    #[inline(always)]
    fn offset_from(&self, start: &Self) -> usize {
        self.input.offset_from(&start.input)
    }
}

impl<'i, S> Location for Input<'i, S> {
    #[inline(always)]
    fn previous_token_end(&self) -> usize {
        self.input.previous_token_end()
    }

    #[inline(always)]
    fn current_token_start(&self) -> usize {
        self.input.current_token_start()
    }
}

impl<'i, S> StreamIsPartial for Input<'i, S> {
    type PartialState = <LocatingSlice<&'i str> as StreamIsPartial>::PartialState;

    #[inline(always)]
    fn complete(&mut self) -> Self::PartialState {
        self.input.complete()
    }

    #[inline(always)]
    fn restore_partial(&mut self, state: Self::PartialState) {
        self.input.restore_partial(state);
    }

    #[inline(always)]
    fn is_partial_supported() -> bool {
        <LocatingSlice<&'i str> as StreamIsPartial>::is_partial_supported()
    }

    #[inline(always)]
    fn is_partial(&self) -> bool {
        self.input.is_partial()
    }
}

impl<'i, S> AsBytes for Input<'i, S> {
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self.input.as_bytes()
    }
}

impl<'i, S> AsBStr for Input<'i, S> {
    #[inline(always)]
    fn as_bstr(&self) -> &[u8] {
        self.input.as_bstr()
    }
}

impl<'i, S, U> Compare<U> for Input<'i, S>
where
    LocatingSlice<&'i str>: Compare<U>,
{
    #[inline(always)]
    fn compare(&self, other: U) -> CompareResult {
        self.input.compare(other)
    }
}

impl<'i, S, T> FindSlice<T> for Input<'i, S>
where
    LocatingSlice<&'i str>: FindSlice<T>,
{
    #[inline(always)]
    fn find_slice(&self, substr: T) -> Option<core::ops::Range<usize>> {
        self.input.find_slice(substr)
    }
}

impl<'i, S> UpdateSlice for Input<'i, S>
where
    S: Clone + core::fmt::Debug,
{
    #[inline(always)]
    fn update_slice(mut self, inner: Self::Slice) -> Self {
        self.input = UpdateSlice::update_slice(self.input, inner);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Input, LocatingSlice};
    use winnow::stream::{Location, Stream};

    #[test]
    fn checkpoint_restores_state_and_position() {
        let mut input = Input {
            input: LocatingSlice::new("abc"),
            state: 1usize,
            furthest_pos: 0,
        };

        let checkpoint = input.checkpoint();
        assert_eq!(input.next_token(), Some('a'));
        input.state = 9;

        input.reset(&checkpoint);

        assert_eq!(input.state, 1);
        assert_eq!(input.current_token_start(), 0);
    }
}
