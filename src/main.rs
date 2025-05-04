mod cli;
mod core;
mod domain;
mod infra;

use cli::commands::run;

fn main() -> anyhow::Result<()> {
    run()
}
