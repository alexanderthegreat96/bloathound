use std::process::ExitCode;

mod bloathound;

fn main() -> ExitCode {
    bloathound::cli::run()
}
