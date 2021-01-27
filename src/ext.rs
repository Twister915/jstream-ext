use crate::{FuseOnFail, TryDedupStream, TryFilterMapOk, TryStreamFirst};
use futures::TryStream;
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
}

impl<T> JTryStreamExt for T where T: TryStream + Sized {}
