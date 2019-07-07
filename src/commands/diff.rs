use clap::{App, SubCommand, ArgMatches};
use crate::BoxResult;

pub fn cli() -> App<'static, 'static> {
    SubCommand::with_name("diff")
}

pub fn exec(matches: &ArgMatches) -> BoxResult<()> {
    Ok(())
}
