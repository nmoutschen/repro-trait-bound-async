use hyper::{server::conn::AddrStream, Server, Request, Body, Response};
use std::sync::Arc;
use std::net::SocketAddr;
use tower::{service_fn, make::MakeService, Service};
use tokio::sync::Mutex;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct MyRequest;
pub struct MyResponse;

/// Wrapper function that transform hyper::Request into MyRequest, and hyper::Response into MyResponse
pub async fn wrapper<S>(service: Arc<Mutex<S>>, req: Request<Body>) -> Result<Response<Body>, Error>
where
    S: Service<MyRequest, Response = MyResponse>,
{
    let _body = hyper::body::to_bytes(req.into_body()).await?;
    let my_request = MyRequest;

    {
        let mut service = service.lock().await;
        let _my_response = service.call(my_request).await;
    }

    Ok(hyper::Response::new(hyper::Body::empty()))
}

pub async fn run<'a, M>(mut make_service: M) -> Result<(), Error>
where
    M: MakeService<(), MyRequest, Response = MyResponse> + Send + Sync + 'static,
    M::Service: Service<MyRequest, Response = MyResponse> + Send + Sync,
    <M::Service as Service<MyRequest>>::Future: Send + 'a,
    M::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    M::MakeError: Into<Box<dyn std::error::Error + Send + Sync>>,
    M::Future: Send,
{
    let make_service = service_fn(move |_: &AddrStream| {
        let service = make_service.make_service(());

        async move {
            // let service = Arc::new(Mutex::new(service.await?));
            let service = Arc::new(Mutex::new(service));
            Ok::<_, M::MakeError>(service_fn(move |req| wrapper(service.clone(), req)))
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let server = Server::bind(&addr).serve(make_service);

    tokio::spawn(async move {
        server.await
    });

    Ok(())
}