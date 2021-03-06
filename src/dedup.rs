use crate::op_prelude::*;
use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::hash::{BuildHasher, Hash, Hasher};

pin_project! {
    /// Stream for the [`try_dedup`](super::ext::JTryStreamExt::try_dedup) method
    #[must_use = "streams do nothing unless polled"]
    pub struct TryDedupStream<S> {
        #[pin]
        src: S,
        size_hint: (usize, Option<usize>),
        known: HashSet<u64>,
        hasher: RandomState,
    }
}

impl<S> Stream for TryDedupStream<S>
where
    S: TryStream,
    S::Ok: Hash,
{
    type Item = Result<S::Ok, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        Poll::Ready(loop {
            match ready!(this.src.as_mut().try_poll_next(cx)) {
                Some(Ok(v)) => if this.known.insert(hash(&*this.hasher, &v)) {
                    break Some(Ok(v));
                }
                other => break other,
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.size_hint
    }
}

impl<S> FusedStream for TryDedupStream<S>
where
    S: TryStream + FusedStream,
    S::Ok: Hash,
{
    delegate_fused!(src);
}

#[cfg(feature = "sink")]
impl<S, Item, E> Sink<Item> for TryDedupStream<S>
where
    S: Sink<Item, Error=E> + TryStream,
    S::Ok: Hash
{
    delegate_sink!(src, E, Item);
}

impl<S> TryDedupStream<S>
where
    S: TryStream,
    S::Ok: Hash,
{
    //noinspection DuplicatedCode
    pub(crate) fn new(src: S) -> Self {
        let size_hint = src.size_hint();
        Self {
            src,
            size_hint,
            hasher: RandomState::default(),
            known: HashSet::default(),
        }
    }
}

pin_project! {
    /// Stream for the [`dedup`](super::ext::JStreamExt::dedup) method
    #[must_use = "streams do nothing unless polled"]
    pub struct DedupStream<S> {
        #[pin]
        src: S,
        size_hint: (usize, Option<usize>),
        known: HashSet<u64>,
        hasher: RandomState,
    }
}

impl<S> Stream for DedupStream<S>
where
    S: Stream,
    S::Item: Hash,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        Poll::Ready(loop {
            if let Some(next) = ready!(this.src.as_mut().poll_next(cx)) {
                if this.known.insert(hash(&*this.hasher, &next)) {
                    break Some(next);
                }
            } else {
                break None;
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.size_hint
    }
}

impl<S> FusedStream for DedupStream<S>
where
    S: Stream + FusedStream,
    S::Item: Hash
{
    delegate_fused!(src);
}

#[cfg(feature = "sink")]
impl<S, Item> Sink<Item> for DedupStream<S>
where
    S: Sink<Item> + Stream,
    S::Item: Hash
{
    delegate_sink!(src, S::Error, Item);
}

impl<S> DedupStream<S>
where
    S: Stream,
{
    //noinspection DuplicatedCode
    pub(crate) fn new(src: S) -> Self {
        let size_hint = src.size_hint();
        Self {
            src,
            size_hint,
            hasher: RandomState::default(),
            known: HashSet::default(),
        }
    }
}

fn hash<H>(hasher: &RandomState, value: &H) -> u64
where
    H: Hash,
{
    let mut hasher = hasher.build_hasher();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::TryDedupStream;
    use futures::executor::block_on;
    use futures::TryStreamExt;

    #[test]
    fn test_dedup_simple() {
        let src: Vec<Result<&str, ()>> = vec![
            Ok("hello"),
            Ok("hello"),
            Ok("world!"),
            Ok("world!"),
            Ok("123 123!"),
            Ok("123 123!"),
        ];

        let mut raised = TryDedupStream::new(futures::stream::iter(src));
        assert_eq!(block_on(raised.try_next()), Ok(Some("hello")));
        assert_eq!(block_on(raised.try_next()), Ok(Some("world!")));
        assert_eq!(block_on(raised.try_next()), Ok(Some("123 123!")));
        assert_eq!(block_on(raised.try_next()), Ok(None));
    }

    #[test]
    fn test_dedup_err() {
        let src: Vec<Result<&str, ()>> =
            vec![Ok("hello"), Ok("hello"), Ok("abc z"), Err(()), Ok("abc")];
        let mut raised = TryDedupStream::new(futures::stream::iter(src));
        assert_eq!(block_on(raised.try_next()), Ok(Some("hello")));
        assert_eq!(block_on(raised.try_next()), Ok(Some("abc z")));
        assert_eq!(block_on(raised.try_next()), Err(()));
    }
}
