use async_trait::async_trait;
use pingora::{
    http::ResponseHeader,
    proxy::{ProxyHttp, Session},
    upstreams::peer::HttpPeer,
    Error, ErrorType, Result,
};
use rack_hooks::HookRegistry;
use tokio::time::{sleep, Duration, Instant};

use crate::{
    hooks,
    services::{self, Destination, ServiceRoutes},
};

#[derive(Clone)]
pub(super) struct RackProxy {
    routes: ServiceRoutes,
    hooks: HookRegistry,
    proxy_port: u16,
}

impl RackProxy {
    pub(super) fn new(routes: ServiceRoutes, hooks: HookRegistry, proxy_port: u16) -> Self {
        Self {
            routes,
            hooks,
            proxy_port,
        }
    }
}

#[derive(Default)]
pub(super) struct RequestCtx {
    destination: Option<Destination>,
}

#[async_trait]
impl ProxyHttp for RackProxy {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        RequestCtx::default()
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let Some(host) = host_header(session) else {
            return respond(session, 400, "missing Host header").await;
        };

        if hooks::is_hooks_host(&host) {
            return hooks::dispatch(session, &self.hooks, &host).await;
        }

        let Some(origin) = services::origin_from_host(&host) else {
            return respond(session, 404, &format!("unsupported Host header `{host}`")).await;
        };

        let Some(destination) = wait_for_destination(&self.routes, &origin).await else {
            return respond(session, 502, "service destination is not running").await;
        };

        if destination.port() == self.proxy_port {
            return respond(
                session,
                508,
                "proxy loop detected: service destination points back to rack proxy",
            )
            .await;
        }

        ctx.destination = Some(destination);
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let Some(destination) = &ctx.destination else {
            return Error::e_explain(ErrorType::InternalError, "missing proxy destination");
        };

        Ok(Box::new(HttpPeer::new(
            ("127.0.0.1", destination.port()),
            false,
            "localhost".to_string(),
        )))
    }
}

const ROUTE_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const ROUTE_WAIT_INTERVAL: Duration = Duration::from_millis(50);

async fn wait_for_destination(routes: &ServiceRoutes, origin: &str) -> Option<Destination> {
    if let Some(destination) = routes.destination_for(origin) {
        return Some(destination);
    }

    let deadline = Instant::now() + ROUTE_WAIT_TIMEOUT;
    while Instant::now() < deadline {
        sleep(ROUTE_WAIT_INTERVAL).await;
        if let Some(destination) = routes.destination_for(origin) {
            return Some(destination);
        }
    }

    None
}

fn host_header(session: &Session) -> Option<String> {
    session
        .req_header()
        .headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .map(str::to_string)
}

async fn respond(session: &mut Session, status: u16, message: &str) -> Result<bool> {
    let body = format!("{message}\n");
    let mut response = ResponseHeader::build(status, None)?;
    response.insert_header("content-length", body.len().to_string())?;
    response.insert_header("content-type", "text/plain; charset=utf-8")?;
    response.insert_header("connection", "close")?;
    session
        .write_response_header(Box::new(response), false)
        .await?;
    session.write_response_body(Some(body.into()), true).await?;
    Ok(true)
}
