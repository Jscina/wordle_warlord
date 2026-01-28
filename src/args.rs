use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    /// Your guessed word
    pub guess: String,

    /// Pattern like GXXYX
    pub pattern: String,
}
