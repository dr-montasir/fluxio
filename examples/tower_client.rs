#![deny(warnings)]

use fluxio::client::conn::Builder;
use fluxio::client::connect::HttpConnector;
use fluxio::client::service::Connect;
use fluxio::service::Service;
use fluxio::{Body, Request};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let mut mk_svc = Connect::new(HttpConnector::new(), Builder::new());

    let uri = "http://127.0.0.1:8080".parse::<http::Uri>()?;

    let mut svc = mk_svc.call(uri.clone()).await?;

    let body = Body::empty();

    let req = Request::get(uri).body(body)?;
    let res = svc.call(req).await?;

    println!("RESPONSE={:?}", res);

    Ok(())
}
