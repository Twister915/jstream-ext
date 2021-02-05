use crate::op_prelude::*;

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct FuseOnFail<S> {
        #[pin]
        src: S,
        fused: bool,
    }
}

impl<S> Stream for FuseOnFail<S>
where
    S: TryStream,
{
    type Item = Result<S::Ok, S::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::*;

        let this = self.as_mut().project();

        if !*this.fused {
            match futures::ready!(this.src.try_poll_next(cx)) {
                r @ Some(Ok(_)) => Ready(r),
                None => {
                    *this.fused = true;
                    Ready(None)
                }
                Some(Err(err)) => {
                    *this.fused = true;
                    Ready(Some(Err(err)))
                }
            }
        } else {
            Poll::Ready(None)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.src.size_hint()
    }
}

impl<S> FusedStream for FuseOnFail<S>
where
    S: TryStream,
{
    fn is_terminated(&self) -> bool {
        self.fused
    }
}

#[cfg(feature="sink")]
impl<S, Item, E> Sink<Item> for FuseOnFail<S>
where
    S: TryStream + Sink<Item, Error=E>
{
    type Error = E;

    delegate_sink!(src, Item);
}

impl<S> FuseOnFail<S>
where
    S: TryStream,
{
    pub(crate) fn new(src: S) -> Self {
        Self { src, fused: false }
    }
}

#[cfg(test)]
mod tests {
    use super::FuseOnFail;
    use futures::executor::block_on;
    use futures::TryStreamExt;

    #[test]
    fn test_fuse_on_fail() {
        let src = futures::stream::iter(vec![
            Ok("a"),
            Ok("b"),
            Err("oh no!"),
            Ok("shouldn't be there"),
        ]);

        let mut lifted = FuseOnFail::new(src);

        assert_eq!(block_on(lifted.try_next()), Ok(Some("a")));
        assert_eq!(block_on(lifted.try_next()), Ok(Some("b")));
        assert_eq!(block_on(lifted.try_next()), Err("oh no!"));
        assert_eq!(block_on(lifted.try_next()), Ok(None));
    }

    #[test]
    fn test_fuse_on_none() {
        let items: Vec<Result<&str, ()>> = vec![
            Ok("a"),
            Ok("b"),
            Ok("hello"),
        ];

        let src = futures::stream::iter(items);

        let mut lifted = FuseOnFail::new(src);

        assert_eq!(block_on(lifted.try_next()), Ok(Some("a")));
        assert_eq!(block_on(lifted.try_next()), Ok(Some("b")));
        assert_eq!(block_on(lifted.try_next()), Ok(Some("hello")));
        assert_eq!(block_on(lifted.try_next()), Ok(None));
    }
}
