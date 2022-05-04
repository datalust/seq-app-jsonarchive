#[macro_use]
extern crate serde_derive;

use std::result::Result;
use std::env;
use chrono::{DateTime, Utc};
use std::io;
use std::io::prelude::*;
use std::fs::{self, OpenOptions, File};
use std::path::{PathBuf, Path};
use std::error::Error;
use std::fmt;

#[derive(Serialize)]
struct FatalErrorEvent<'a> {
    #[serde(rename="@t")]
    timestamp: DateTime<Utc>,

    #[serde(rename="@l")]
    level: &'static str,

    #[serde(rename="@mt")]
    message_template: &'static str,

    #[serde(rename="Failure")]
    failure: &'a str
}

impl<'a> FatalErrorEvent<'a> {
    pub fn new(failure: &'a str) -> FatalErrorEvent<'a> {
        FatalErrorEvent {
            timestamp: Utc::now(),
            message_template: "Unable to create JSON archive: {Failure}",
            failure,
            level: "Fatal"
        }
    }
}

struct AppError(&'static str);

impl Error for AppError {
    fn description(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Copy,Clone)]
enum FilenameFormat {
    Timestamp,          // log-00000000577f6df3.clef
    Readable,           // log-2016-07-08T09_10_11Z.clef
}

struct FileSet<'a> {
    dir: &'a Path,
    file_name_template: String,
    format: FilenameFormat,
}

impl<'a> FileSet<'a> {
    fn new(file_set: &'a str, format: FilenameFormat) -> Result<FileSet<'a>, Box<dyn Error>> {
        let file_set_path = Path::new(file_set);

        let dir = file_set_path.parent()
            .ok_or(AppError("the file set must specify a filename"))?;

        let file_set_filename = file_set_path.file_name()
            .ok_or(AppError("the file set must specify a filename pattern"))?;

        let file_name_template = file_set_filename.to_os_string().into_string()
            .map_err(|_| AppError("filename character set conversion failed"))?;

        if !file_name_template.contains("*") {
            Err(AppError("the filename pattern must include the `*` wildcard"))?
        };

        Ok(FileSet{dir, file_name_template, format })
    }

    fn make_file_path(&self, timestamp: DateTime<Utc>) -> PathBuf {
        const READABLE_FORMAT: &str = "%Y-%m-%dT%H_%M_%SZ"; // 2016-07-08T09_10_11Z

        let timestamp = match &self.format {
            FilenameFormat::Timestamp => format!("{:01$x}", timestamp.timestamp(), 16),
            FilenameFormat::Readable => format!("{}", timestamp.format(READABLE_FORMAT))
        };
        let full_file_name = self.file_name_template.replace("*", &timestamp);
        let mut buf = self.dir.to_path_buf();
        buf.push(full_file_name);
        buf
    }
    
    fn ensure_dir_exists(&self) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(self.dir)?;
        Ok(())
    }

    fn open_next_file(&self) -> Result<(File, u64), Box<dyn Error>> {
        self.ensure_dir_exists()?;

        let current_file_path = self.make_file_path(Utc::now());
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(current_file_path.as_path())?;

        let current_len = file.metadata()?.len();
        Ok((file, current_len))
    }
}

/// Does the environment variable `name` have the value 'True' or 'true'?
fn is_truthy(name: impl AsRef<str>) -> Result<bool, Box<dyn Error>> {
    match env::var(name.as_ref()) {
        // The evironment variable contains a truthy value
        Ok(ref v) if v == "True" || v == "true" => return Ok(true),
        // The environment variable is not set or doesn't contain
        // a truthy value
        Ok(_) | Err(env::VarError::NotPresent) => return Ok(false),
        // The environment variable is invalid
        Err(e) => Err(e)?,
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let file_set_var = env::var("SEQ_APP_SETTING_FILESET")
        .map_err(|_| AppError("the `SEQ_APP_SETTING_FILESET` environment variable is not set"))?;

    let file_set = FileSet::new(
        &file_set_var,
        if is_truthy("SEQ_APP_SETTING_READABLEFILENAMES")? {
            FilenameFormat::Readable
        } else {
            FilenameFormat::Timestamp
        })?;

    let chunk_size_var = env::var("SEQ_APP_SETTING_CHUNKSIZE")
        .unwrap_or(String::new());

    let chunk_size = if chunk_size_var.len() > 0 {
        chunk_size_var
            .parse::<u64>()
            .map_err(|_| AppError("the `SEQ_APP_SETTING_CHUNKSIZE` environment variable could not be parsed as an integer"))?
    } else {
        104857600
    };

    let stdin = io::stdin();

    let (mut file, mut current_len) = file_set.open_next_file()?;
    const NEWLINE_LEN : u64 = 1;

    for input in stdin.lock().lines() {
        let line = input?;

        if (current_len + NEWLINE_LEN + line.len() as u64) > chunk_size {
            let (f, c) = file_set.open_next_file()?;
            file = f;
            current_len = c;
        }

        writeln!(file, "{}", line)?;
        current_len += NEWLINE_LEN + line.len() as u64;
    }

    Ok(())
}

fn main() {
    ::std::process::exit(match run() {
        Ok(_) => 0,
        Err(err) => {
            let err_str = err.to_string();
            let evt = FatalErrorEvent::new(&err_str);
            let json = serde_json::to_string(&evt).expect("infallible json");
            eprintln!("{}", json);
            1
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    impl<'a> FileSet<'a> {
        fn dir_name(&self) -> &str {
            self.dir.to_str().unwrap()
        }

        fn file_name_template(&self) -> &str {
            &self.file_name_template
        }
    }

    fn join_path(parts: &[&'static str]) -> String {
        let mut buf = PathBuf::new();
        for p in parts {
            buf.push(p);
        }
        buf.to_string_lossy().to_string()
    }
    
    #[test]
    fn file_set_is_split() {
        let p = join_path(&["path", "to", "log-*.clef"]);
        let file_set = FileSet::new(&p, FilenameFormat::Timestamp).unwrap();

        let q = join_path(&["path", "to"]);
        assert_eq!(&q, file_set.dir_name());
        assert_eq!("log-*.clef", file_set.file_name_template());
    }

    #[test]
    fn timestamped_file_path_constructed() {
        let p = join_path(&["path", "to", "log-*.clef"]);
        let file_set = FileSet::new(&p, FilenameFormat::Timestamp).unwrap();
        let timestamp = DateTime::<Utc>::from_utc(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11), Utc);
        let path = file_set.make_file_path(timestamp);

        let q = join_path(&["path", "to", "log-00000000577f6df3.clef"]);
        assert_eq!(&q, path.to_str().unwrap());
    }

    #[test]
    fn readable_file_path_constructed() {
        let p = join_path(&["path", "to", "log-*.clef"]);
        let file_set = FileSet::new(&p, FilenameFormat::Readable).unwrap();
        let timestamp = DateTime::<Utc>::from_utc(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11), Utc);
        let path = file_set.make_file_path(timestamp);

        let q = join_path(&["path", "to", "log-2016-07-08T09_10_11Z.clef"]);
        assert_eq!(&q, path.to_str().unwrap());
    }
}
