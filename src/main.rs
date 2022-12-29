use std::{fs, path::PathBuf, process::exit};

use globset::{GlobBuilder, GlobSetBuilder};
use structopt::StructOpt;
use walkdir::WalkDir;

const ERROR_UNEXISTED: i32 = 1;
const ERROR_GLOB: i32 = 2;
const ERROR_FILE_OPERATION: i32 = 3;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "glopy",
    version = "0.1.0",
    about = "Copy files using glob pattern"
)]
pub struct Opt {
    #[structopt(short, long, help = "excluded patterns")]
    excludes: Vec<String>,

    #[structopt(short, long, help = "patterns are case insensitive")]
    ignore_case: bool,

    #[structopt(short = "p", help = "create destination folder if it does not exist")]
    create_dest: bool,

    #[structopt(
        short,
        long,
        help = "do not overwrite existing files at destination folder"
    )]
    no_overwrite: bool,

    #[structopt(help = "glob patterns to copy")]
    patterns: Vec<String>,

    #[structopt(
        short,
        long,
        default_value = ".",
        help = r#"source folder, default is current folder"#
    )]
    source: PathBuf,

    #[structopt(short, long, parse(from_os_str), help = "destination folder")]
    dest: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    validate_paths(&opt);

    // Build includes globset
    let mut includes_builder = GlobSetBuilder::new();
    opt.patterns
        .iter()
        .for_each(|p| add_pattern(&mut includes_builder, p, opt.ignore_case));
    let includes = match includes_builder.build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error while building patterns: {}", e);
            exit(ERROR_GLOB);
        }
    };

    // Build excludes globset
    let mut excludes_builder = GlobSetBuilder::new();
    opt.excludes
        .iter()
        .for_each(|p| add_pattern(&mut excludes_builder, p, opt.ignore_case));
    let excludes = match excludes_builder.build() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Error while building exclude patterns: {}", e);
            exit(ERROR_GLOB);
        }
    };

    for file in WalkDir::new(opt.source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            // Ignore files in destination folder
            !e.path()
                .canonicalize()
                .unwrap()
                .starts_with(opt.dest.canonicalize().unwrap())
        })
    {
        let path = file.path();
        if !excludes.is_match(path) && includes.is_match(path) {
            let dest_path = opt.dest.join(path.file_name().unwrap());
            if opt.no_overwrite && dest_path.exists() {
                continue;
            }
            if let Err(e) = fs::copy(path, &dest_path) {
                eprintln!(
                    "[Error] Copy {} -> {}: {}",
                    path.display(),
                    dest_path.display(),
                    e
                );
                exit(ERROR_FILE_OPERATION);
            }
            println!("Copy {} -> {}", path.display(), dest_path.display());
        }
    }
}

fn validate_paths(opt: &Opt) {
    // Check if source folder exists
    if !opt.source.exists() {
        eprintln!(
            r#"Source folder "{}" does not exists."#,
            opt.source.display()
        );
        exit(ERROR_UNEXISTED);
    }

    // Check if destination folder exists
    if !opt.dest.exists() {
        if opt.create_dest {
            if let Err(e) = fs::create_dir_all(&opt.dest) {
                eprintln!("Cannot create folder {}: {}", opt.dest.display(), e);
                exit(ERROR_FILE_OPERATION);
            }
        } else {
            eprintln!(
                r#"Destination folder "{}" does not exists."#,
                opt.dest.display()
            );
            exit(ERROR_UNEXISTED);
        }
    }
}

fn add_pattern(builder: &mut GlobSetBuilder, pattern: &str, case_insensitive: bool) {
    match GlobBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
    {
        Err(e) => {
            eprintln!("{} is not a valid glob pattern", e.glob().unwrap());
            exit(ERROR_GLOB);
        }
        Ok(glob) => {
            builder.add(glob);
        }
    }
}
