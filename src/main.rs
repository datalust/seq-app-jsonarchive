extern crate chrono;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde_derive;

use std::result::Result;
use failure::{Error,Fail,err_msg};
use std::env;
use chrono::{DateTime, Utc};
use std::io::{self, Write};
use std::io::prelude::*;
use std::fs::{self, OpenOptions, File};
use std::path::{PathBuf, Path};

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
    pub fn new<'b>(failure: &'b str) -> FatalErrorEvent<'b> {
        FatalErrorEvent {
            timestamp: Utc::now(),
            message_template: "Unable to create JSON archive: {Failure}",
            failure: failure,
            level: "Fatal"
        }
    }
}

fn parse_file_set<'a>(file_set: &'a str) -> Result<(&'a Path, String), Error> {
    let file_set_path = Path::new(file_set);

    let dir = file_set_path.parent()
        .ok_or(err_msg("the file set must specify a filename"))?;

    let file_set_filename = file_set_path.file_name()
        .ok_or(err_msg("the 00000000577f6df300000000577f6df3file set must specify a filename pattern"))?;

    let fn_template = file_set_filename.to_os_string().into_string()
        .map_err(|_| err_msg("filename character set conversion failed"))?;

    ensure!(fn_template.contains("*"), "the filename pattern must include the `*` wildcard");

    Ok((dir, fn_template))
}

fn make_file_path(dir: &Path, fn_template: &str, timestamp: DateTime<Utc>) -> PathBuf {
    let timestamp = format!("{:01$x}", timestamp.timestamp(), 16);
    let full_file_name = fn_template.replace("*", &timestamp);
    let mut buf = dir.to_path_buf();
    buf.push(full_file_name);
    buf
}

fn open_file(dir: &Path, fn_template: &str) -> Result<(File, u64), Error> {
    let current_file_path = make_file_path(dir, fn_template, Utc::now());

    let file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(current_file_path.as_path())?;

    let current_len = file.metadata()?.len();
    Ok((file, current_len))
}

fn run() -> Result<(), Error> {
    let file_set = env::var("SEQ_APP_SETTING_FILESET")
        .map_err(|e| e.context("the `SEQ_APP_SETTING_FILESET` environment variable is not set"))?;

    let (dir, fn_template) = parse_file_set(&file_set)?;

    let chunk_size : u64 = env::var("SEQ_APP_SETTING_CHUNKSIZE")
        .unwrap_or("104857600".to_string())
        .parse()
        .map_err(|_| failure::err_msg("the `SEQ_APP_SETTING_CHUNKSIZE` environment variable could not be parsed as an integer"))?;
    
    fs::create_dir_all(dir)?;

    let stdin = io::stdin();

    let (mut file, mut current_len) = open_file(&dir, &fn_template)?;
    const NEWLINE_LEN : u64 = 1;

    for input in stdin.lock().lines() {
        let line = input.unwrap();

        if (current_len + NEWLINE_LEN + line.len() as u64) > chunk_size {
            let (f, c) = open_file(&dir, &fn_template)?;
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
            let json = serde_json::to_string(&evt).unwrap();
            eprintln!("{}", json);
            1
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    
    #[test]
    fn file_set_is_split() {
        let file_set = "/path/to/log-*.clef";
        let (dir, fn_template) = parse_file_set(file_set).unwrap();
        assert_eq!("/path/to", dir.to_str().unwrap());
        assert_eq!("log-*.clef", &fn_template);
    }

    #[test]
    fn timestamped_file_path_constructed() {
        let (dir, fn_template) = (Path::new("/path/to"), "log-*.clef");
        let timestamp = DateTime::<Utc>::from_utc(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11), Utc);
        let path = make_file_path(dir, fn_template, timestamp);
        assert_eq!("/path/to/log-00000000577f6df3.clef", path.to_str().unwrap());
    }

}
