use std::path::PathBuf;

use clap::{Parser, Subcommand};
pub mod cmd;
pub mod object;
pub mod repo;

#[derive(Parser)]
#[command(name = "delta", about = "A version control system, written in rust.")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Add,
    CatFile,
    CheckIgnore,
    Checkout,
    Commit,
    HashObject,
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Log,
    LsFiles,
    LsTree,
    RevParse,
    Rm,
    ShowRef,
    Status,
    Tag,
}

pub fn main() {
    let args = Args::parse();

    match args.command {
        Command::Add => cmd::add(),
        Command::CatFile => cmd::cat_file(),
        Command::CheckIgnore => cmd::check_ignore(),
        Command::Checkout => cmd::checkout(),
        Command::Commit => cmd::commit(),
        Command::HashObject => cmd::hash_object(),
        Command::Init { path } => cmd::init(path),
        Command::Log => cmd::log(),
        Command::LsFiles => cmd::ls_files(),
        Command::LsTree => cmd::ls_tree(),
        Command::RevParse => cmd::rev_parse(),
        Command::Rm => cmd::rm(),
        Command::ShowRef => cmd::show_ref(),
        Command::Status => cmd::status(),
        Command::Tag => cmd::tag(),
    }
}
