//! Real HTTP integration tests using raw tokio TCP sockets.
//!
//! Spins up actual TCP servers and makes real HTTP requests — no wiremock.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream as TokioTcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;

async fn start_counting_server(start: u32) -> (SocketAddr, Arc<Mutex<u32>>) {
    let counter = Arc::new(Mutex::new(start));
    let cnt = counter.clone();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let actual = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = [0u8; 8192];
            let n = match stream.read(&mut buf).await {
                Ok(n) => n,
                Err(_) => continue,
            };
            if n == 0 {
                continue;
            }
            let mut guard = cnt.lock().await;
            *guard += 1;
            let body = if *guard == 1 {
                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n"
            } else {
                "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"
            };
            let _ = stream.write_all(body.as_bytes()).await;
        }
    });
    (actual, counter)
}

#[tokio::test]
async fn test_http_connection_refused() {
    let result = TokioTcpStream::connect("127.0.0.1:1").await;
    assert!(result.is_err(), "expected connection refused on port 1");
}

#[tokio::test]
async fn test_http_server_returns_5xx_then_200() {
    let (addr, counter) = start_counting_server(0).await;
    sleep(Duration::from_millis(30)).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    let resp1 = client
        .get(format!("http://{}/test", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status().as_u16(), 500);

    let resp2 = client
        .get(format!("http://{}/test", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status().as_u16(), 200);

    let guard = counter.lock().await;
    assert_eq!(*guard, 2, "server should have been called twice");
}

#[tokio::test]
async fn test_http_redirect_307_followed() {
    let final_called = Arc::new(Mutex::new(false));

    let final_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let final_port = final_listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let mut stream = final_listener.accept().await.unwrap().0;
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        assert!(
            request.contains("PUT") || request.contains("GET"),
            "expected HTTP request, got: {}",
            request
        );
        *final_called.lock().await = true;
        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes()).await;
    });

    let redirect_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let redirect_port = redirect_listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let mut stream = redirect_listener.accept().await.unwrap().0;
        let mut buf = [0u8; 1024];
        let _n = stream.read(&mut buf).await.unwrap_or(0);
        let resp = format!(
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{}/final\r\nContent-Length: 0\r\n\r\n",
            final_port
        );
        let _ = stream.write_all(resp.as_bytes()).await;
    });

    sleep(Duration::from_millis(30)).await;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let resp = client
        .put(format!("http://127.0.0.1:{}/upload", redirect_port))
        .body(b"test data".as_slice())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 307);
    let location = resp.headers().get("location").unwrap().to_str().unwrap();

    let final_resp = client
        .put(location)
        .body(b"test data".as_slice())
        .send()
        .await
        .unwrap();
    assert_eq!(final_resp.status().as_u16(), 200);
}

#[tokio::test]
async fn test_http_raw_body_echoed_back() {
    // Server that echoes the full request body back
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = [0u8; 8192];
            let n = match stream.read(&mut buf).await {
                Ok(n) => n,
                Err(_) => continue,
            };
            if n == 0 {
                continue;
            }
            let s = String::from_utf8_lossy(&buf[..n]);
            if let Some(pos) = s.find("\r\n\r\n") {
                let body = &buf[pos + 4..n];
                let resp = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.write_all(body).await;
            } else {
                let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        }
    });

    sleep(Duration::from_millis(30)).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    let body = b"Hello, real HTTP body!";
    let resp = client
        .request(reqwest::Method::POST, format!("http://{}/echo", addr))
        .body(body.as_slice())
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body_text = resp.text().await.unwrap();
    assert_eq!(
        body_text.as_bytes(),
        b"Hello, real HTTP body!",
        "expected echoed body"
    );
}

#[tokio::test]
async fn test_http_connection_close_triggers_reconnect() {
    let request_count = Arc::new(Mutex::new(0u32));
    let counter = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _n = stream.read(&mut buf).await.unwrap_or(0);
            let mut guard = counter.lock().await;
            *guard += 1;
            let resp = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nHello"
                .to_string();
            let _ = stream.write_all(resp.as_bytes()).await;
            drop(stream);
        }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let resp1 = client
        .get(format!("http://{}/", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status().as_u16(), 200);
    sleep(Duration::from_millis(50)).await;

    let resp2 = client
        .get(format!("http://{}/", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status().as_u16(), 200);

    let guard = request_count.lock().await;
    assert_eq!(*guard, 2, "expected 2 separate connections");
}

#[tokio::test]
async fn test_http_tcp_echo_server() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let n = match stream.read(&mut buf).await {
                Ok(n) => n,
                Err(_) => continue,
            };
            if n == 0 {
                continue;
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                n,
                String::from_utf8_lossy(&buf[..n])
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });

    sleep(Duration::from_millis(30)).await;

    let mut stream = TokioTcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET /test HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();

    let mut buf = [0u8; 4096];
    let n = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf))
        .await
        .unwrap()
        .unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(
        response.contains("/test"),
        "expected echoed path, got: {}",
        response
    );
}
