use anyhow::Context;
use log::LevelFilter;
use log4rs::{
    append::{console::ConsoleAppender, file::FileAppender},
    config::{Appender, Config, Logger, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};
use std::fs;
use std::path;

const LOG_FILE_NAME: &str = "rosario.log";
const LOG_DIR: &str = "logs/";
const LOG_FILE_LEVEL: LevelFilter = LevelFilter::Debug;
const LOG_CONSOLE_LEVEL: LevelFilter = LevelFilter::Info;

pub(crate) fn init() -> anyhow::Result<()> {
    fs::create_dir_all(LOG_DIR)
        .with_context(|| format!("failed to create dir, dir= {:?}", LOG_DIR))?;
    let file_path = path::Path::new(LOG_DIR).join(LOG_FILE_NAME);
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} {l} {t} {f} {L}- {m}{n}")))
        .build(file_path)?;

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} {l} {t} {f} {L}- {m}{n}")))
        .build();

    let rosario_logger = Logger::builder()
        .appender("logfile")
        .appender("stdout")
        .additive(false)
        .build("rosario", LevelFilter::Trace);

    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LOG_FILE_LEVEL)))
                .build("logfile", Box::new(logfile)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LOG_CONSOLE_LEVEL)))
                .build("stdout", Box::new(stdout)),
        )
        .logger(rosario_logger)
        .build(Root::builder().build(LevelFilter::Trace))?;

    let _handle = log4rs::init_config(config)?;
    Ok(())
}
