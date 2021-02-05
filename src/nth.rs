use crate::op_prelude::*;
use std::ops::SubAssign;

pin_project! {
    ///
    /// Stream for the [`try_nth`](super::JTryStreamExt::try_nth) method
    ///
    /// Also supports the [`try_first`](super::JTryStreamExt::try_first) method.
    ///
    #[must_use = "streams do nothing unless polled"]
    pub struct TryStreamNth<S> {
        #[pin]
        src: S,
        fused: bool,
        remaining: usize,
    }
}

impl<S> Future for TryStreamNth<S>
where
    S: TryStream,
{
    type Output = Result<Option<S::Ok>, S::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if *this.fused {
            panic!("poll() called after future was already completed...")
        }

        let remaining: &mut usize = this.remaining;

        Poll::Ready(loop {
            match ready!(this.src.as_mut().try_poll_next(cx)) {
                Some(Ok(value)) => {
                    if *remaining == 0 {
                        *this.fused = true;
                        break Ok(Some(value));
                    } else {
                        remaining.sub_assign(1);
                    }
                },
                Some(Err(err)) => break Err(err),
                None => break Ok(None),
            }
        })
    }
}

#[cfg(feature="sink")]
impl<S, Item, E> Sink<Item> for TryStreamNth<S>
where
    S: TryStream + Sink<Item, Error=E>,
{
    delegate_sink!(src, E, Item);
}

impl<S> TryStreamNth<S>
where
    S: TryStream,
{
    pub(crate) fn first(src: S) -> Self {
        Self::new(src, 0)
    }

    pub(crate) fn new(src: S, remaining: usize) -> Self {
        Self { src, fused: false, remaining }
    }
}

pin_project! {
    ///
    /// Stream for the [`nth`](super::JStreamExt::nth) method
    ///
    /// Also supports the [`first`](super::JStreamExt::first) method.
    ///
    #[must_use = "streams do nothing unless polled"]
    pub struct StreamNth<S> {
        #[pin]
        src: S,
        fused: bool,
        remaining: usize,
    }
}

impl<S> Future for StreamNth<S>
where
    S: Stream
{
    type Output = S::Item;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        Poll::Ready(loop {
            if let Some(next) = ready!(this.src.as_mut().poll_next(cx)) {
                if *this.remaining == 0 {
                    break next;
                } else {
                    this.remaining.sub_assign(1);
                }
            }
        })
    }
}

#[cfg(feature="sink")]
impl<S, Item> Sink<Item> for StreamNth<S>
where
    S: Stream + Sink<Item>,
{
    delegate_sink!(src, S::Error, Item);
}

impl<S> StreamNth<S>
where
    S: Stream
{
    pub(crate) fn first(src: S) -> Self {
        Self::new(src, 0)
    }

    pub(crate) fn new(src: S, remaining: usize) -> Self {
        Self { src, fused: false, remaining }
    }
}

#[cfg(test)]
mod tests {
    use super::TryStreamNth;
    use futures::executor::block_on;

    #[test]
    fn test_try_stream_first() {
        let items: Vec<Result<&str, ()>> = vec![Ok("hello!"), Ok("should not show up")];
        let src = futures::stream::iter(items);
        let raised = TryStreamNth::first(src);
        assert_eq!(block_on(raised), Ok(Some("hello!")));
    }

    #[test]
    fn test_try_stream_nothing() {
        let src = futures::stream::empty::<Result<(), ()>>();
        let raised = TryStreamNth::first(src);
        assert_eq!(block_on(raised), Ok(None));
    }
}
