// BASE/ROUTES/API/DOWNLOAD/FILE
// STREAM FILE DOWNLOADS FROM SD CARD - CHUNK BY CHUNK
// VERIFYING CHUNK SIZE BETWEEN EACH TRANSFER TO AVOID CORRUPT DATA

// ───────────────────────────────────────────────────────────────────────
// USAGE:

// ───────────────────────────────────────────────────────────────────────
// DOWNLOAD MP3 FILE FROM `/Music` ON SD CARD

pub struct MusicDownload;

impl tinyapi::StreamHandler for MusicDownload {
    fn handle<'s>(
        &'s self,
        socket: embassy_net::tcp::TcpSocket<'s>,
        req: tinyapi::Request<'s>,
    ) -> core::pin::Pin<
        alloc::boxed::Box<
            dyn core::future::Future<
                Output = (embassy_net::tcp::TcpSocket<'s>, Result<(), embassy_net::tcp::Error>),
            > + 's,
        >,
    > {
        alloc::boxed::Box::pin(async move {
            let filename = req.param("filename").unwrap_or("unknown.mp3");
            let full_path = alloc::format!("/Music/Music/{}", filename);
            defmt::info!("Streaming download: {}", full_path.as_str());

            let mut file = match crate::components::storage::open_file_stream(&full_path) {
                Ok(f) => f,
                Err(_) => {
                    defmt::error!("File not found: {}", full_path.as_str());
                    let mut sock = socket;
                    let _ = tinyapi::send_response(
                        &mut sock,
                        tinyapi::Response::not_found(),
                    )
                    .await;
                    return (sock, Ok(()));
                }
            };

            let mime = if filename.ends_with(".mp3") {
                "audio/mpeg"
            } else if filename.ends_with(".wav") {
                "audio/wav"
            } else {
                "application/octet-stream"
            };

            let mut sock = socket;
            let result = tinyapi::send_streaming_response(
                &mut sock,
                "200 OK",
                mime,
                &mut file,
            )
            .await;
            // file is dropped → SD file automatically closed
            (sock, result)
        })
    }
}


// ───────────────────────────────────────────────────────────────────────
// DOWNLOAD FILE FROM `/share` ON SD CARD

pub struct ShareDownload;

impl tinyapi::StreamHandler for ShareDownload {
    fn handle<'s>(
        &'s self,
        socket: embassy_net::tcp::TcpSocket<'s>,
        req: tinyapi::Request<'s>,
    ) -> core::pin::Pin<
        alloc::boxed::Box<
            dyn core::future::Future<
                Output = (embassy_net::tcp::TcpSocket<'s>, Result<(), embassy_net::tcp::Error>),
            > + 's,
        >,
    > {
        alloc::boxed::Box::pin(async move {
            let filename = req.param("filename").unwrap_or("file.bin");
            let full_path = alloc::format!("/share/{}", filename);
            defmt::info!("Streaming download: {}", full_path.as_str());

            let mut file = match crate::components::storage::open_file_stream(&full_path) {
                Ok(f) => f,
                Err(_) => {
                    defmt::error!("File not found: {}", full_path.as_str());
                    let mut sock = socket;
                    let _ = tinyapi::send_response(
                        &mut sock,
                        tinyapi::Response::not_found(),
                    )
                    .await;
                    return (sock, Ok(()));
                }
            };

            let mime = "application/octet-stream"; // or detect by extension

            let mut sock = socket;
            let result = tinyapi::send_streaming_response(
                &mut sock,
                "200 OK",
                mime,
                &mut file,
            )
            .await;
            // file is dropped → SD handle automatically closed
            (sock, result)
        })
    }
}
