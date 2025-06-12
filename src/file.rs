use crate::{issue::IssueTracker, preprocessor, program::Program, rcsubstring::RcSubString};
use std::io::{Read, Write};
use std::process::Stdio;
use std::rc::Rc;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    str,
};
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct File {
    pub full_path: Option<String>,
    pub short_path: Option<String>,
    pub raw: String,
    pub processed: RcSubString,
    pub is_entry: bool,
    pub pp_chunks: Vec<preprocessor::Chunk>,
}

pub enum FindImportRealPathResult {
    Found(String),
    NotFound,
}
pub enum FindImportResult {
    Found(Rc<File>),
    NotFound,
}

impl File {
    pub fn from_filesystem(
        full_path: String,
        short_path: String,
        is_entry: bool,
    ) -> Result<Self, Box<dyn Error>> {
        // Replace tabs with spaces to avoid issues with error reporting and formatting.
        let raw = fs::read_to_string(&full_path)?.replace('\t', "    ");
        let (processed, diffs) = preprocessor::preprocess(&raw);
        Ok(Self {
            full_path: Some(full_path),
            short_path: Some(short_path),
            raw,
            processed: RcSubString::from_str(&processed),
            pp_chunks: diffs,
            is_entry,
        })
    }

    pub fn from_str(source: &str) -> Self {
        let (processed, pp_chunks) = preprocessor::preprocess(source);
        Self {
            full_path: None,
            short_path: None,
            processed: RcSubString::from_str(&processed),
            raw: String::from(source),
            is_entry: true,
            pp_chunks,
        }
    }

    pub fn from_cli_arg(arg: &String) -> Result<Self, Box<dyn Error>> {
        let file_path = arg;
        let parent_path = Path::new(file_path)
            .parent()
            .map_or("", |parent| parent.to_str().unwrap_or(""));
        let short_path = file_path
            .strip_prefix(parent_path)
            .expect("file_path should start with parent_path")
            .into();
        Self::from_filesystem(file_path.to_string(), short_path, true)
    }

    /// Creates a File from a string, preprocesses it, and runs the linter with the default configuration.
    /// This is useful for testing and quick checks.
    pub fn lint_with_default_config(source: &str) -> (Program, IssueTracker) {
        let file = Self::from_str(source);
        let self_rc = Rc::new(file);
        let mut issues = IssueTracker::new();
        // It's safe to unwrap here because the `File` doesn't have a path,
        // so it won't have any issues related to follow imports.
        let program = Program::from_file(&self_rc, &mut issues)
            .expect("from_file shouldn't return Error with a File with no path");
        program.lint(&mut issues);
        (program, issues)
    }

    pub fn find_import(
        fx_path: &str,
        import_path: &str,
    ) -> Result<FindImportResult, Box<dyn Error>> {
        let found_import_path = Self::find_import_real_path(fx_path, import_path);
        let FindImportRealPathResult::Found(full_path) = found_import_path else {
            return Ok(FindImportResult::NotFound);
        };
        Ok(FindImportResult::Found(Rc::new(Self::from_filesystem(
            full_path,
            import_path.to_string(),
            false,
        )?)))
    }

    fn find_import_real_path(fx_path: &str, import_path: &str) -> FindImportRealPathResult {
        fn remove_leftmost_component(path: &str) -> String {
            let mut iter = Path::new(path).iter();
            iter.next();
            iter.collect::<PathBuf>().to_string_lossy().to_string()
        }
        let import_path = Path::new(import_path);
        let parent_dir = Path::new(fx_path).parent().expect("Can't find parent dir");
        // https://askjf.com/index.php?q=7154s
        // If you specify a relative_path, e.g a/b/c.jsfx-inc
        if import_path.is_relative() {
            // it first looks for path_of_calling_code/a/b/c.jsfx-inc, then path_of_calling_code/b/c.jsfx-inc then path_of_calling_code/c.jsfx-inc.
            let mut path = import_path.to_string_lossy().to_string();
            while !path.is_empty() {
                let relative_path: PathBuf = parent_dir.join(Path::new(&path));
                if relative_path.exists() {
                    return FindImportRealPathResult::Found(
                        relative_path.to_string_lossy().to_string(),
                    );
                }
                path = remove_leftmost_component(&path);
            }
        } else if import_path.parent().is_none() {
            // If you specify a filename only, e.g. c.jsfx-inc, it first looks in path_of_calling_code/c.jsfx-inc.
            let relative_path = parent_dir.join(import_path);
            if relative_path.exists() {
                return FindImportRealPathResult::Found(
                    relative_path.to_string_lossy().to_string(),
                );
            }
        }
        if let Some(file_name) = import_path.file_name() {
            // If the above paths are not found, then it does a recursive search of the JSFX Effects directory for the file in question,
            // ignoring any relative path. The recursive search is a bit random in the order,
            // if you have foo/c.jsfx-inc and foo/bar/way/down/deep/c.jsfx-inc, it could return either depending on the filesystem.
            for entry in WalkDir::new(parent_dir).into_iter().filter_map(Result::ok) {
                if entry.file_name() == file_name {
                    return FindImportRealPathResult::Found(
                        entry.into_path().to_string_lossy().to_string(),
                    );
                }
            }
            let jsfx_path = Self::jsfx_effects_path();
            let Some(jsfx_path) = jsfx_path else {
                return FindImportRealPathResult::NotFound;
            };
            if !jsfx_path.exists() {
                return FindImportRealPathResult::NotFound;
            }
            for entry in WalkDir::new(jsfx_path).into_iter().filter_map(Result::ok) {
                if entry.file_name() == file_name {
                    return FindImportRealPathResult::Found(
                        entry.into_path().to_string_lossy().to_string(),
                    );
                }
            }
        }
        FindImportRealPathResult::NotFound
    }

    fn jsfx_effects_path() -> Option<PathBuf> {
        if cfg!(target_os = "windows") {
            Some(PathBuf::from("%APPDATA%\\REAPER\\Effects"))
        } else if cfg!(target_os = "linux") {
            let home = std::env::var("HOME").ok()?;
            Some(PathBuf::from(home).join(".config/REAPER/Effects"))
        } else if cfg!(target_os = "macos") {
            let home = std::env::var("HOME").ok()?;
            Some(PathBuf::from(home).join("Library/Application Support/REAPER/Effects"))
        } else {
            None
        }
    }

    fn eel_pp_path() -> PathBuf {
        let name = if cfg!(target_os = "windows") {
            "eel_pp.exe"
        } else if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            "eel_pp"
        } else {
            panic!("Unsupported target os")
        };
        // For now, the eel_pp binary is required to be in the same directory (or the parent, see below) as the executable
        let binary_path = std::env::current_exe().expect("Couldn't get current exe path");
        let binary_dir = binary_path
            .parent()
            .expect("Couldn't get parent dir")
            .to_str()
            .expect("Couldn't convert path to str");

        let path = Path::new(binary_dir).join(name);
        if path.exists() {
            return path;
        }
        // Check in parent dir (useful for development, as sometimes the binary ends up in the deps/ directory)
        if let Some(path) = Path::new(binary_dir).parent() {
            let path = path.join(name);
            if path.exists() {
                return path;
            }
        }
        panic!("eel_pp not found in the same directory as the executable");
    }

    pub fn preprocess_str(source: &str) -> String {
        let path = Self::eel_pp_path();
        let mut command = std::process::Command::new(path)
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn eel_pp");
        let mut stdin = command.stdin.take().expect("Failed to open stdin");
        let string = source.to_string();
        std::thread::spawn(move || {
            stdin
                .write_all(string.as_bytes())
                .expect("Failed to write to stdin");
        });
        let output = command.wait_with_output().expect("Failed to read stdout");
        String::from(str::from_utf8(&output.stdout).unwrap_or_default())
    }

    pub fn get_printable_path(&self) -> &str {
        self.short_path
            .as_deref()
            .or(self.full_path.as_deref())
            .unwrap_or_default()
    }

    pub fn from_stdin() -> Result<Self, Box<dyn Error>> {
        let mut buf = String::new();
        std::io::BufReader::new(std::io::stdin().lock()).read_to_string(&mut buf)?;
        Ok(Self::from_str(&buf))
    }
}
