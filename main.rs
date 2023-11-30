use std::env;
use std::io::{self, BufRead};
use std::io::Write;
use std::process::Command;

use serde::{Deserialize, Serialize};

use simple_home_dir::expand_tilde;
use file_rotate::{FileRotate, ContentLimit, suffix::AppendCount, compression::Compression};

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
    let home_dir = expand_tilde("~/.git-lfs-agent-rclone").unwrap();
    let log_dir = home_dir.join("logs");
    // create log dir if not exist
    if !log_dir.exists() {
        std::fs::create_dir_all(log_dir.clone()).unwrap();
    }
    let log_file_path = log_dir.join("errors.log");
    log_file_path.to_str().unwrap().to_string()
}

fn get_log_file() -> FileRotate<AppendCount> {
    let log_file_path = get_log_file_path();
    let log_file = FileRotate::new(
        log_file_path.clone(),
        AppendCount::new(3),
        ContentLimit::Lines(1000),
        Compression::None,
        #[cfg(unix)]
        None,
    );
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
    let mut args: Vec<String> = env::args().collect();

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    // show help version
    if args.len() == 2 && (args[1] == "-h" || args[1] == "--help") {
        println!("git-lfs-agent-rclone v{}", VERSION);
        println!("Usage: git-lfs-agent-rclone <scp args>");
        // exit
        std::process::exit(0);

    // show version
    } else if args.len() == 2 && (args[1] == "-v" || args[1] == "--version") {
        println!("git-lfs-agent-rclone v{}", VERSION);
        // exit
        std::process::exit(0);
    }

    // set environment variable
    if let Some(index) = args.iter().position(|elem| elem == "--tmpdir") {
        args.remove(index);
        if args.len() <= index {
            respond(Response::Error(ErrorResponse {
                event: "initialize".to_string(),
                oid: "-1".to_string(),
                error: Error {
                    code: 1,
                    message: "`--tmpdir` needs a temporary directory path".to_string(),
                },
            }));
            std::process::exit(1);
        }
        match std::fs::metadata(&args[index]) {
            Ok(path) => {
                if !path.is_dir() {
                    respond(Response::Error(ErrorResponse {
                        event: "initialize".to_string(),
                        oid: "-1".to_string(),
                        error: Error {
                            code: 1,
                            message: "given temporary directory path is invalid.".to_string(),
                        },
                    }));
                    std::process::exit(1);
                }
            }
            Err(mes) => {
                respond(Response::Error(ErrorResponse {
                    event: "initialize".to_string(),
                    oid: "-1".to_string(),
                    error: Error {
                        code: 1,
                        message: mes.to_string(),
                    },
                }));
                std::process::exit(1);
            }
        }
        std::env::set_var("TMPDIR", &args[index]);
        args.remove(index);
    }

    let rclone_args = &args[1..];
    let rclone_arg_str = rclone_args.join(" ");

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
            // call rclone to upload file
            let src_path = &ev.path;
            // let dst_path = Path::new(&rclone_arg_str).join(&ev.oid);
            let sep = "/";
            let dst_path = format!("{}{}{}", &rclone_arg_str, sep, &ev.oid);
            let cmd = Command::new("rclone")
                .arg("copy")
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
                writeln!(log_file, "upload failed: {:?} ( request json: {:?})", cmd, ev).unwrap();
            }
        } else if event == "download" {
            let ev = match obj {
                Event::Download(e) => e,
                _ => panic!("invalid event"),
            };
            // call rclone to download file
            let _tmp_dir = tempfile::tempdir().unwrap();
            let tmp_path = _tmp_dir.into_path();

            let sep = "/";
            // let src_path = Path::new(&rclone_arg_str).join(&ev.oid);
            let src_path = format!("{}{}{}", &rclone_arg_str, sep, &ev.oid);
            let cmd = Command::new("rclone")
                .arg("copy")
                .arg(src_path)
                .arg(tmp_path.to_str().unwrap().to_string())
                .output()
                .expect("failed to execute process");
            if cmd.status.success() {
                respond(Response::Download(DownloadResponse {
                    event: "complete".to_string(),
                    oid: ev.oid.clone(),
                    path: tmp_path.join(&ev.oid).to_str().unwrap().to_string(),
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
                writeln!(log_file, "download failed: {:?} ( request json: {:?})", cmd, ev).unwrap();
            }
        }
    }
}