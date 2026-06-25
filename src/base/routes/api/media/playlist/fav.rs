// BASE/ROUTES/API/MEDIA/PLAYLIST/FAV
// PLAYS MOST LIKED SONGS 

pub async fn fav_playlist_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    crate::applications::media_player::play_favourites().await;
    tinyapi::Response::text("Playing your favourite songs.")
}
