use crate::core::DbConnection;
use crate::errors;

use database::episode::Episode;
use database::genre::*;
use database::media::Media;
use database::mediafile::MediaFile;
use database::progress::Progress;
use database::season::Season;

use serde_json::json;
use serde_json::Value as JsonValue;

pub mod auth;
pub mod dashboard;
pub mod general;
pub mod library;
pub mod media;
pub mod mediafile;
pub mod settings;
pub mod statik;
pub mod stream;
pub mod tv;

pub mod global_filters {
    use crate::errors;
    use database::DbConnection;

    use std::convert::Infallible;
    use std::error::Error;
    use warp::Filter;
    use warp::Reply;

    pub fn with_db(
        conn: DbConnection,
    ) -> impl Filter<Extract = (DbConnection,), Error = Infallible> + Clone {
        warp::any().map(move || conn.clone())
    }

    pub fn with_state<T: Send + Clone>(
        state: T,
    ) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
        warp::any().map(move || state.clone())
    }

    pub async fn handle_rejection(
        err: warp::reject::Rejection,
    ) -> Result<impl warp::Reply, warp::reject::Rejection> {
        if let Some(e) = err.find::<errors::AuthError>() {
            return Ok(e.clone().into_response());
        } else if let Some(e) = err.find::<errors::DimError>() {
            return Ok(e.clone().into_response());
        } else if let Some(_) = err.find::<auth::JWTError>() {
            return Ok(errors::DimError::AuthRequired.into_response());
        } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
            return Ok(errors::DimError::MissingFieldInBody {
                description: e.source().unwrap().to_string(),
            }
            .into_response());
        }

        Err(err)
    }

    pub fn api_not_found(
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / ..)
            .and(warp::any())
            .map(|| crate::errors::DimError::NotFoundError)
    }
}

pub async fn get_season(conn: &DbConnection, data: &Media) -> Result<Season, errors::DimError> {
    Ok(Season::get_first(conn, data.id).await?)
}

pub async fn get_episode(conn: &DbConnection, data: &Season) -> Result<Episode, errors::DimError> {
    Ok(Episode::get_first_for_season(conn, data.id).await?)
}

pub async fn construct_standard_quick(data: &Media) -> Result<JsonValue, errors::DimError> {
    Ok(json!({
        "id": data.id,
        "name": data.name,
        "library_id": data.library_id
    }))
}

pub async fn construct_standard(
    conn: &DbConnection,
    data: &Media,
    user: &::auth::Wrapper,
) -> Result<JsonValue, errors::DimError> {
    // TODO: convert to enums
    let duration = MediaFile::get_largest_duration(conn, data.id).await?;
    let season = get_season(conn, data).await;

    let genres = Genre::get_by_media(&conn, data.id)
        .await?
        .into_iter()
        .map(|x| x.name)
        .collect::<Vec<String>>();

    if let Ok(season) = season {
        if let Ok(episode) = get_episode(conn, &season).await {
            let progress = Progress::get_for_media_user(conn, user.0.claims.get_user(), episode.id)
                .await
                .map(|x| x.delta)
                .unwrap_or(0);

            let duration = MediaFile::get_largest_duration(conn, episode.id).await?;

            return Ok(json!({
                "id": data.id,
                "library_id": data.library_id,
                "name": data.name,
                "description": data.description,
                "rating": data.rating,
                "year": data.year,
                "added": data.added,
                "poster_path": data.poster_path,
                "backdrop_path": data.backdrop_path,
                "media_type": data.media_type,
                "genres": genres,
                "duration": duration,
                "episode": episode.episode,
                "season": season.season_number,
                "progress": progress,
            }));
        }
    }

    let progress = Progress::get_for_media_user(conn, user.0.claims.get_user(), data.id)
        .await
        .map(|x| x.delta)
        .unwrap_or(0);
    Ok(json!({
        "id": data.id,
        "library_id": data.library_id,
        "name": data.name,
        "description": data.description,
        "rating": data.rating,
        "year": data.year,
        "added": data.added,
        "poster_path": data.poster_path,
        "backdrop_path": data.backdrop_path,
        "media_type": data.media_type,
        "genres": genres,
        "duration": duration,
        "progress": progress,
    }))
}
