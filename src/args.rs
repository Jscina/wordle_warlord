use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Repeated guess entries: WORD PATTERN
    #[arg(long, num_args = 2, value_names = ["WORD", "PATTERN"])]
    pub guess: Vec<String>,

    /// Run in interactive mode
    #[arg(long)]
    pub interactive: bool,
}
