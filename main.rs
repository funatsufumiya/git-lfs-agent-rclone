use std::env;
use std::io::{self, BufRead};
use std::process::Command;
use std::fs::File;
use std::io::prelude::*;

use serde::{Deserialize, Serialize};

// event json examples
// init
// { "event": "init", "operation": "download", "remote": "origin", "concurrent": true, "concurrenttransfers": 3 }
// upload
// { "event": "upload", "oid": "bf3e3e2af9366a3b704ae0c31de5afa64193ebabffde2091936ad2e7510bc03a", "size": 346232, "path": "/path/to/file.png", "action": { "href": "nfs://server/path", "header": { "key": "value" } } }
// download
// { "event": "download", "oid": "22ab5f63670800cc7be06dbed816012b0dc411e774754c7579467d2536a9cf3e", "size": 21245, "action": { "href": "nfs://server/path", "header": { "key": "value" } } }

#[derive(Serialize, Deserialize, Debug)]
struct EventInit {
    event: String,
    operation: String,
    remote: String,
    concurrent: bool,
    concurrenttransfers: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct EventTerminate {
    event: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EventUpload {
    event: String,
    oid: String,
    size: u64,
    path: String,
    action: Option<Action>,
}

#[derive(Serialize, Deserialize, Debug)]
struct EventDownload {
    event: String,
    oid: String,
    size: u64,
    action: Option<Action>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Action {
    href: String,
    header: Header,
}

#[derive(Serialize, Deserialize, Debug)]
struct Header {
    key: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Event {
    Init(EventInit),
    Upload(EventUpload),
    Download(EventDownload),
    Terminate(EventTerminate),
}

#[derive(Serialize, Deserialize, Debug)]
struct DownloadResponse {
    event: String,
    oid: String,
    path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UploadResponse {
    event: String,
    oid: String
}

#[derive(Serialize, Deserialize, Debug)]
struct EmptyResponse {}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorResponse {
    event: String,
    oid: String,
    error: Error
}

#[derive(Serialize, Deserialize, Debug)]
struct Error {
    code: u64,
    message: String,
}

enum Response {
    Download(DownloadResponse),
    Upload(UploadResponse),
    Empty(EmptyResponse),
    Error(ErrorResponse),
}


fn respond(obj: Response) {
    // let json = serde_json::to_string(&obj).unwrap()
    // println!("{}", json);
    let json = match obj {
        Response::Download(download_response) => serde_json::to_string(&download_response).unwrap(),
        Response::Upload(upload_response) => serde_json::to_string(&upload_response).unwrap(),
        Response::Empty(empty_response) => serde_json::to_string(&empty_response).unwrap(),
        Response::Error(error_response) => serde_json::to_string(&error_response).unwrap(),
    };

    println!("{}", json);
}

fn get_log_file_path() -> String {
    let script_dir = env::current_dir().unwrap();
    let log_dir = script_dir.join("logs");
    // create log dir if not exist
    if !log_dir.exists() {
        std::fs::create_dir(log_dir.clone()).unwrap();
    }
    let log_file_path = log_dir.join("git-lfs-rs.error.log");
    log_file_path.to_str().unwrap().to_string()
}

fn get_log_file() -> File {
    let log_file_path = get_log_file_path();
    let log_file = File::create(log_file_path).unwrap();
    log_file
}

fn get_event (ev: &Event) -> String {
    match ev {
        Event::Init(_) => "init".to_string(),
        Event::Terminate(_) => "terminate".to_string(),
        Event::Upload(_) => "upload".to_string(),
        Event::Download(_) => "download".to_string(),
    }
}

fn main() {
    // get args
    let args: Vec<String> = env::args().collect();
    let scp_args = &args[1..];
    let scp_arg_str = scp_args.join(" ");

    // read stdin
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let obj:Event = serde_json::from_str(&line).unwrap();
        let event = get_event(&obj);

        // // write to log for debug (with typename of obj)
        // let mut log_file = get_log_file();
        // log_file.write_all(format!("{}: {}\n", event, line).as_bytes()).unwrap();

        if event == "init" {
            respond(Response::Empty(EmptyResponse {}));
        } else if event == "terminate" {
            // pass
        } else if event == "upload" {
            let ev = match obj {
                Event::Upload(ref e) => e,
                _ => panic!("invalid event"),
            };
            // call scp to upload file
            let src_path = &ev.path;
            // let dst_path = Path::new(&scp_arg_str).join(&ev.oid);
            let sep = "/";
            let dst_path = format!("{}{}{}", &scp_arg_str, sep, &ev.oid);
            let cmd = Command::new("scp")
                .arg(src_path)
                .arg(dst_path)
                .output()
                .expect("failed to execute process");
            if cmd.status.success() {
                respond(Response::Upload(UploadResponse {
                    event: "complete".to_string(),
                    oid: ev.oid.clone(),
                }));
            } else {
                respond(Response::Error(ErrorResponse {
                    event: "complete".to_string(),
                    oid: ev.oid.clone(),
                    error: Error {
                        code: 1,
                        message: "Upload failed".to_string(),
                    },
                }));

                // error log
                let mut log_file = get_log_file();
                log_file.write_all(format!("upload failed: {:?} ( request json: {:?})", cmd, ev).as_bytes()).unwrap()
            }
        } else if event == "download" {
            let ev = match obj {
                Event::Download(e) => e,
                _ => panic!("invalid event"),
            };
            // call scp to download file
            let (_tmp_file, tmp_path_buf) = tempfile::NamedTempFile::new().unwrap().keep().unwrap();
            let tmp_path = tmp_path_buf.to_str().unwrap();

            let sep = "/";
            // let src_path = Path::new(&scp_arg_str).join(&ev.oid);
            let src_path = format!("{}{}{}", &scp_arg_str, sep, &ev.oid);
            let cmd = Command::new("scp")
                .arg(src_path)
                .arg(tmp_path)
                .output()
                .expect("failed to execute process");
            if cmd.status.success() {
                respond(Response::Download(DownloadResponse {
                    event: "complete".to_string(),
                    oid: ev.oid.clone(),
                    path: tmp_path.to_string(),
                }));
            } else {
                respond(Response::Error(ErrorResponse {
                    event: "complete".to_string(),
                    oid: ev.oid.clone(),
                    error: Error {
                        code: 1,
                        message: "Download failed".to_string(),
                    },
                }));

                // error log
                let mut log_file = get_log_file();
                log_file.write_all(format!("download failed: {:?} ( request json: {:?})", cmd, ev).as_bytes()).unwrap();
            }
        }
    }
}