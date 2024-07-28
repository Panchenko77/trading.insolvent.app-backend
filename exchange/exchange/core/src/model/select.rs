use std::fmt::Debug;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::Poll;

use async_trait::async_trait;
use eyre::{ContextCompat, Result};
use futures::future::{poll_fn, LocalBoxFuture};
use futures::{FutureExt, Stream, StreamExt};

use crate::model::{BoxedServiceAsync, PlainData, ServiceAsync};

struct WithIndex<T> {
    index: usize,
    value: T,
}

struct ServiceStream<Service: ServiceAsync + ?Sized> {
    service: NonNull<Service>,
    next: Option<LocalBoxFuture<'static, Option<Result<Service::Response>>>>,
    index: usize,
}

impl<Service: ServiceAsync + ?Sized + 'static> Stream for ServiceStream<Service> {
    type Item = WithIndex<Result<Service::Response>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut service = self.service;
        let task = self.next.get_or_insert_with(|| {
            async move {
                let service = unsafe { service.as_mut() };
                service.next().await
            }
            .boxed_local()
        });
        let result = task.poll_unpin(cx);
        self.next = None;
        match result {
            Poll::Ready(Some(response)) => {
                let index = self.index;
                Poll::Ready(Some(WithIndex { index, value: response }))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
impl<Service: ServiceAsync + ?Sized> Drop for ServiceStream<Service> {
    fn drop(&mut self) {
        self.next = None;
        unsafe {
            let _ = Box::from_raw(self.service.as_ptr());
        }
    }
}
pub struct SelectService<Request: PlainData, Response: PlainData> {
    streams: Vec<ServiceStream<dyn ServiceAsync<Request = Request, Response = Response>>>,
}

impl<Request: PlainData, Response: PlainData> Debug for SelectService<Request, Response> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectService2")
            .field("streams", &self.streams.len())
            .finish()
    }
}

impl<Request: PlainData, Response: PlainData> SelectService<Request, Response> {
    pub fn new() -> Self {
        Self { streams: Vec::new() }
    }

    pub fn push(&mut self, service: BoxedServiceAsync<Request, Response>) {
        let index = self.streams.len();
        let service = ServiceStream {
            service: NonNull::new(Box::leak(service) as *mut _).unwrap(),
            next: None,
            index,
        };

        self.streams.push(service);
    }
    fn poll_next_with_index(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Option<WithIndex<Result<Response>>>> {
        let mut polled = false;
        let mut dead = vec![];
        for (i, stream) in self.streams.iter_mut().enumerate() {
            polled = true;

            match stream.poll_next_unpin(cx) {
                Poll::Ready(Some(response)) => {
                    return Poll::Ready(Some(response));
                }
                Poll::Ready(None) => {
                    dead.push(i);
                }
                Poll::Pending => {}
            }
        }
        for i in dead.into_iter().rev() {
            self.streams.remove(i);
        }
        if polled {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }
    async fn next_with_index(&mut self) -> Option<(Result<Response>, usize)> {
        poll_fn(|cx| self.poll_next_with_index(cx))
            .await
            .map(|item| (item.value, item.index))
    }
    pub async fn request_or_else(&mut self, request: &Request, or_else: impl FnOnce() -> Result<()>) -> Result<()> {
        let stream = self
            .streams
            .iter_mut()
            .find(|stream| unsafe { stream.service.as_ref().accept(request) });
        if let Some(stream) = stream {
            unsafe {
                stream.next = None;
                stream.service.as_mut().request(request).await
            }
        } else {
            or_else()
        }
    }
}

#[async_trait(? Send)]
impl<Request: PlainData, Response: PlainData> ServiceAsync for SelectService<Request, Response> {
    type Request = Request;
    type Response = Response;

    fn accept(&self, request: &Self::Request) -> bool {
        self.streams
            .iter()
            .any(|stream| unsafe { stream.service.as_ref().accept(request) })
    }

    async fn request(&mut self, request: &Self::Request) -> Result<()> {
        let stream = self
            .streams
            .iter_mut()
            .find(|stream| unsafe { stream.service.as_ref().accept(request) })
            .with_context(|| format!("No acceptor for the request: {:?}", request))?;
        unsafe {
            stream.next = None;
            stream.service.as_mut().request(request).await
        }
    }

    async fn next(&mut self) -> Option<Result<Self::Response>> {
        self.next_with_index().await.map(|(response, _index)| response)
    }
}
