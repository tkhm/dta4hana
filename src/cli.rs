//! Definition of CLI commands/sub commands + its option parameters
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "DTA4HANA", about = "Delete them all for HANA")]
pub struct CommandLineArgs {
    #[structopt(subcommand)]
    pub action: Action,

    /// Use a different journal file.
    #[structopt(parse(from_os_str), short, long)]
    pub config_file: Option<PathBuf>,
}

#[derive(Debug, StructOpt)]
pub enum Action {
    #[structopt(about = "Delete your tweets")]
    Delete {
        #[structopt(
            short,
            long,
            help = "The most earliest date for the action e.g. 2022-01-01"
        )]
        since: Option<String>,

        #[structopt(
            short,
            long,
            help = "The most latest date for the action e.g. 2022-12-31"
        )]
        until: Option<String>,
    },
    #[structopt(
        about = "Fetch your tweets, this is for the test purpose(pull the tweets and save it in your local)"
    )]
    Fetch {
        #[structopt(
            short,
            long,
            help = "The most earliest date for the action e.g. 2022-01-01"
        )]
        since: Option<String>,

        #[structopt(
            short,
            long,
            help = "The most latest date for the action e.g. 2022-12-31"
        )]
        until: Option<String>,
    },
    #[structopt(about = "Login and overwrite existing credential")]
    Login,
    #[structopt(about = "Unlike your liked tweets")]
    Unlike {
        #[structopt(
            short,
            long,
            help = "The most earliest date for the action e.g. 2022-01-01"
        )]
        since: Option<String>,

        #[structopt(
            short,
            long,
            help = "The most latest date for the action e.g. 2022-12-31"
        )]
        until: Option<String>,
    },
}
