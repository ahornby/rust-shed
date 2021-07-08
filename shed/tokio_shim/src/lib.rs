/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use futures::{future::Future, ready, stream::Stream};
use pin_project::pin_project;
use std::any::Any;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use thiserror::Error;

pub mod task {
    use super::*;

    #[derive(Debug, Error)]
    pub enum JoinHandleError {
        #[error("Tokio 0.2 JoinError")]
        Tokio02(#[from] tokio_02::task::JoinError),
        #[error("Tokio 1.0 JoinError")]
        Tokio10(#[from] tokio_10::task::JoinError),
    }

    impl JoinHandleError {
        // For now just implement the required apis


        // See https://docs.rs/tokio/1/tokio/task/struct.JoinError.html#method.into_panic
        pub fn into_panic(self) -> Box<dyn Any + Send + 'static> {
            match self {
                JoinHandleError::Tokio02(inner) => inner.into_panic(),
                JoinHandleError::Tokio10(inner) => inner.into_panic(),
            }
        }
    }

    #[pin_project(project = JoinHandleProj)]
    pub enum JoinHandle<T> {
        Tokio02(#[pin] tokio_02::task::JoinHandle<T>),
        Tokio10(#[pin] tokio_10::task::JoinHandle<T>),
    }

    impl<T> Future for JoinHandle<T>
    where
        T: Send + 'static,
    {
        type Output = Result<T, JoinHandleError>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let ret = match self.project() {
                JoinHandleProj::Tokio02(f) => ready!(f.poll(cx)).map_err(JoinHandleError::from),
                JoinHandleProj::Tokio10(f) => ready!(f.poll(cx)).map_err(JoinHandleError::from),
            };

            Poll::Ready(ret)
        }
    }

    pub fn spawn<F>(fut: F) -> JoinHandle<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        if let Ok(handle) = tokio_02::runtime::Handle::try_current() {
            return JoinHandle::Tokio02(handle.spawn(fut));
        }

        if let Ok(handle) = tokio_10::runtime::Handle::try_current() {
            return JoinHandle::Tokio10(handle.spawn(fut));
        }

        // This is what tokio::spawn would give you, so we don't try to do better here.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }

    pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        if let Ok(handle) = tokio_02::runtime::Handle::try_current() {
            return JoinHandle::Tokio02(handle.spawn_blocking(f));
        }

        if let Ok(handle) = tokio_10::runtime::Handle::try_current() {
            return JoinHandle::Tokio10(handle.spawn_blocking(f));
        }

        // This is what tokio::spawn_blocking would give you, so we don't try to do better here.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }
}

pub mod time {
    use super::*;

    #[pin_project(project = SleepProj)]
    pub enum Sleep {
        Tokio02(#[pin] tokio_02::time::Delay),
        Tokio10(#[pin] tokio_10::time::Sleep),
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.project() {
                SleepProj::Tokio02(f) => f.poll(cx),
                SleepProj::Tokio10(f) => f.poll(cx),
            }
        }
    }

    pub fn sleep(duration: Duration) -> Sleep {
        if tokio_02::runtime::Handle::try_current().is_ok() {
            return Sleep::Tokio02(tokio_02::time::delay_for(duration));
        }

        if tokio_10::runtime::Handle::try_current().is_ok() {
            return Sleep::Tokio10(tokio_10::time::sleep(duration));
        }

        // This is what tokio::time::sleep would do.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }

    pub fn sleep_until(instant: Instant) -> Sleep {
        if tokio_02::runtime::Handle::try_current().is_ok() {
            return Sleep::Tokio02(tokio_02::time::delay_until(
                tokio_02::time::Instant::from_std(instant),
            ));
        }

        if tokio_10::runtime::Handle::try_current().is_ok() {
            return Sleep::Tokio10(tokio_10::time::sleep_until(
                tokio_10::time::Instant::from_std(instant),
            ));
        }

        // This is what tokio::time::sleep would do.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }

    #[derive(Debug, Error)]
    #[error("deadline has elapsed")]
    pub struct Elapsed;

    #[pin_project(project = TimeoutProj)]
    pub enum Timeout<F> {
        Tokio02(#[pin] tokio_02::time::Timeout<F>),
        Tokio10(#[pin] tokio_10::time::Timeout<F>),
    }

    impl<F> Future for Timeout<F>
    where
        F: Future,
    {
        type Output = Result<<F as Future>::Output, Elapsed>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let res = match self.project() {
                TimeoutProj::Tokio02(f) => {
                    ready!(f.poll(cx)).map_err(|_: tokio_02::time::Elapsed| Elapsed)
                }
                TimeoutProj::Tokio10(f) => {
                    ready!(f.poll(cx)).map_err(|_: tokio_10::time::error::Elapsed| Elapsed)
                }
            };

            Poll::Ready(res)
        }
    }

    pub fn timeout<F: Future>(duration: Duration, fut: F) -> Timeout<F> {
        if tokio_02::runtime::Handle::try_current().is_ok() {
            return Timeout::Tokio02(tokio_02::time::timeout(duration, fut));
        }

        if tokio_10::runtime::Handle::try_current().is_ok() {
            return Timeout::Tokio10(tokio_10::time::timeout(duration, fut));
        }

        // This is what tokio::time::timeout would do.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }

    #[pin_project(project = IntervalStreamProj)]
    pub enum IntervalStream {
        Tokio02(#[pin] tokio_02::time::Interval),
        Tokio10(#[pin] tokio_10_stream::wrappers::IntervalStream),
    }

    impl Stream for IntervalStream {
        type Item = Instant;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let ret = match self.project() {
                IntervalStreamProj::Tokio02(f) => ready!(f.poll_next(cx)).map(|i| i.into_std()),
                IntervalStreamProj::Tokio10(f) => ready!(f.poll_next(cx)).map(|i| i.into_std()),
            };

            Poll::Ready(ret)
        }
    }

    pub fn interval_stream(period: Duration) -> IntervalStream {
        if tokio_02::runtime::Handle::try_current().is_ok() {
            return IntervalStream::Tokio02(tokio_02::time::interval(period));
        }

        if tokio_10::runtime::Handle::try_current().is_ok() {
            let interval = tokio_10::time::interval(period);
            let stream = tokio_10_stream::wrappers::IntervalStream::new(interval);
            return IntervalStream::Tokio10(stream);
        }

        // This is what tokio::time::interval_at would do.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }
}

pub mod runtime {
    use super::*;

    use task::JoinHandle;

    #[derive(Debug, Clone)]
    pub enum Handle {
        Tokio02(tokio_02::runtime::Handle),
        Tokio10(tokio_10::runtime::Handle),
    }

    impl Handle {
        pub fn current() -> Self {
            if let Ok(hdl) = tokio_02::runtime::Handle::try_current() {
                return Self::Tokio02(hdl);
            }

            if let Ok(hdl) = tokio_10::runtime::Handle::try_current() {
                return Self::Tokio10(hdl);
            }

            panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
        }

        pub fn spawn<F>(&self, fut: F) -> JoinHandle<<F as Future>::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
        {
            match self {
                Self::Tokio02(hdl) => JoinHandle::Tokio02(hdl.spawn(fut)),
                Self::Tokio10(hdl) => JoinHandle::Tokio10(hdl.spawn(fut)),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use futures::{future, stream::StreamExt};

    async fn test() {
        task::spawn(future::ready(())).await.unwrap();
        task::spawn_blocking(|| ()).await.unwrap();

        time::sleep(Duration::from_millis(1)).await;
        time::sleep_until(Instant::now() + Duration::from_millis(1)).await;

        time::interval_stream(Duration::from_millis(1)).next().await;

        assert!(
            time::timeout(Duration::from_millis(1), future::pending::<()>())
                .await
                .is_err()
        );

        runtime::Handle::current()
            .spawn(future::ready(()))
            .await
            .unwrap();
    }

    #[test]
    fn test_02() -> Result<(), anyhow::Error> {
        let mut rt = tokio_02::runtime::Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()?;

        rt.block_on(test());

        Ok(())
    }

    #[test]
    fn test_10() -> Result<(), anyhow::Error> {
        let rt = tokio_10::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(test());

        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_panic_forwarding_02() {
        let mut rt = tokio_02::runtime::Builder::new()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let je = task::spawn(async {
                panic!("gus");
            })
            .await
            .unwrap_err();
            std::panic::resume_unwind(je.into_panic())
        });
    }

    #[test]
    #[should_panic]
    fn test_panic_forwarding_10() {
        let rt = tokio_10::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let je = task::spawn(async {
                panic!("gus");
            })
            .await
            .unwrap_err();
            std::panic::resume_unwind(je.into_panic())
        });
    }
}
