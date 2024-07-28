use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Sink, Stream};

use trading_exchange_core::model::{ExecutionRequest, ExecutionResponse};

use crate::execution::GateioExecutionConnection;

impl Sink<ExecutionRequest> for GateioExecutionConnection {
    type Error = eyre::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: ExecutionRequest) -> Result<(), Self::Error> {
        match item {
            ExecutionRequest::PlaceOrder(req) => self.start_new_order(&req),
            ExecutionRequest::CancelOrder(req) => self.start_cancel_order(&req),
            _ => unimplemented!("unsupported request: {:?}", item),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Stream for GateioExecutionConnection {
    type Item = Result<ExecutionResponse, eyre::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // if let Poll::Ready(msg) = self.ws.poll_next_unpin(cx) {
        //     return Poll::Ready(msg);
        // }

        if let Poll::Ready(_) = self.sync_orders_interval.poll_tick(cx) {
            let manager = self.manager.clone();
            self.session.send_sync_orders(manager);
        }
        if let Poll::Ready(_) = self.sync_balances_interval.poll_tick(cx) {
            let manager = self.manager.clone();
            self.session.send_query_user_assets(manager);
        }
        if let Poll::Ready(msg) = self.session.poll_next(cx) {
            return Poll::Ready(Some(Ok(msg)));
        }

        Poll::Pending
    }
}
