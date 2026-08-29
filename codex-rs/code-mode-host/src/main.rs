//! CLI-точка входа Code Mode host.
//!
//! Помимо выбора transport endpoint корневой help публикует Git-ревизию
//! бинарника, которую проверяет общий fork workflow установки.

use clap::CommandFactory;
use clap::FromArgMatches;
use clap::Parser;
use codex_build_info::BuildInfo;

#[derive(Debug, Parser)]
struct Cli {
    /// Transport endpoint: `stdio`, `stdio://`, `ws://IP:PORT`, or `grpc://IP:PORT`.
    #[arg(
        long,
        value_name = "URL",
        default_value = codex_code_mode_host::DEFAULT_LISTEN_URL
    )]
    listen: String,
}

/// Строит CLI с машинно-читаемой строкой ревизии для установщика.
fn cli_command(build_commit: &str) -> clap::Command {
    Cli::command().after_help(format!("Revision: {build_commit}"))
}

/// Разбирает аргументы после чтения встроенной ревизии конечного бинарника.
fn parse_cli() -> Cli {
    let matches = cli_command(BuildInfo::get().build_commit()).get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    codex_build_info::initialize!();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    codex_code_mode_host::run_main(&parse_cli().listen).await
}
