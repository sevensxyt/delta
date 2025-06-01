use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
pub mod cmd;
pub mod ignore;
pub mod index;
pub mod kvlm;
pub mod object;
pub mod reference;
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
    #[command(about = "Check path(s) against ignore rules")]
    CheckIgnore {
        #[arg(help = "Path(s) to check")]
        paths: Vec<PathBuf>,
    },
    #[command(about = "Checkout a commit inside of a directory")]
    Checkout {
        #[arg(help = "The commit or tree to checkout")]
        commit: String,

        #[arg(help = "The EMPTY directory to checkout on")]
        path: PathBuf,
    },
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
    #[command(about = "List all staged files")]
    LsFiles {
        #[arg(help = "Show everything", short = 'v', long = "verbose")]
        verbose: bool,
    },
    #[command(about = "Pretty-print a tree object")]
    LsTree {
        #[arg(help = "A tree-ish object")]
        tree: String,

        #[arg(short = 'r', long = "recursive", help = "Recurse into sub-trees")]
        recurse: bool,
    },
    #[command(about = "Parse revision (or other objects) identifiers")]
    RevParse {
        #[arg(help = "Specify the expected format", long = "format", default_value = None,  value_parser = ["blob", "commit", "tag", "tree"])]
        format: Option<String>,

        #[arg(help = "The name to parse")]
        name: String,
    },
    Rm,
    Kill {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    #[command(about = "List references")]
    ShowRef,
    Status,
    #[command(about = "List and create tags")]
    Tag {
        #[arg(help = "The new tag's name")]
        name: Option<String>,

        #[arg(default_value = "HEAD", help = "The object the tag will point to")]
        object: String,

        #[arg(short = 'a', help = "Whether to create the tag object")]
        create_tag_object: bool,
    },
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Add => cmd::add(),
        Command::CatFile { object, format } => cmd::cat_file(object, format)?,
        Command::CheckIgnore { paths } => cmd::check_ignore(paths)?,
        Command::Checkout { commit, path } => cmd::checkout(commit, path)?,
        Command::Commit => cmd::commit(),
        Command::HashObject {
            path,
            format,
            write,
        } => cmd::hash_object(path, format, write)?,
        Command::Init { path } => cmd::init(path)?,
        Command::Log { commit } => cmd::log(commit)?,
        Command::LsFiles { verbose } => cmd::ls_files(verbose)?,
        Command::LsTree { tree, recurse } => cmd::ls_tree(tree, recurse)?,
        Command::RevParse { format, name } => cmd::rev_parse(format, name)?,
        Command::Rm => cmd::rm(),
        Command::Kill { path } => cmd::kill(path)?,
        Command::ShowRef => cmd::show_ref()?,
        Command::Status => cmd::status(),
        Command::Tag {
            name,
            object,
            create_tag_object,
        } => cmd::tag(name, object, create_tag_object)?,
    }

    Ok(())
}
