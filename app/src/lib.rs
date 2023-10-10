use std::net::SocketAddr;

use client::Client;
use config::Config;
use serde_json::Value;
use tiny_jrpc::{tiny_http, JsonRpcServer, Request, Response};

pub mod client;
pub mod config;
pub mod consts;
pub mod error;

pub struct App {
    _rpc: JsonRpcServer,
    config: Config,
}

pub type Result<T> = std::result::Result<T, error::Error>;

impl App {
    pub fn new(config: Config) -> Result<App> {
        tracing::info!("Creating new app with config: {:?}", config);

        let server = tiny_http::Server::http(config.addr)?;
        let _rpc = tiny_jrpc::JsonRpcServer::new(server, process);

        Ok(App { _rpc, config })
    }

    pub fn addr(&self) -> SocketAddr {
        self.config.addr
    }

    pub fn client(&self) -> Result<Client> {
        Client::new(self.config.addr)
    }
}

fn process(request: Request) -> tiny_jrpc::Result<Response> {
    let response = match request.method.as_str() {
        "echo" => {
            let value = match request.params {
                Some(value) => value,
                None => Value::Null,
            };
            Response::result(request.id, value)
        }
        "version" => Response::result(request.id, consts::APP_VERSION.into()),
        _ => Response::unimplemented(request.id),
    };
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    fn app_random_port() -> App {
        let addr = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let config = Config { addr };
        App::new(config).unwrap()
    }

    #[test]
    fn version() {
        let app = app_random_port();
        let addr = app.addr();
        let url = addr.to_string();
        dbg!(&url);

        let client = jsonrpc::Client::simple_http(&url, None, None).unwrap();
        let request = client.build_request("version", &[]);
        let response = client.send_request(request).unwrap();

        let result = response.result.unwrap().to_string();
        let actual: &str = serde_json::from_str(&result).unwrap();
        assert_eq!(actual, consts::APP_VERSION);
    }
}
