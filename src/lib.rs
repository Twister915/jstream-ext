#[macro_use]
extern crate futures;

macro_rules! op_mods {
    {$($nam: ident),*$(,)*} => {
        $(pub mod $nam;)*
        pub(crate) mod ops {
            $(pub use super::$nam::*;)*
        }
        pub use ops::*;
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
    try_first,
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
