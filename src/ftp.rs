use async_ftp::{types::FileType, FtpStream};
use crate::output::get_output;
use failure::format_err;
use url::Url;

fn parse_ftp_address(address: &str) -> (String, String, String, Vec<String>, String) {
    let url = Url::parse(address).unwrap();
    let ftp_server = format!(
        "{}:{}",
        url .host_str()
            .ok_or_else(|| format_err!("failed to parse hostname from url: {}", url))
            .unwrap(),
        url .port_or_known_default()
            .ok_or_else(|| format_err!("failed to parse port from url: {}", url))
            .unwrap(),
    );
    let username = if url.username().is_empty() {
        "anonymous".to_string()
    } else {
        url.username().to_string()
    };
    let password = url.password().unwrap_or("anonymous").to_string();

    let mut path_segments: Vec<String> = url
        .path_segments()
        .ok_or_else(|| format_err!("failed to get url path segments: {}", url))
        .unwrap()
        .map(|s| s.to_string())
        .collect();

    let file = path_segments
        .pop()
        .ok_or_else(|| format_err!("got empty path segments from url: {}", url))
        .unwrap();

    (ftp_server, username, password, path_segments, file.to_string())
}
pub async fn get(url: &str, path: &str) -> String {
    let (mut output, mut downloaded) = get_output(path);
    let (ftp_server, ref username, ref password, path_segments, ref file) = parse_ftp_address(url);

    let mut ftp_stream = FtpStream::connect(ftp_server).await.unwrap();
    let _ = ftp_stream.login(username, password).await.unwrap();

    for path in &path_segments {
        ftp_stream.cwd(&path).await.unwrap();
    }

    ftp_stream.transfer_type(FileType::Binary).await;
    let total_size = downloaded + ftp_stream
        .size(file)
        .await
        .unwrap()
        .unwrap() as u64;
    // ftp_stream.restart_from(downloaded); Unsupported yet, see: https://github.com/mattnenterprise/rust-ftp/issues/67 
    let remote_file = ftp_stream.simple_retr(file).await.unwrap();
    let contents = std::str::from_utf8(&remote_file.into_inner()).unwrap().to_string();
    println!("Read file with contents\n{}\n", contents);

    let _ = ftp_stream.quit();
    contents
}

#[tokio::test]
async fn get_ftp_works() {
    let out_file = "";
    let contents = get("ftp://ftp.fau.de:21/gnu/ProgramIndex", out_file).await;
    let first_line = contents.split(' ').collect::<Vec<&str>>();
    assert!(first_line[0].starts_with("Here"));
}