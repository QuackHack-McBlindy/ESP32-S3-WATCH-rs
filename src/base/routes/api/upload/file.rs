// BASE/ROUTES/API/UPLOAD/FILE
// STREAM FILE UPLOADS TO SD CARD - CHUNK BY CHUNK
// VERIFYING CHUNK SIZE BETWEEN EACH TRANSFER TO AVOID CORRUPT DATA

// ───────────────────────────────────────────────────────────────────────
// USAGE:

// curl -X POST \
//  -H "Transfer-Encoding: chunked" \
//  --data-binary @/Path/To/File.mp3 \
//  "http://<ESP_IP>:80/api/upload/file/music/<FILE_NAME.mp3>"

//curl -X POST \
//  -H "Transfer-Encoding: chunked" \
//  --data-binary @/Pool/Music/Anka/ducksong2.mp3 \
//  "http://192.168.1.182:80/api/upload/file/music/ducksong2.mp3"

// ───────────────────────────────────────────────────────────────────────
// UPLOAD MP3 FILE TO `/Music` ON SD CARD

pub struct MusicUpload;

impl tinyapi::UploadHandler for MusicUpload {
    fn handle<'s>(
        &'s self,
        mut socket: embassy_net::tcp::TcpSocket<'s>,
        req: tinyapi::UploadRequest,
    ) -> core::pin::Pin<
        alloc::boxed::Box<
            dyn core::future::Future<
                Output = (embassy_net::tcp::TcpSocket<'s>, tinyapi::Response),
            > + 's,
        >,
    > {
        alloc::boxed::Box::pin(async move {
            let filename = req.param("filename").unwrap_or("unknown.mp3");
            let path = alloc::format!("/Music/{}", filename);
            defmt::info!("Receiving upload: {}", path.as_str());

            let result = crate::components::storage::create_file_for_writing(&path);
            match result {
                Ok(mut file) => {
                    match tinyapi::receive_chunked_body(&mut socket, &mut file).await {
                        Ok(()) => {
                            // Properly close the file and handle any error
                            match file.close() {
                                Ok(()) => {
                                    defmt::info!("Upload complete: {}", path.as_str());
                                    (socket, tinyapi::Response::text("Upload OK"))
                                }
                                Err(e) => {
                                    defmt::error!("Close failed: {:?} for {}", e, path.as_str());
                                    (socket, tinyapi::Response::text("Upload OK (close error)"))
                                }
                            }
                        }
                        Err(e) => {
                            defmt::error!("Chunked receive failed: {:?}", e);
                            // file will be dropped and auto-closed (best effort)
                            (socket, tinyapi::Response::text("Upload failed"))
                        }
                    }
                }
                Err(e) => {
                    defmt::error!("Could not create file: {:?} for {}", e, path.as_str());
                    (socket, tinyapi::Response::text("Could not create file"))
                }
            }
        })
    }
}
