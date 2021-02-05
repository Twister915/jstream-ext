use crate::op_prelude::*;

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct FuseOnFail<S> {
        #[pin]
        src: S,
        fused: bool,
        failed: bool,
    }
}

impl<S> Stream for FuseOnFail<S>
where
    S: TryStream,
{
    type Item = Result<S::Ok, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if *this.fused {
            panic!("poll after fused!")
        }

        Poll::Ready({
            if *this.failed {
                *this.fused = true;
                None
            } else {
                match ready!(this.src.try_poll_next(cx)) {
                    r @ Some(Ok(_)) => r,
                    None => {
                        *this.fused = true;
                        None
                    }
                    Some(Err(err)) => {
                        *this.failed = true;
                        Some(Err(err))
                    }
                }
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.src.size_hint()
    }
}

impl<S> FusedStream for FuseOnFail<S> where S: TryStream {
    fn is_terminated(&self) -> bool {
        self.fused
    }
}

#[cfg(feature="sink")]
impl<S, Item, E> Sink<Item> for FuseOnFail<S>
where
    S: TryStream + Sink<Item, Error=E>
{
    delegate_sink!(src, E, Item);
}

impl<S> FuseOnFail<S>
where
    S: TryStream,
{
    pub(crate) fn new(src: S) -> Self {
        Self { src, fused: false, failed: false }
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
