use alleycat_rs::startup::run;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::Duration;

#[tokio::test]
async fn oversized_upload_announces_close_and_preserves_the_error_response() {
    // Exercise the real server's shared Bytes extractor through one representative
    // route. Rejection happens before the handler needs a database connection.
    let pool = PgPoolOptions::new().connect_lazy_with(PgConnectOptions::new());
    pool.close().await;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = run(listener, pool).unwrap();
    let handle = server.handle();
    let server_task = tokio::spawn(server);

    let result = tokio::task::spawn_blocking(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
        const BODY_SIZE: usize = 262_145;
        write!(
            stream,
            "POST /device-logs HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/protobuf\r\nContent-Length: {BODY_SIZE}\r\n\r\n"
        )
        .unwrap();

        // Withhold the body until the early response arrives. This guarantees
        // unread request data, instead of depending on TCP buffer timing.
        let mut reader = BufReader::new(stream);
        let mut status = String::new();
        reader.read_line(&mut status).unwrap();
        assert!(status.starts_with("HTTP/1.1 413 "), "{status}");
        let mut closes_connection = false;
        let mut content_length = None;
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0, "Truncated headers");
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            if name.eq_ignore_ascii_case("connection") {
                closes_connection = value.trim().eq_ignore_ascii_case("close");
            }
            if name.eq_ignore_ascii_case("content-length") {
                content_length = Some(value.trim().parse::<usize>().unwrap());
            }
        }
        assert!(closes_connection, "413 must prevent reuse of this connection");

        // A client may still be uploading when the rejection arrives. The
        // server must drain during close, preserving the response instead of
        // resetting the socket while the client finishes its write.
        reader.get_mut().write_all(&vec![0; BODY_SIZE]).unwrap();
        reader.get_mut().shutdown(Shutdown::Write).unwrap();
        let mut body = Vec::new();
        reader.read_to_end(&mut body).unwrap();
        assert!(!body.is_empty());
        assert_eq!(Some(body.len()), content_length);
    })
    .await;

    handle.stop(true).await;
    server_task.await.unwrap().unwrap();
    result.unwrap();
}
