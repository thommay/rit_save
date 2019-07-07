use crate::BoxResult;
use clap::{App, ArgMatches, SubCommand};

pub fn cli() -> App<'static, 'static> {
    SubCommand::with_name("diff")
}

pub fn exec(matches: &ArgMatches) -> BoxResult<()> {
    Ok(())
}
