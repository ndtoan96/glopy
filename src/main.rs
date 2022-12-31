use std::{path::PathBuf, process::exit, sync::Arc};

use structopt::StructOpt;
use walkdir::WalkDir;

const ERROR_VALIDATION: i32 = 1;
const ERROR_GLOB: i32 = 2;
const ERROR_FILE_OPERATION: i32 = 3;

mod utils;

#[derive(Debug, StructOpt)]
#[structopt(name = "glopy", about = "Copy files using glob pattern")]
pub struct Opt {
    #[structopt(short, long, help = "excluded patterns")]
    excludes: Vec<String>,

    #[structopt(
        short,
        long,
        default_value = "4",
        help = "number of threads to run, default is 4"
    )]
    num_threads: usize,

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

    if let Err(e) = utils::validate(&opt) {
        eprintln!("{}", e);
        exit(ERROR_VALIDATION);
    }

    // Build includes globset
    let includes = match utils::build_globset(&opt.patterns, opt.ignore_case) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{}", e);
            exit(ERROR_GLOB);
        }
    };

    // Build excludes globset
    let excludes = match utils::build_globset(&opt.excludes, opt.ignore_case) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{}", e);
            exit(ERROR_GLOB);
        }
    };

    let pool = threadpool::Builder::new().num_threads(opt.num_threads).build();
    let includes_ref = Arc::new(includes);
    let excludes_ref = Arc::new(excludes);

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
        let path = file.into_path();
        let dest_path = opt.dest.join(path.file_name().unwrap());
        let overwrite = !opt.no_overwrite;
        let includes_sync_ref = Arc::clone(&includes_ref);
        let excludes_sync_ref = Arc::clone(&excludes_ref);

        pool.execute(move || {
            match utils::match_copy(
                &path,
                &dest_path,
                includes_sync_ref.as_ref(),
                excludes_sync_ref.as_ref(),
                overwrite,
            ) {
                Ok(true) => {
                    println!("Copy {} -> {}", path.display(), dest_path.display());
                }
                Err(e) => {
                    eprintln!("{}", e);
                    exit(ERROR_FILE_OPERATION);
                }
                _ => (),
            }
        });
    }
    pool.join();
}
