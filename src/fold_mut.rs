use crate::op_prelude::*;
use futures::Sink;

const POLL_AFTER_COMPLETED_MSG: &'static str = "call to poll after completed!";

pin_project! {
    #[must_use = "futures do nothing unless polled"]
    pub struct TryFoldMut<S, T, F, Fut> {
        #[pin]
        upstream: S,
        #[pin]
        pending_future: Option<Fut>,
        state: Option<T>,
        handler: F,
    }
}

impl<S, T, F, Fut> TryFoldMut<S, T, F, Fut>
where
    S: TryStream + FusedStream,
    F: FnMut(&mut T, S::Ok) -> Fut,
    Fut: TryFuture<Ok=(), Error=S::Error>,
{
    pub(crate) fn new(upstream: S, initial: T, handler: F) -> Self {
        Self {
            upstream,
            pending_future: None,
            state: Some(initial),
            handler,
        }
    }
}

impl<S, T, F, Fut> Future for TryFoldMut<S, T, F, Fut>
where
    S: TryStream + FusedStream,
    F: FnMut(&mut T, S::Ok) -> Fut,
    Fut: TryFuture<Ok=(), Error=S::Error>,
{
    type Output = Result<T, S::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        Poll::Ready(loop {
            // poll future if we have one
            if let Some(future) = this.pending_future.as_mut().as_pin_mut() {
                let out = futures::ready!(future.try_poll(cx));
                this.pending_future.set(None);
                if let Err(err) = out {
                    this.state.take();
                    break Err(err);
                }
            }

            // poll upstream
            match futures::ready!(this.upstream.as_mut().try_poll_next(cx)) {
                // got something, no error
                Some(Ok(next)) => {
                    let state = this.state.as_mut().expect(POLL_AFTER_COMPLETED_MSG);
                    let future = (this.handler)(state, next);
                    this.pending_future.set(Some(future));
                }
                // got error
                Some(Err(err)) => {
                    this.state.take();
                    break Err(err);
                },
                // upstream done
                None => {
                    break Ok(this.state.take().expect(POLL_AFTER_COMPLETED_MSG));
                }
            }
        })
    }
}

#[cfg(feature = "sink")]
impl<S, T, F, Fut, Item, E> Sink<Item> for TryFoldMut<S, T, F, Fut>
where
    S: Sink<Item, Error=E> + Stream + FusedStream,
    F: FnMut(&mut T, S::Item) -> Fut,
    Fut: Future<Output=()>,
{

    type Error = E;

    delegate_sink!(upstream, Item);
}

pin_project! {
    #[must_use = "futures do nothing unless polled"]
    pub struct FoldMut<S, T, F, Fut> {
        #[pin]
        upstream: S,
        #[pin]
        pending_future: Option<Fut>,
        state: Option<T>,
        handler: F,
    }
}

impl<S, T, F, Fut> FoldMut<S, T, F, Fut>
where
    S: Stream + FusedStream,
    F: FnMut(&mut T, S::Item) -> Fut,
    Fut: Future<Output=()>,
{
    pub(crate) fn new(upstream: S, initial: T, handler: F) -> Self {
        Self {
            upstream,
            pending_future: None,
            state: Some(initial),
            handler,
        }
    }
}


impl<S, T, F, Fut> Future for FoldMut<S, T, F, Fut>
where
    S: Stream + FusedStream,
    F: FnMut(&mut T, S::Item) -> Fut,
    Fut: Future<Output=()>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        Poll::Ready(loop {
            // poll future if we have one
            if let Some(future) = this.pending_future.as_mut().as_pin_mut() {
                futures::ready!(future.poll(cx));
                this.pending_future.set(None);
            }

            // poll upstream
            match futures::ready!(this.upstream.as_mut().poll_next(cx)) {
                // got next item
                Some(next) => {
                    let state = this.state.as_mut().expect(POLL_AFTER_COMPLETED_MSG);
                    let future = (this.handler)(state, next);
                    this.pending_future.set(Some(future));
                }
                // upstream done
                None => {
                    break this.state.take().expect(POLL_AFTER_COMPLETED_MSG)
                }
            }

        })
    }
}

#[cfg(feature = "sink")]
impl<S, T, F, Fut, Item> Sink<Item> for FoldMut<S, T, F, Fut>
where
    S: Sink<Item> + Stream + FusedStream,
    F: FnMut(&mut T, S::Item) -> Fut,
    Fut: Future<Output=()>,
{

    type Error = S::Error;

    delegate_sink!(upstream, Item);
}