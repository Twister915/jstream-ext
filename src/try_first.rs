use crate::op_prelude::*;

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct TryStreamFirst<S> {
        #[pin]
        src: S,
        fused: bool,
    }
}

impl<S> Future for TryStreamFirst<S>
where
    S: TryStream,
{
    type Output = Result<Option<S::Ok>, S::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.fused {
            panic!("poll() called after future was already completed...")
        }

        Poll::Ready({
            let out = ready!(this.src.try_poll_next(cx));
            *this.fused = true;

            match out {
                Some(Ok(value)) => Ok(Some(value)),
                Some(Err(err)) => Err(err),
                None => Ok(None),
            }
        })
    }
}

#[cfg(feature="sink")]
impl<S, Item, E> Sink<Item> for TryStreamFirst<S>
where
    S: TryStream + Sink<Item, Error=E>,
{
    delegate_sink!(src, E, Item);
}

impl<S> TryStreamFirst<S>
where
    S: TryStream,
{
    pub(crate) fn new(src: S) -> Self {
        Self { src, fused: false }
    }
}

#[cfg(test)]
mod tests {
    use super::TryStreamFirst;
    use futures::executor::block_on;

    #[test]
    fn test_try_stream_first() {
        let items: Vec<Result<&str, ()>> = vec![Ok("hello!"), Ok("should not show up")];
        let src = futures::stream::iter(items);
        let raised = TryStreamFirst::new(src);
        assert_eq!(block_on(raised), Ok(Some("hello!")));
    }

    #[test]
    fn test_try_stream_nothing() {
        let src = futures::stream::empty::<Result<(), ()>>();
        let raised = TryStreamFirst::new(src);
        assert_eq!(block_on(raised), Ok(None));
    }
}
