//!
//!
//! A [`Server`](crate::server::Server) can be seen as a collection of requests and subscriptions.
//! Calling [`next_request`](crate::server::Server::next_request) returns a `Future` that returns
//! the next incoming request from a client.
//!
//! When a request arrives, can choose to:
//! 
//! - Answer the request immediately.
//! - Turn the request into a subscription.
//! - Ignore this request and process it later. This can only be done for requests that have an ID,
//! and not for notifications.
//!
//! ## About batches
//!
//! If a client sends [a batch](https://www.jsonrpc.org/specification#batch) of requests and/or
//! notification, the `Server` automatically splits each element of the batch. The batch is later
//! properly recomposed when the answer is sent back.
//!
//! ## Example usage
//!
//! TODO: write example
//!

use crate::raw_server::{RawServerRef, RawServerRq};
use crate::types::{self, from_value, to_value, JsonValue};
use fnv::FnvHashMap;
use futures::prelude::*;
use std::{collections::HashMap, fmt, io, marker::PhantomData, pin::Pin};

pub use self::params::{ServerRequestParams, Iter as ServerRequestParamsIter, ParamKey as ServerRequestParamsKey};
pub use self::run::run;
pub use self::wrappers::http;

mod params;
mod run;
mod wrappers;

/// Wraps around a "raw server".
///
/// See the module-level documentation for more information.
pub struct Server<R> {
    /// Internal "raw" server.
    raw: R,
}

impl<R> Server<R> {
    /// Starts a `Server` using the given raw server internally.
    pub fn new(inner: R) -> Self {
        Server {
            raw: inner,
        }
    }
}

impl<R> Server<R> {
    /// Returns a `Future` resolving to the next request that this server generates.
    pub async fn next_request<'a>(&'a mut self) -> Result<ServerRq<'a, <&'a mut R as RawServerRef<'a>>::Request>, ()>
    where
        &'a mut R: RawServerRef<'a>,
    {
        // This piece of code is where we analyze requests.
        loop {
            let request = self.raw.next_request().await?;
            let _ = match request.request() {
                types::Request::Single(rq) => rq,
                types::Request::Batch(requests) => unimplemented!(),
            };

            return Ok(ServerRq {
                inner: request,
                marker: PhantomData,
            })
        }

        panic!()        // TODO: 
    }

    /*/// Returns a request previously returned by `next_request` by its id.
    ///
    /// Note that previous notifications don't have an ID and can't be accessed with this method.
    ///
    /// Returns `None` if the request ID is invalid or if the request has already been answered in
    /// the past.
    pub fn request_by_id<'a>(&'a mut self, id: &types::Id) -> Option<ServerRq<<&'a mut R as RawServerRef<'a>>::Request>> {
        unimplemented!()
    }*/

    /*pub fn subscriptions_by_id(&mut self, id: &String) -> Option<ServerSubscription<R>> {
        unimplemented!()
    }*/
}

impl<R> From<R> for Server<R> {
    fn from(inner: R) -> Self {
        Server::new(inner)
    }
}

/// Request generated by a `Server`.
///
/// > **Note**: Holds a borrow of the `Server`. Therefore, must be dropped before the `Server` can
/// >           be dropped.
pub struct ServerRq<'a, R> {
    inner: R,
    marker: PhantomData<&'a mut ()>,
}

impl<'a, R> ServerRq<'a, R>
    where R: RawServerRq<'a>
{
    fn call(&self) -> &types::Call {
        match self.inner.request() {
            types::Request::Single(s) => s,
            types::Request::Batch(_) => unreachable!(),     // TODO: justification
        }
    }

    /// Returns the id of the request.
    ///
    /// If this request object is dropped, you can retreive it again later by calling
    /// `request_by_id`. This isn't possible for notifications.
    pub fn id(&self) -> Option<&types::Id> {
        match self.call() {
            types::Call::MethodCall(types::MethodCall { id, .. }) => Some(id),
            types::Call::Notification(types::Notification { .. }) => None,
            types::Call::Invalid { id } => Some(id),        // TODO: shouldn't we panic here or something?
        }
    }

    /// Returns the method of this request.
    pub fn method(&self) -> &str {
        match self.call() {
            types::Call::MethodCall(types::MethodCall { method, .. }) => method,
            types::Call::Notification(types::Notification { method, .. }) => method,
            types::Call::Invalid { .. } => unimplemented!()     // TODO:
        }
    }

    /// Returns the parameters of the request, as a `types::Params`.
    pub fn params(&self) -> ServerRequestParams {
        let p = match self.call() {
            types::Call::MethodCall(types::MethodCall { params, .. }) => params,
            types::Call::Notification(types::Notification { params, .. }) => params,
            types::Call::Invalid { .. } => unimplemented!()     // TODO:
        };

        ServerRequestParams::from(p)
    }

    /// Send back a response.
    ///
    /// If this request is part of a batch:
    ///
    /// - If all requests of the batch have been responded to, then the response is actively
    ///   sent out.
    /// - Otherwise, this response is buffered.
    ///
    pub async fn respond(self, response: Result<types::JsonValue, types::Error>) -> Result<(), io::Error> {
        let output = types::Output::from(response, types::Id::Null, types::Version::V2);      // TODO: id
        self.inner.finish(&types::Response::Single(output)).await?;
        Ok(())
    }

    /*/// Sends back a response similar to `respond`, then returns a `ServerSubscription` object
    /// that allows you to push more data on the corresponding connection.
    // TODO: better docs
    pub async fn into_subscription(self, response: JsonValue)
        -> Result<ServerSubscription<'a, R>, io::Error>
    {
        unimplemented!();
        Ok(ServerSubscription {
            server: self.server,
        })
    }*/
}

/*/// Active subscription of a client towards a server.
///
/// > **Note**: Holds a borrow of the `Server`. Therefore, must be dropped before the `Server` can
/// >           be dropped.
pub struct ServerSubscription<'a, R> {
    server: &'a Server<R>,
}

impl<'a, R> ServerSubscription<'a, R>
where
    for<'r> &'r R: RawServerRef<'r>
{
    pub fn id(&self) -> String {
        unimplemented!()
    }

    pub fn is_valid(&self) -> bool {
        true        // TODO:
    }

    /// Pushes a notification.
    pub async fn push(self, message: JsonValue) -> Result<(), io::Error> {
        unimplemented!()
    }
}*/
