use crate::ops::*;
use futures::stream::FusedStream;
use futures::{Future, Stream, TryFuture, TryStream};
use std::hash::Hash;

pub trait JTryStreamExt: TryStream + Sized {
    fn try_first(self) -> TryStreamFirst<Self> {
        TryStreamFirst::new(self)
    }

    fn try_filter_map_ok<F, R>(self, predicate: F) -> TryFilterMapOk<Self, F, R>
    where
        F: FnMut(Self::Ok) -> Option<R>,
    {
        TryFilterMapOk::new(self, predicate)
    }

    fn try_dedup(self) -> TryDedupStream<Self>
    where
        Self::Ok: Hash,
    {
        TryDedupStream::new(self)
    }

    fn fuse_on_fail(self) -> FuseOnFail<Self> {
        FuseOnFail::new(self)
    }

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

pub trait JStreamExt: Stream + Sized {
    fn dedup(self) -> DedupStream<Self>
    where
        Self::Item: Hash,
    {
        DedupStream::new(self)
    }

    fn fold_mut<T, F, Fut>(self, initial: T, handler: F) -> FoldMut<Self, T, F, Fut>
    where
        Self: FusedStream,
        F: FnMut(&mut T, Self::Item) -> Fut,
        Fut: Future<Output = ()>,
    {
        FoldMut::new(self, initial, handler)
    }
}

impl<T> JStreamExt for T where T: Stream + Sized {}
