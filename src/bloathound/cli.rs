use clap::{Parser, Subcommand};
use dialoguer::console::Term;
use figlet_rs::Toilet;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::bloathound::{interactive, report};

const APP_NAME: &str = "BLOATHOUND";
const APP_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

#[derive(Parser, Debug)]
#[command(name = "bloathound", version, about = "Fast disk analyzer and cleaner")]
struct Cli {
    #[arg(long)]
    permanent: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Interactive,
    Scan {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
        #[arg(short, long, value_delimiter = ',')]
        ignore: Vec<String>,
    },
}

pub fn run() -> ExitCode {
    print_banner();

    let cli = Cli::parse();
    let permanent = cli.permanent;

    match cli.command.unwrap_or(Commands::Interactive) {
        Commands::Interactive => {
            interactive::run(permanent);
            ExitCode::SUCCESS
        }
        Commands::Scan {
            path,
            limit,
            ignore,
        } => report::run(&path, limit, &ignore),
    }
}

fn print_banner() {
    let future_font = Toilet::future().unwrap();
    println!(
        "{}",
        future_font
            .convert(&format!("{APP_NAME} {APP_VERSION}"))
            .unwrap()
    );

    println!(
        "Disk scan & analysis tool. Will scan for the top largest files to enable you to delete them"
    );
    println!();
}

pub fn prompt<T>(result: dialoguer::Result<T>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            let _ = Term::stderr().show_cursor();
            eprintln!("\nAborted: {err}");
            std::process::exit(130);
        }
    }
}
