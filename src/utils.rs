use core::time;
use std::path::Path;
use std::thread;

use globset::GlobBuilder;

use std::fs;

use super::Opt;

use globset::GlobSetBuilder;

use globset::GlobSet;

use globset;

pub(crate) fn build_globset(
    patterns: &Vec<String>,
    ignore_case: bool,
) -> Result<globset::GlobSet, String> {
    let mut includes_builder = GlobSetBuilder::new();
    for p in patterns {
        add_pattern(&mut includes_builder, p, ignore_case)?;
    }
    let includes = match includes_builder.build() {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Error while building patterns: {}", e));
        }
    };
    return Ok(includes);
}

pub(crate) fn validate(opt: &Opt) -> Result<(), String> {
    // Check if source folder exists
    if !opt.source.exists() {
        return Err(format!(
            r#"Source folder "{}" does not exists."#,
            opt.source.display()
        ));
    }

    // Check if destination folder exists
    if !opt.dest.exists() {
        if opt.create_dest {
            if let Err(e) = fs::create_dir_all(&opt.dest) {
                return Err(format!("Error creating {}: {}", opt.dest.display(), e));
            }
        } else {
            return Err(format!(
                r#"Destination folder "{}" does not exists."#,
                opt.dest.display()
            ));
        }
    }

    return Ok(());
}

pub(crate) fn add_pattern(
    builder: &mut GlobSetBuilder,
    pattern: &str,
    case_insensitive: bool,
) -> Result<(), String> {
    match GlobBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
    {
        Err(e) => {
            return Err(format!("{} is not a valid glob pattern", e.glob().unwrap()));
        }
        Ok(glob) => {
            builder.add(glob);
            return Ok(());
        }
    }
}

pub(crate) fn match_copy(
    src: &Path,
    dest: &Path,
    includes: &GlobSet,
    excludes: &GlobSet,
    overwrite: bool,
) -> Result<bool, String> {
    if !excludes.is_match(src) && includes.is_match(src) {
        if !overwrite && dest.exists() {
            return Ok(false);
        }

        // Copy src to dest, if error happens, wait 100ms and retry again for maximum 3 times because
        // the dest is likely being accessed by another thread
        let mut err_cnt = 0;
        while let Err(e) = fs::copy(src, dest) {
            err_cnt += 1;
            if err_cnt >= 3 {
                return Err(format!(
                    "[Error] Copy {} -> {}: {}",
                    src.display(),
                    dest.display(),
                    e
                ));
            }
            thread::sleep(time::Duration::from_millis(100));
        }
        return Ok(true);
    } else {
        return Ok(false);
    }
}
