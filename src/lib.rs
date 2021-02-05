//!
//! # Introduction
//!
//! Extensions to the [`Stream`](futures::Stream) and [`TryStream`](futures::TryStream) traits
//! which implement behavior that I've implemented at least a few times while working with them.
//!
//! To use these extensions, simply `use` the [`JStreamExt`](crate::JStreamExt) or
//! [`JTryStreamExt`](crate::JTryStreamExt) items exported by this crate.
//!
//! # Summary
//!
//! Here's a list of the various extensions provided by this crate:
//!
//! ## `Stream` Extensions
//!
//! The extensions to [`Stream`](futures::Stream) are provided by the [`JStreamExt`](crate::JStreamExt)
//! trait.
//!
//! * [`dedup`](crate::JStreamExt::dedup) - remove duplicate items from a stream
//! * [`fold_mut`](crate::JStreamExt::fold_mut) - Similar to [`fold`](futures::StreamExt::fold), but
//!   asks for a `(&mut T, Self::Item)` -> `Future<Output=()>` instead of a
//!   `(T, Self::Item)` -> `Future<Output=T>` folding function.
//! * [`first`](crate::JStreamExt::first) - turns a stream into a future which emits only the first
//!   item emitted by the source.
//! * [`nth`](crate::JStreamExt::nth) - turns a stream into a future which emits an item after skipping
//!   a specified number of preceding items.
//!
//! ## `TryStream` Extensions
//!
//! The extensions to [`TryStream`](futures::TryStream) are provided by the [`JTryStreamExt`](crate::JTryStreamExt)
//! trait.
//!
//! * [`try_first`](crate::JTryStreamExt::try_first) - turns the stream into a future which emits only
//!   the first result emitted by the source.
//! * [`try_nth`](crate::JTryStreamExt::try_nth) - turns the stream into a future which emits an item
//!   after skipping a specified number of preceding items, or emits an error immediately when encountered.
//! * [`try_filter_map_ok`](crate::JTryStreamExt::try_filter_map_ok) - similar to
//!   [`filter_map`](futures::StreamExt::filter_map), except it allows you to filter-map on the `Ok`
//!   part of the `TryStream`, and it emits any errors immediately when they are encountered.
//! * [`try_dedup`](crate::JTryStreamExt::try_dedup) - remove duplicate items from a stream, but also
//!   emit any errors immediately when they are seen.
//! * [`fuse_on_fail`](crate::JTryStreamExt::fuse_on_fail) - if an error is seen, "fuse" the stream
//!   such that it panics if `try_poll_next` is called after an `Err(Self::Error)` item is emitted.
//!   This also makes a [`TryStream`](futures::TryStream) implement [`FusedStream`](futures::stream::FusedStream)
//!   regardless if the source implements that trait.
//! * [`try_fold_mut`](crate::JTryStreamExt::try_fold_mut) - Similar to
//!   [`try_fold`](futures::TryStreamExt::try_fold), but asks for a
//!   `(&mut T, Self::Ok)` -> `Future<Output=Result<(), Self::Error>>` instead of a
//!   `(T, Self::Ok)` -> `Future<Output=Result<T, Self::Error>>` folding function.
//!

#[macro_use]
extern crate futures;

macro_rules! op_mods {
    {$($nam: ident),*$(,)*} => {
        $(mod $nam;)*

        /// The various structs which wrap various [`Stream`](futures::Stream) and
        /// [`TryStream`](futures::TryStream) upstreams to implement various behavior live in this
        /// module.
        ///
        /// You should not have to directly import this module. Instead, use the extension traits
        /// detailed in the module documentation (see: [`JStreamExt`](crate::JStreamExt) and
        /// [`JTryStreamExt`](crate::JTryStreamExt)).
        pub mod ops {
            $(pub use super::$nam::*;)*
        }
    }
}

#[cfg(feature = "sink")]
macro_rules! delegate_sink {
    ($field:ident, $err: ty, $item: ty) => {
        type Error = $err;

        fn poll_ready(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Result<(), Self::Error>> {
            self.project().$field.poll_ready(cx)
        }

        fn start_send(self: core::pin::Pin<&mut Self>, item: $item) -> Result<(), Self::Error> {
            self.project().$field.start_send(item)
        }

        fn poll_flush(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Result<(), Self::Error>> {
            self.project().$field.poll_flush(cx)
        }

        fn poll_close(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Result<(), Self::Error>> {
            self.project().$field.poll_close(cx)
        }
    };
}

macro_rules! delegate_fused {
    ($nam: ident) => {
        fn is_terminated(&self) -> bool {
            self.$nam.is_terminated()
        }
    };
}

op_mods! {
    fuse_on_fail,
    dedup,
    try_filter_map_ok,
    nth,
    fold_mut,
}

pub(crate) mod op_prelude {
    #[cfg(feature = "sink")]
    pub use futures::sink::Sink;
    pub use futures::stream::FusedStream;
    pub use futures::{Future, Stream, TryFuture, TryStream};
    pub use pin_project_lite::pin_project;
    pub use std::marker::PhantomData;
    pub use std::pin::Pin;
    pub use std::task::{Context, Poll};
}

mod ext;
pub use ext::*;
