use crate::op_prelude::*;

pin_project! {
    /// Stream for the [`try_filter_map_ok`](super::JTryStreamExt::try_filter_map_ok) method
    #[must_use = "streams do nothing unless polled"]
    pub struct TryFilterMapOk<S, F, R> {
        #[pin]
        src: S,
        predicate: F,
        _rt: PhantomData<R>,
    }
}

impl<S, F, R> Stream for TryFilterMapOk<S, F, R>
where
    S: TryStream,
    F: FnMut(S::Ok) -> Option<R>,
{
    type Item = Result<R, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        Poll::Ready(loop {
            match ready!(this.src.as_mut().try_poll_next(cx)) {
                Some(Ok(next)) => if let Some(out) = (this.predicate)(next) {
                    break Some(Ok(out));
                }
                Some(Err(err)) => break Some(Err(err)),
                None => break None,
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.src.size_hint()
    }
}

impl<S, F, R> FusedStream for TryFilterMapOk<S, F, R>
where
    S: TryStream + FusedStream,
    F: FnMut(S::Ok) -> Option<R>,
{
    delegate_fused!(src);
}

#[cfg(feature="sink")]
impl<S, F, R, Item, E> Sink<Item> for TryFilterMapOk<S, F, R>
where
    S: TryStream + FusedStream + Sink<Item, Error=E>,
    F: FnMut(S::Ok) -> Option<R>,
{
    delegate_sink!(src, E, Item);
}

impl<S, F, R> TryFilterMapOk<S, F, R>
where
    S: TryStream,
    F: FnMut(S::Ok) -> Option<R>,
{
    pub(crate) fn new(src: S, predicate: F) -> Self {
        Self {
            src,
            predicate,
            _rt: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TryFilterMapOk;
    use futures::executor::block_on;
    use futures::TryStreamExt;

    #[test]
    fn test_filter_map_ok_simple() {
        let items: Vec<Result<&str, ()>> = vec![Ok("hello"), Ok(""), Ok("world!")];
        let src = futures::stream::iter(items);
        let mut raised = TryFilterMapOk::new(src, filter_empty);
        assert_eq!(block_on(raised.try_next()), Ok(Some("hello".to_owned())));
        assert_eq!(block_on(raised.try_next()), Ok(Some("world!".to_owned())));
        assert_eq!(block_on(raised.try_next()), Ok(None));
    }

    #[test]
    fn test_filter_map_ok_error() {
        let items: Vec<Result<&str, ()>> =
            vec![Ok("hello"), Ok(""), Ok("world!"), Err(()), Ok("test 123")];
        let src = futures::stream::iter(items);
        let mut raised = TryFilterMapOk::new(src, filter_empty);
        assert_eq!(block_on(raised.try_next()), Ok(Some("hello".to_owned())));
        assert_eq!(block_on(raised.try_next()), Ok(Some("world!".to_owned())));
        assert_eq!(block_on(raised.try_next()), Err(()));
    }

    fn filter_empty(v: &str) -> Option<String> {
        if !v.is_empty() {
            Some(v.to_string())
        } else {
            None
        }
    }
}
