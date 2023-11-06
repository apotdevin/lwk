use std::{fs::File, sync::Once};

use anyhow::{anyhow, Context};
use app::config::Config;
use clap::Parser;
use serde_json::Value;
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};

use crate::args::{CliCommand, Network};

mod args;

static TRACING_INIT: Once = Once::new();

fn main() -> anyhow::Result<()> {
    let args = args::Cli::parse();
    // config
    // - network
    // - config file
    // - json rpc host/port
    // - electrum server
    // - file/directory path

    let value = inner_main(args)?;
    println!("{value:#}");
    Ok(())
}

fn inner_main(args: args::Cli) -> anyhow::Result<Value> {
    init_logging(&args);

    // start the app with default host/port
    let config = match args.network {
        Network::Mainnet => Config::default_mainnet(),
        Network::Testnet => Config::default_testnet(),
        Network::Regtest => Config::default_regtest(
            &args
                .electrum_url
                .ok_or_else(|| anyhow!("on regtest you have to specify --electrum-url"))?,
        ),
    };
    let mut app = app::App::new(config)?;
    // get a client to make requests
    let client = app.client()?;

    Ok(match args.command {
        CliCommand::Server => {
            app.run().with_context(|| {
                format!(
                    "Cannot start the server at \"{}\". It is probably already running.",
                    app.addr()
                )
            })?;
            // get the app version
            let version = client.version()?.version;
            tracing::info!("App running version {}", version);

            app.join_threads()?;
            Value::Null
        }
        CliCommand::Signer(a) => match a.command {
            args::SignerCommand::Generate => {
                let j = client.generate_signer().with_context(|| {
                    format!(
                        "Cannot connect to \"{}\". Is the server running?",
                        app.addr()
                    )
                })?;
                serde_json::to_value(&j)?
            }
        },
        CliCommand::Wallet(_) => todo!(),
    })
}

fn init_logging(args: &args::Cli) {
    TRACING_INIT.call_once(|| {
        let file = File::options()
            .create(true)
            .append(true)
            .open("debug.log")
            .expect("must have write permission");
        let (appender, _guard) = if args.stderr {
            tracing_appender::non_blocking(std::io::stderr())
        } else {
            tracing_appender::non_blocking(file)
        };
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_writer(appender)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("called once because sync::once");
        tracing::info!("CLI initialized with args: {:?}", args);
    });
}

#[cfg(test)]
mod test {
    use clap::Parser;
    use serde_json::Value;

    use crate::{args::Cli, inner_main};

    fn sh(command: &str) -> Value {
        let cli = Cli::try_parse_from(command.split(' ')).unwrap();
        inner_main(cli).unwrap()
    }

    #[test]
    fn test_commands() {
        std::thread::spawn(|| {
            sh("cli server");
        });

        let result = sh("cli signer generate");
        assert!(result.get("mnemonic").is_some());
    }
}
