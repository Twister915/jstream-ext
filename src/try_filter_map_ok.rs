use futures::stream::FusedStream;
use futures::task::{Context, Poll};
use futures::{Stream, TryStream};
use pin_project_lite::pin_project;
use std::marker::PhantomData;
use std::pin::Pin;

pin_project! {
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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::*;
        loop {
            let this = self.as_mut().project();

            match futures::ready!(this.src.try_poll_next(cx)) {
                Some(Ok(next)) => {
                    if let Some(out) = (this.predicate)(next) {
                        return Ready(Some(Ok(out)));
                    }
                }
                Some(Err(err)) => return Ready(Some(Err(err))),
                None => return Ready(None),
            }
        }
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
    fn is_terminated(&self) -> bool {
        self.src.is_terminated()
    }
}

impl<S, F, R> TryFilterMapOk<S, F, R>
where
    S: TryStream,
    F: FnMut(S::Ok) -> Option<R>,
{
    pub fn new(src: S, predicate: F) -> Self {
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
