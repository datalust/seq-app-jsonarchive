extern crate chrono;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde_derive;

use std::result::Result;
use failure::{Error,Fail};
use std::env;
use chrono::{DateTime, Utc};
use std::io::{self, Write};
use std::io::prelude::*;
use std::fs::{self, OpenOptions, File};
use std::path::Path;

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

fn open_file(dir: &Path, fn_template: &str) -> Result<(File, u64), Error> {
    let timestamp = format!("{:01$x}", Utc::now().timestamp(), 16);
    let full_file_name = fn_template.replace("*", &timestamp);
    let current_file_path = dir.to_path_buf()
        .with_file_name(full_file_name);

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

    let file_set_path = Path::new(&file_set);

    let dir = file_set_path.parent()
        .ok_or(failure::err_msg("the file set must specify a filename"))?;

    let file_set_filename = file_set_path.file_name()
        .ok_or(failure::err_msg("the file set must specify a filename pattern"))?;

    let fn_template = file_set_filename.to_os_string().into_string().unwrap();
    ensure!(fn_template.contains("*"), "the filename pattern must include the `*` wildcard");

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
