pub fn setup_logger(verbosity: u8) -> Result<(), log::SetLoggerError> {
    let env = env_logger::Env::default().filter_or(
        "RICH_PROMPT_LOG_LEVEL",
        match verbosity {
            0 => "error",
            1 => "warn",
            2 => "info",
            _ => "debug",
        },
    );

    Ok(env_logger::Builder::from_env(env)
        .format_timestamp(None)
        .init())
}
