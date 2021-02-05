use crate::ops::*;
use futures::stream::FusedStream;
use futures::{Future, Stream, TryFuture, TryStream};
use std::hash::Hash;

///
/// Extensions to the [`TryStream`](futures::TryStream) type which aren't already covered by the
/// included [`TryStreamExt`](futures::TryStreamExt).
///
/// This is implemented using a blanket impl for all `TryStream` implementors (which means any
/// [`Stream`](futures::Stream) that emits a `Result`).
///
pub trait JTryStreamExt: TryStream + Sized {
    ///
    /// Turn this [`TryStream`](futures::TryStream) into a [`TryFuture`](futures::TryFuture) which
    /// gives the first item emitted by this stream (in the form of an `Option`, because the stream
    /// doesn't necessarily have to emit anything).
    ///
    fn try_first(self) -> TryStreamNth<Self> {
        TryStreamNth::first(self)
    }

    ///
    /// Turn this [`TryStream`](futures::TryStream) into a [`TryFuture`](futures::TryFuture) which
    /// gives the nth item emitted by this stream (in the form of an `Option`, because the stream
    /// doesn't have to emit enough items to reach the index `n`).
    ///
    /// It will only emit exactly the `n`'th item. If the stream completes before the `n`th item
    /// is reached, then the future will emit a value of `None`.
    ///
    /// Any errors encountered while reaching `n`th item will be immediately returned.
    ///
    /// The future emits a value of type `Result<Option<Self::Ok>, Self::Error>`
    ///
    fn try_nth(self, n: usize) -> TryStreamNth<Self> {
        TryStreamNth::new(self, n)
    }

    ///
    /// filter+map on the `Self::Ok` value of this stream.
    ///
    /// This stream, with the item type `Result<Self::Ok, Self::Error>`, can be converted
    /// to a stream which skips some values of `Self::Ok` and otherwise transforms non-skipped
    /// values to a new type `T`, giving a new stream with item type `Result<T, Self::Error>`.
    ///
    /// If the current stream emits an `Ok(Self::Ok)` value, then the function passed to this
    /// method will be called to transform it to an `Ok(T)` message.
    ///
    /// If the current stream emits some error by emitting the message `Err(Self::Error)`, then
    /// this message will be passed straight-through.
    ///
    fn try_filter_map_ok<F, R>(self, predicate: F) -> TryFilterMapOk<Self, F, R>
    where
        F: FnMut(Self::Ok) -> Option<R>,
    {
        TryFilterMapOk::new(self, predicate)
    }

    ///
    /// Given some stream where the `Self::Ok` type is `Hash`, then this method will allow you
    /// to "de-duplicate" that stream.
    ///
    /// This is implemented by storing the hash (a `u64` value) of the `Self::Ok` messages in a
    /// `HashSet<u64>` internally. If an item is emitted, it's hash is computed, and if the hash
    /// has been seen before, then the item is skipped.
    ///
    /// Any error items will not be checked for duplication, and will simply be emitted by the
    /// modified "de-duplicated" stream.
    ///
    fn try_dedup(self) -> TryDedupStream<Self>
    where
        Self::Ok: Hash,
    {
        TryDedupStream::new(self)
    }

    ///
    /// If an `Err(Self::Error)` item is emitted from the stream, then panic on further calls to
    /// this stream's `try_poll_next` method, and also implement
    /// [FusedStream](futures::stream::FusedStream) for this stream (even if the current stream
    /// doesn't actually implement `FusedStream`).
    ///
    fn fuse_on_fail(self) -> FuseOnFail<Self> {
        FuseOnFail::new(self)
    }

    ///
    /// Given some initial value of a type `T`, and some function which accepts `&mut T` and
    /// `Self::Ok` and returns a `Future<Output=Result<(), Self::Error>>`, this stream can be
    /// converted into a `Future<Output=Result<T, Self::Error>>`.
    ///
    /// The purpose of this method is basically "fold, but with mutable references, instead of
    /// moving the value."
    ///
    /// Most implementations will not actually need to return a `Future` in the handler function,
    /// but it is required to be a `Future` to support rare use-cases where async code must be
    /// executed to update the `T`. In the common case, you can just use an `async move` block
    /// to turn your non-async code into a `Future`, or you can use `futures::future::ready` to
    /// construct a ready `Future` returning `Ok(())`.
    ///
    /// If the source stream ever emits an `Err(Self::Error)` item, then that causes this future
    /// to immediately emit that same message. Otherwise, the returned future completes when
    /// the stream completes.
    ///
    /// If the stream emits no items, then the initial value of `T` passed as the first parameter
    /// to this method is emitted as `Ok(T)`.
    ///
    fn try_fold_mut<T, F, Fut>(self, initial: T, handler: F) -> TryFoldMut<Self, T, F, Fut>
    where
        Self: FusedStream,
        F: FnMut(&mut T, Self::Ok) -> Fut,
        Fut: TryFuture<Ok = (), Error = Self::Error>,
    {
        TryFoldMut::new(self, initial, handler)
    }
}

impl<T> JTryStreamExt for T where T: TryStream + Sized {}

///
/// Extensions to the [`Stream`](futures::Stream) type which aren't already covered in
/// [`StreamExt`](futures::StreamExt).
///
/// This is implemented using a blanket impl for all `Stream` implementors.
///
pub trait JStreamExt: Stream + Sized {
    ///
    /// Given some stream where the item is `Hash`, return a stream which only emits the unique
    /// items emitted by the source stream.
    ///
    /// You can think of this as "de-duplicating" this stream. Only the first instance of every
    /// unique item will be emitted.
    ///
    /// This is implemented by computing and storing the hash (a `u64` value) in a `HashSet` for
    /// each item emitted by the stream.
    ///
    fn dedup(self) -> DedupStream<Self>
    where
        Self::Item: Hash,
    {
        DedupStream::new(self)
    }

    ///
    /// fold, but with mutable references.
    ///
    /// Turns this stream into a `Future<Output=T>`. You must provide some initial value of type `T`,
    /// and some function which accepts `&mut T` and `Self::Item` and returns `Future<Output=()>`.
    ///
    /// After all items are emitted by this stream, the current value of `T` is emitted by the returned
    /// future.
    ///
    /// If the stream emits no values, then the initial value of `T` is emitted.
    ///
    fn fold_mut<T, F, Fut>(self, initial: T, handler: F) -> FoldMut<Self, T, F, Fut>
    where
        Self: FusedStream,
        F: FnMut(&mut T, Self::Item) -> Fut,
        Fut: Future<Output = ()>,
    {
        FoldMut::new(self, initial, handler)
    }

    ///
    /// Turn this [`Stream`](futures::Stream) into a [`Future`](futures::Future) which gives the
    /// first item emitted by this stream (in the form of an `Option`, because the stream doesn't
    /// necessarily have to emit anything).
    ///
    fn first(self) -> StreamNth<Self> {
        StreamNth::first(self)
    }

    ///
    /// Turn this [`Stream`](futures::Stream) into a [`Future`](futures::Future) which gives the
    /// nth item emitted by this stream (in the form of an `Option`, because the stream doesn't
    /// have to emit enough items to reach the index `n`).
    ///
    /// It will only emit exactly the `n`'th item. If the stream completes before the `n`th item
    /// is reached, then the future will emit a value of `None`.
    ///
    fn nth(self, index: usize) -> StreamNth<Self> {
        StreamNth::new(self, index)
    }
}

impl<T> JStreamExt for T where T: Stream + Sized {}
