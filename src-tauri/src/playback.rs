//! Local, session-scoped streaming proxy for the embedded Bilibili player.
use crate::{ApiResponse, AppState};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, io::Read, sync::Arc, time::Duration};
use tauri::Manager;
use tokio::sync::{OnceCell, RwLock};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Default)]
pub struct PlaybackState {
    server: OnceCell<ProxyServer>,
}

struct ProxyServer {
    base_url: String,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}
#[derive(Clone)]
struct Session {
    manifest: String,
    sources: HashMap<String, Vec<String>>,
}
#[derive(Clone)]
struct ProxyContext {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaybackPart {
    pub page: u32,
    pub title: String,
    pub duration: u64,
}
#[derive(Debug, Serialize)]
pub struct PlaybackQuality {
    id: u64,
    label: String,
}
#[derive(Debug, Serialize)]
pub struct PlaybackInfo {
    session_id: String,
    source: String,
    title: String,
    cid: u64,
    page: u32,
    duration: u64,
    quality: u64,
    qualities: Vec<PlaybackQuality>,
    parts: Vec<PlaybackPart>,
}
struct ResolvedVideo {
    title: String,
    bvid: String,
    cid: u64,
    ep_id: Option<u64>,
    page: u32,
    parts: Vec<PlaybackPart>,
}

fn allowed_media_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url
            .host_str()
            .map(|host| {
                ["bilivideo.com", "bilivideo.cn", "biliapi.net"]
                    .iter()
                    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
            })
            .unwrap_or(false)
}

fn media_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() < 5 && allowed_media_url(attempt.url().as_str()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|e| e.to_string())
}

fn api_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .no_deflate()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

fn allowed_origin(origin: &str) -> bool {
    reqwest::Url::parse(origin)
        .ok()
        .map(|url| {
            ["http", "https", "tauri"].contains(&url.scheme())
                && matches!(
                    url.host_str(),
                    Some("localhost" | "127.0.0.1" | "tauri.localhost")
                )
        })
        .unwrap_or(false)
}

fn with_cors(mut response: Response, origin: Option<&str>) -> Response {
    if let Some(origin) = origin.filter(|origin| allowed_origin(origin)) {
        if let Ok(value) = HeaderValue::from_str(origin) {
            response
                .headers_mut()
                .insert("access-control-allow-origin", value);
        }
    }
    for (key, value) in [
        ("access-control-allow-methods", "GET, HEAD, OPTIONS"),
        ("access-control-allow-headers", "Range"),
        (
            "access-control-expose-headers",
            "Content-Length, Content-Range, Accept-Ranges",
        ),
        ("cache-control", "no-store"),
        ("vary", "Origin"),
    ] {
        response
            .headers_mut()
            .insert(key, HeaderValue::from_static(value));
    }
    response
}

fn error_response(status: StatusCode, origin: Option<&str>) -> Response {
    with_cors(
        Response::builder()
            .status(status)
            .body(Body::empty())
            .unwrap(),
        origin,
    )
}

async fn proxy_options(headers: HeaderMap) -> Response {
    let origin = headers.get("origin").and_then(|h| h.to_str().ok());
    error_response(
        if origin.map(allowed_origin).unwrap_or(true) {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::FORBIDDEN
        },
        origin,
    )
}

async fn proxy_resource(
    State(context): State<ProxyContext>,
    Path((id, resource)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let origin = headers.get("origin").and_then(|h| h.to_str().ok());
    if origin.is_some_and(|value| !allowed_origin(value)) {
        return error_response(StatusCode::FORBIDDEN, origin);
    }
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if !host.starts_with("127.0.0.1:") && !host.starts_with("localhost:") {
        return error_response(StatusCode::FORBIDDEN, origin);
    }
    let Some(session) = context.sessions.read().await.get(&id).cloned() else {
        return error_response(StatusCode::NOT_FOUND, origin);
    };
    if resource == "manifest.mpd" {
        return with_cors(
            Response::builder()
                .header("content-type", "application/dash+xml")
                .body(Body::from(session.manifest))
                .unwrap(),
            origin,
        );
    }
    let Some(urls) = session.sources.get(&resource) else {
        return error_response(StatusCode::NOT_FOUND, origin);
    };
    let range = headers.get("range").and_then(|h| h.to_str().ok());
    if range.is_some_and(|value| !valid_range(value)) {
        return error_response(StatusCode::RANGE_NOT_SATISFIABLE, origin);
    }
    for url in urls {
        let mut request = context
            .client
            .get(url)
            .header("Referer", "https://www.bilibili.com/")
            .header("Accept-Encoding", "identity");
        if let Some(range) = range {
            request = request.header("Range", range);
        }
        let Ok(upstream) = request.send().await else {
            continue;
        };
        if !upstream.status().is_success() {
            continue;
        }
        let mut builder = Response::builder().status(upstream.status().as_u16());
        for name in [
            "content-type",
            "content-length",
            "content-range",
            "accept-ranges",
        ] {
            if let Some(value) = upstream.headers().get(name) {
                builder = builder.header(name, value.as_bytes());
            }
        }
        return with_cors(
            builder
                .body(Body::from_stream(upstream.bytes_stream()))
                .unwrap(),
            origin,
        );
    }
    error_response(StatusCode::BAD_GATEWAY, origin)
}

fn valid_range(range: &str) -> bool {
    let Some((start, end)) = range
        .strip_prefix("bytes=")
        .and_then(|value| value.split_once('-'))
    else {
        return false;
    };
    (!start.is_empty() || !end.is_empty())
        && start.chars().all(|c| c.is_ascii_digit())
        && end.chars().all(|c| c.is_ascii_digit())
}

async fn start_proxy() -> Result<ProxyServer, String> {
    let sessions = Arc::new(RwLock::new(HashMap::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("无法启动播放器：{e}"))?;
    let base_url = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let context = ProxyContext {
        sessions: sessions.clone(),
        client: media_client()?,
    };
    let router = Router::new()
        .route("/:id/:resource", get(proxy_resource).options(proxy_options))
        .with_state(context);
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Ok(ProxyServer { base_url, sessions })
}

async fn api_json(
    client: &reqwest::Client,
    path: &str,
    query: &[(&str, String)],
    cookies: &str,
) -> Result<Value, String> {
    let json: Value = client
        .get(format!("https://api.bilibili.com{path}"))
        .query(query)
        .header("Referer", "https://www.bilibili.com/")
        .header("Cookie", cookies)
        .send()
        .await
        .map_err(|e| format!("连接 B 站失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("播放请求失败：{e}"))?
        .json()
        .await
        .map_err(|_| "B 站返回了无法读取的播放信息".to_string())?;
    if json["code"].as_i64() != Some(0) {
        return Err(format!(
            "无法播放：{}",
            json["message"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("请稍后重试，或登录后再试")
        ));
    }
    let data = if json["data"].is_object() {
        &json["data"]
    } else {
        &json["result"]
    };
    if !data.is_object() {
        return Err("播放信息为空，请稍后重试".into());
    }
    Ok(data.clone())
}

fn url_page(url: &str) -> Option<u32> {
    reqwest::Url::parse(url)
        .ok()?
        .query_pairs()
        .find(|(key, _)| key == "p")?
        .1
        .parse::<u32>()
        .ok()
        .filter(|page| *page > 0)
}

async fn resolve_video(
    client: &reqwest::Client,
    url: &str,
    requested_page: Option<u32>,
    cookies: &str,
) -> Result<ResolvedVideo, String> {
    if requested_page == Some(0) {
        return Err("所选分 P 不存在".into());
    }
    let mut resolved_url = url.trim().to_string();
    if let Ok(short) = reqwest::Url::parse(&resolved_url) {
        if matches!(short.host_str(), Some("b23.tv" | "bili2233.cn")) {
            let response = client
                .get(short)
                .send()
                .await
                .map_err(|_| "短链接解析失败，请使用完整视频链接".to_string())?;
            let destination = response.url();
            if !matches!(
                destination.host_str(),
                Some("www.bilibili.com" | "m.bilibili.com" | "bilibili.com")
            ) {
                return Err("短链接未跳转到 B 站视频，请使用完整视频链接".into());
            }
            resolved_url = destination.to_string();
        }
    }
    let url = resolved_url.as_str();
    let (bvid, aid, season_id, ep_id) = crate::extract_video_id(url);
    if season_id.is_some() || ep_id.is_some() {
        let query = if let Some(ep) = ep_id {
            vec![("ep_id", ep.to_string())]
        } else {
            vec![("season_id", season_id.unwrap().to_string())]
        };
        let data = api_json(client, "/pgc/view/web/season", &query, cookies).await?;
        let episodes = data["episodes"].as_array().ok_or("无法获取番剧剧集")?;
        let selected = if let Some(page) = requested_page {
            page
        } else if let Some(ep) = ep_id {
            episodes
                .iter()
                .position(|item| item["id"].as_u64() == Some(ep))
                .map(|i| i as u32 + 1)
                .ok_or("该剧集无法播放")?
        } else {
            1
        };
        let item = episodes
            .get(selected.saturating_sub(1) as usize)
            .ok_or("所选剧集不存在")?;
        return Ok(ResolvedVideo {
            title: data["title"].as_str().unwrap_or("番剧").to_string(),
            bvid: String::new(),
            cid: item["cid"]
                .as_u64()
                .filter(|cid| *cid > 0)
                .ok_or("剧集缺少播放信息")?,
            ep_id: item["id"].as_u64(),
            page: selected,
            parts: episodes
                .iter()
                .enumerate()
                .map(|(i, ep)| PlaybackPart {
                    page: i as u32 + 1,
                    title: format!(
                        "{} {}",
                        ep["title"].as_str().unwrap_or(""),
                        ep["long_title"].as_str().unwrap_or("")
                    )
                    .trim()
                    .to_string(),
                    duration: ep["duration"].as_u64().unwrap_or(0) / 1000,
                })
                .collect(),
        });
    }
    let query = if let Some(bvid) = bvid {
        vec![("bvid", bvid)]
    } else if let Some(aid) = aid {
        vec![("aid", aid.to_string())]
    } else {
        return Err("请输入 B 站视频链接或 BV 号".into());
    };
    let data = api_json(client, "/x/web-interface/view", &query, cookies).await?;
    let pages = data["pages"].as_array().ok_or("视频缺少分 P 信息")?;
    let page = requested_page.or_else(|| url_page(url)).unwrap_or(1);
    let item = pages
        .iter()
        .find(|item| item["page"].as_u64() == Some(page as u64))
        .ok_or("所选分 P 不存在")?;
    Ok(ResolvedVideo {
        title: data["title"].as_str().unwrap_or("B站视频").to_string(),
        bvid: data["bvid"].as_str().ok_or("视频缺少 BV 号")?.to_string(),
        cid: item["cid"]
            .as_u64()
            .filter(|cid| *cid > 0)
            .ok_or("视频缺少播放信息")?,
        ep_id: None,
        page,
        parts: pages
            .iter()
            .map(|item| PlaybackPart {
                page: item["page"].as_u64().unwrap_or(1) as u32,
                title: item["part"].as_str().unwrap_or("").to_string(),
                duration: item["duration"].as_u64().unwrap_or(0),
            })
            .collect(),
    })
}

fn stream_urls(stream: &Value) -> Vec<String> {
    let base = stream["baseUrl"]
        .as_str()
        .or_else(|| stream["base_url"].as_str());
    let backups = stream["backupUrl"]
        .as_array()
        .or_else(|| stream["backup_url"].as_array());
    let mut urls: Vec<String> = base
        .into_iter()
        .chain(backups.into_iter().flatten().filter_map(Value::as_str))
        .filter(|url| allowed_media_url(url))
        .map(str::to_string)
        .collect();
    // Prefer ordinary HTTPS CDN mirrors to peer CDN endpoints with nonstandard ports.
    urls.sort_by_key(|url| {
        reqwest::Url::parse(url)
            .ok()
            .map(|url| url.port_or_known_default() != Some(443))
            .unwrap_or(true)
    });
    urls.dedup();
    urls
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn representation(stream: &Value, kind: &str) -> Result<String, String> {
    let segment = if stream["SegmentBase"].is_object() {
        &stream["SegmentBase"]
    } else {
        &stream["segment_base"]
    };
    let index = segment["indexRange"]
        .as_str()
        .or_else(|| segment["index_range"].as_str())
        .filter(|s| valid_range(&format!("bytes={s}")))
        .ok_or("视频索引不可用")?;
    let init = segment["Initialization"]
        .as_str()
        .or_else(|| segment["initialization"].as_str())
        .filter(|s| valid_range(&format!("bytes={s}")))
        .ok_or("视频初始化信息不可用")?;
    let codecs = stream["codecs"].as_str().ok_or("视频编码信息不可用")?;
    let dimensions = if kind == "video" {
        format!(
            " width=\"{}\" height=\"{}\"",
            stream["width"].as_u64().unwrap_or(0),
            stream["height"].as_u64().unwrap_or(0)
        )
    } else {
        String::new()
    };
    Ok(format!("<AdaptationSet contentType=\"{kind}\" mimeType=\"{kind}/mp4\" segmentAlignment=\"true\"><Representation id=\"{kind}\" bandwidth=\"{}\" codecs=\"{}\"{dimensions}><BaseURL>{kind}</BaseURL><SegmentBase indexRange=\"{}\" indexRangeExact=\"true\"><Initialization range=\"{}\"/></SegmentBase></Representation></AdaptationSet>", stream["bandwidth"].as_u64().unwrap_or(100000), xml_escape(codecs), xml_escape(index), xml_escape(init)))
}

async fn prepare_playback(
    server: &ProxyServer,
    url: &str,
    page: Option<u32>,
    quality: Option<u64>,
    cookies: &str,
) -> Result<PlaybackInfo, String> {
    let client = api_client()?;
    let video = resolve_video(&client, url, page, cookies).await?;
    let mut query = vec![
        ("cid", video.cid.to_string()),
        ("qn", quality.unwrap_or(80).to_string()),
        ("fnval", "16".into()),
        ("fourk", "1".into()),
    ];
    let path = if let Some(ep) = video.ep_id {
        query.push(("ep_id", ep.to_string()));
        "/pgc/player/web/playurl"
    } else {
        query.push(("bvid", video.bvid));
        "/x/player/playurl"
    };
    let data = api_json(&client, path, &query, cookies).await?;
    if data["drm_tech_type"].as_u64().unwrap_or(0) > 0 {
        return Err("此视频采用受保护的播放格式，请在 B 站观看".into());
    }
    let dash = &data["dash"];
    let streams = dash["video"]
        .as_array()
        .ok_or("此视频暂无可用的在线播放地址，请确认登录状态或在 B 站观看")?;
    let mut streams: Vec<&Value> = streams
        .iter()
        .filter(|stream| {
            stream["codecs"]
                .as_str()
                .is_some_and(|s| s.starts_with("avc1"))
                && !stream_urls(stream).is_empty()
        })
        .collect();
    streams.sort_by_key(|stream| stream["id"].as_u64().unwrap_or(0));
    let chosen = streams
        .iter()
        .rev()
        .find(|stream| stream["id"].as_u64().unwrap_or(0) <= quality.unwrap_or(80))
        .or_else(|| streams.first())
        .ok_or("当前设备没有可播放的视频编码")?;
    let audio = dash["audio"].as_array().and_then(|streams| {
        streams
            .iter()
            .filter(|stream| {
                stream["codecs"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("mp4a"))
                    && !stream_urls(stream).is_empty()
            })
            .max_by_key(|stream| stream["bandwidth"].as_u64().unwrap_or(0))
    });
    let duration = dash["duration"]
        .as_u64()
        .or_else(|| data["timelength"].as_u64().map(|ms| ms.div_ceil(1000)))
        .filter(|value| *value > 0)
        .ok_or("视频时长不可用")?;
    let mut sources = HashMap::from([("video".to_string(), stream_urls(chosen))]);
    let mut representations = representation(chosen, "video")?;
    if let Some(audio) = audio {
        sources.insert("audio".to_string(), stream_urls(audio));
        representations.push_str(&representation(audio, "audio")?);
    }
    let manifest = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><MPD xmlns=\"urn:mpeg:dash:schema:mpd:2011\" type=\"static\" profiles=\"urn:mpeg:dash:profile:isoff-on-demand:2011\" mediaPresentationDuration=\"PT{duration}S\" minBufferTime=\"PT1.5S\"><Period start=\"PT0S\" duration=\"PT{duration}S\">{representations}</Period></MPD>");
    let id = uuid::Uuid::new_v4().to_string();
    let mut sessions = server.sessions.write().await;
    if sessions.len() >= 16 {
        return Err("播放器会话过多，请关闭播放器后重试".into());
    }
    sessions.insert(id.clone(), Session { manifest, sources });
    let mut qualities: Vec<PlaybackQuality> = streams
        .iter()
        .map(|stream| {
            let id = stream["id"].as_u64().unwrap_or(0);
            let label = match id {
                16 => "360P",
                32 => "480P",
                64 => "720P",
                74 => "720P 60帧",
                80 => "1080P",
                112 => "1080P 高码率",
                116 => "1080P 60帧",
                120 => "4K",
                125 => "HDR",
                126 => "杜比视界",
                127 => "8K",
                _ => "",
            };
            PlaybackQuality {
                id,
                label: if label.is_empty() {
                    format!("{}P", stream["height"].as_u64().unwrap_or(0))
                } else {
                    label.into()
                },
            }
        })
        .collect();
    qualities.sort_by_key(|quality| std::cmp::Reverse(quality.id));
    qualities.dedup_by_key(|quality| quality.id);
    Ok(PlaybackInfo {
        source: format!("{}/{id}/manifest.mpd", server.base_url),
        session_id: id,
        title: video.title,
        cid: video.cid,
        page: video.page,
        duration,
        quality: chosen["id"].as_u64().unwrap_or(0),
        qualities,
        parts: video.parts,
    })
}

fn cookies(app: &tauri::AppHandle) -> String {
    app.state::<AppState>()
        .cookies
        .lock()
        .unwrap()
        .as_ref()
        .map(|cookies| {
            cookies
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

#[tauri::command]
pub async fn start_playback(
    app: tauri::AppHandle,
    url: String,
    page: Option<u32>,
    quality: Option<u64>,
) -> Result<ApiResponse<PlaybackInfo>, String> {
    let state = app.state::<PlaybackState>();
    let server = state.server.get_or_try_init(start_proxy).await?;
    match prepare_playback(server, &url, page, quality, &cookies(&app)).await {
        Ok(data) => Ok(ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }),
        Err(error) => Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(error),
        }),
    }
}

#[tauri::command]
pub async fn release_playback(app: tauri::AppHandle, session_id: String) {
    if let Some(server) = app.state::<PlaybackState>().server.get() {
        server.sessions.write().await.remove(&session_id);
    }
}

async fn fetch_danmaku(cid: u64) -> Result<String, String> {
    if cid == 0 {
        return Err("缺少弹幕编号".into());
    }
    let response = api_client()?
        .get(format!("https://comment.bilibili.com/{cid}.xml"))
        .header("Referer", "https://www.bilibili.com/")
        .send()
        .await
        .map_err(|e| format!("弹幕请求失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("弹幕请求失败：{e}"))?;
    if response.content_length().unwrap_or(0) > 4 * 1024 * 1024 {
        return Err("弹幕数据过大，请稍后重试".into());
    }
    let raw_deflate = response
        .headers()
        .get("content-encoding")
        .and_then(|header| header.to_str().ok())
        == Some("deflate");
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let decoded = if raw_deflate {
        // Bilibili sends raw DEFLATE even when identity is requested. reqwest expects zlib framing.
        let mut decoded = Vec::new();
        let decode = flate2::read::DeflateDecoder::new(bytes.as_ref())
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut decoded);
        if decode.is_err() {
            decoded.clear();
            flate2::read::ZlibDecoder::new(bytes.as_ref())
                .take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut decoded)
                .map_err(|_| "弹幕解压失败".to_string())?;
        }
        decoded
    } else {
        bytes.to_vec()
    };
    let text = String::from_utf8(decoded).map_err(|_| "弹幕编码无效".to_string())?;
    if text.len() > 4 * 1024 * 1024 || !text.trim_start().starts_with("<?xml") {
        return Err("弹幕暂时不可用，请稍后重试".into());
    }
    Ok(text)
}

#[tauri::command]
pub async fn get_video_danmaku(cid: u64) -> Result<ApiResponse<String>, String> {
    match fetch_danmaku(cid).await {
        Ok(data) => Ok(ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }),
        Err(error) => Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(error),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restricts_proxy_destinations_and_ranges() {
        assert!(allowed_media_url(
            "https://upos-sz-mirrorcos.bilivideo.com/v.m4s"
        ));
        assert!(allowed_media_url("https://xy.mcdn.bilivideo.cn:8082/v.m4s"));
        for url in [
            "https://bilivideo.com.evil.com/v",
            "http://127.0.0.1/secret",
            "https://user:password@bilivideo.com/v",
        ] {
            assert!(!allowed_media_url(url));
        }
        assert!(valid_range("bytes=0-1023"));
        assert!(valid_range("bytes=1024-"));
        assert!(!valid_range("bytes=0-10,20-30"));
        assert!(!valid_range("bytes=-"));
        assert!(allowed_origin("http://tauri.localhost"));
        assert!(allowed_origin("http://localhost:1423"));
        assert!(!allowed_origin("https://evil.com"));
        assert_eq!(
            url_page("https://www.bilibili.com/video/BV123?p=2"),
            Some(2)
        );
    }

    #[tokio::test]
    #[ignore = "requires live Bilibili network access"]
    async fn live_playback_and_danmaku() {
        let server = start_proxy().await.unwrap();
        let info = prepare_playback(&server, "BV1xx411c7mD", None, None, "")
            .await
            .unwrap();
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let manifest = client
            .get(&info.source)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(manifest.contains("<MPD"));
        assert!(manifest.contains("audio/mp4"));
        for stream in ["video", "audio"] {
            let url = info.source.replace("manifest.mpd", stream);
            let response = client
                .get(&url)
                .header("Range", "bytes=0-1023")
                .header("Origin", "http://localhost:1423")
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 206);
            assert_eq!(
                response.headers()["access-control-allow-origin"],
                "http://localhost:1423"
            );
            let bytes = response.bytes().await.unwrap();
            assert_eq!(bytes.len(), 1024);
            assert!(bytes.windows(4).any(|bytes| bytes == b"ftyp"));
        }
        let response = client
            .get(&info.source)
            .header("Origin", "https://evil.com")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
        let preflight = client
            .request(reqwest::Method::OPTIONS, &info.source)
            .header("Origin", "http://tauri.localhost")
            .header("Access-Control-Request-Headers", "range")
            .send()
            .await
            .unwrap();
        assert_eq!(preflight.status(), 204);
        assert_eq!(
            preflight.headers()["access-control-allow-origin"],
            "http://tauri.localhost"
        );
        let second = prepare_playback(
            &server,
            "https://www.bilibili.com/video/BV17x411w7KC?p=2",
            None,
            Some(16),
            "",
        )
        .await
        .unwrap();
        assert_eq!(second.page, 2);
        assert_eq!(second.parts.len(), 10);
        assert_eq!(second.quality, 16);
        assert!(
            prepare_playback(&server, "BV17x411w7KC", Some(99), None, "")
                .await
                .is_err()
        );
        let xml = fetch_danmaku(info.cid).await.unwrap();
        assert!(xml.contains("<d p="));
        println!(
            "Live playback: {}s, quality {}, {} parts, {} danmaku XML bytes",
            info.duration,
            info.quality,
            info.parts.len(),
            xml.len()
        );
        server.sessions.write().await.remove(&info.session_id);
        assert_eq!(client.get(&info.source).send().await.unwrap().status(), 404);
    }
}
