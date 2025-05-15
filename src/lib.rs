use std::{error::Error, path::PathBuf};

use clap::{Parser, Subcommand};
pub mod cmd;
pub mod kvlm;
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
    CatFile {
        #[arg(name = "type", help = "Specify the type", value_parser = ["blob", "commit", "tag", "tree"])]
        format: String,

        #[arg(name = "object", help = "The object to display")]
        object: String,
    },
    CheckIgnore,
    Checkout,
    Commit,
    HashObject {
        path: PathBuf,

        #[arg(short = 't', long = "type", default_value = "blob", value_parser = ["blob", "commit", "tag", "tree"])]
        format: String,

        #[arg(short = 'w', long = "write")]
        write: bool,
    },
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Log {
        #[arg(default_value = "HEAD")]
        commit: String,
    },
    LsFiles,
    LsTree,
    RevParse,
    Rm,
    Kill {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    ShowRef,
    Status,
    Tag,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match args.command {
        Command::Add => cmd::add(),
        Command::CatFile { object, format } => cmd::cat_file(object, format)?,
        Command::CheckIgnore => cmd::check_ignore(),
        Command::Checkout => cmd::checkout(),
        Command::Commit => cmd::commit(),
        Command::HashObject {
            path,
            format,
            write,
        } => cmd::hash_object(path, format, write)?,
        Command::Init { path } => cmd::init(path)?,
        Command::Log { commit } => cmd::log(commit)?,
        Command::LsFiles => cmd::ls_files(),
        Command::LsTree => cmd::ls_tree(),
        Command::RevParse => cmd::rev_parse(),
        Command::Rm => cmd::rm(),
        Command::Kill { path } => cmd::kill(path)?,
        Command::ShowRef => cmd::show_ref(),
        Command::Status => cmd::status(),
        Command::Tag => cmd::tag(),
    }

    Ok(())
}
