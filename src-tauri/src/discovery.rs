//! Bilibili discovery endpoints, kept separate from download commands.
use crate::{ApiResponse, AppState};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tauri::Manager;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Serialize)]
pub struct HotSearchItem {
    rank: usize,
    keyword: String,
    title: String,
    heat: u64,
}

#[derive(Debug, Serialize)]
pub struct RankingItem {
    rank: usize,
    bvid: String,
    title: String,
    cover: Option<String>,
    duration: String,
    author: String,
    play: u64,
    danmaku: u64,
}

#[derive(Debug, Serialize)]
pub struct RankingResult {
    items: Vec<RankingItem>,
    note: String,
}

fn client(cookies: &str) -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    if !cookies.is_empty() {
        headers.insert(
            reqwest::header::COOKIE,
            cookies
                .parse()
                .map_err(|_| "登录信息无效，请重新登录".to_string())?,
        );
    }
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("无法建立连接：{e}"))
}

async fn request(
    client: &reqwest::Client,
    path: &str,
    query: &[(&str, String)],
) -> Result<Value, String> {
    let response = client
        .get(format!("https://api.bilibili.com{path}"))
        .query(query)
        .header("Referer", "https://www.bilibili.com/")
        .send()
        .await
        .map_err(|e| format!("连接 B 站失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("B 站请求失败：{e}"))?;
    let json: Value = response
        .json()
        .await
        .map_err(|_| "B 站返回了无法读取的数据，请稍后重试".to_string())?;
    match json["code"].as_i64() {
        Some(0) => Ok(json["data"].clone()),
        Some(-352 | -412 | -403) => Err("B 站暂时限制了此请求，请稍后重试或登录后再试".to_string()),
        _ => Err(format!(
            "B 站请求失败：{}",
            json["message"].as_str().unwrap_or("未知错误")
        )),
    }
}

fn parse_hot_search(data: &Value) -> Result<Vec<HotSearchItem>, String> {
    let list = data["trending"]["list"]
        .as_array()
        .ok_or("热搜数据格式已变化，请稍后重试")?;
    Ok(list
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let keyword = item["keyword"].as_str().filter(|s| !s.trim().is_empty())?;
            Some(HotSearchItem {
                rank: index + 1,
                keyword: keyword.to_string(),
                title: item["show_name"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(keyword)
                    .to_string(),
                heat: item["heat_score"].as_u64().unwrap_or(0),
            })
        })
        .collect())
}

fn parse_ranking(data: &Value) -> Result<RankingResult, String> {
    let list = data["list"]
        .as_array()
        .ok_or("榜单数据格式已变化，请稍后重试")?;
    let items = list
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let bvid = item["bvid"].as_str().filter(|s| !s.is_empty())?;
            let cover = item["pic"].as_str().filter(|s| !s.is_empty()).map(|s| {
                if s.starts_with("//") {
                    format!("https:{s}")
                } else {
                    s.replacen("http://", "https://", 1)
                }
            });
            let duration = match item["duration"].as_u64() {
                Some(seconds) if seconds >= 3600 => format!(
                    "{}:{:02}:{:02}",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60
                ),
                Some(seconds) => format!("{}:{:02}", seconds / 60, seconds % 60),
                None => item["duration"].as_str().unwrap_or("").to_string(),
            };
            Some(RankingItem {
                rank: index + 1,
                bvid: bvid.to_string(),
                title: item["title"].as_str().unwrap_or("未命名视频").to_string(),
                cover,
                duration,
                author: item["owner"]["name"]
                    .as_str()
                    .or_else(|| item["author"].as_str())
                    .unwrap_or("")
                    .to_string(),
                play: item["stat"]["view"]
                    .as_u64()
                    .or_else(|| item["play"].as_u64())
                    .unwrap_or(0),
                danmaku: item["stat"]["danmaku"]
                    .as_u64()
                    .or_else(|| item["video_review"].as_u64())
                    .unwrap_or(0),
            })
        })
        .collect();
    Ok(RankingResult {
        items,
        note: data["note"]
            .as_str()
            .unwrap_or("根据稿件内容质量、近期的数据综合展示，动态更新")
            .to_string(),
    })
}

fn cookie_header(app: &tauri::AppHandle) -> String {
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

fn response<T>(result: Result<T, String>) -> ApiResponse<T> {
    match result {
        Ok(data) => ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        },
        Err(error) => ApiResponse {
            success: false,
            data: None,
            error: Some(error),
        },
    }
}

#[tauri::command]
pub async fn get_hot_search(
    app: tauri::AppHandle,
) -> Result<ApiResponse<Vec<HotSearchItem>>, String> {
    let result = async {
        let client = client(&cookie_header(&app))?;
        let data = request(
            &client,
            "/x/web-interface/wbi/search/square",
            &[("limit", "30".into())],
        )
        .await?;
        parse_hot_search(&data)
    }
    .await;
    Ok(response(result))
}

async fn load_ranking(client: &reqwest::Client, rid: u32) -> Result<RankingResult, String> {
    let query = [("rid", rid.to_string()), ("type", "all".into())];
    let data = match request(client, "/x/web-interface/ranking/v2", &query).await {
        Ok(data) => data,
        Err(_) => {
            request(
                client,
                "/x/web-interface/ranking",
                &[
                    ("rid", rid.to_string()),
                    ("day", "3".into()),
                    ("type", "1".into()),
                    ("arc_type", "0".into()),
                ],
            )
            .await?
        }
    };
    parse_ranking(&data)
}

#[tauri::command]
pub async fn get_ranking(
    app: tauri::AppHandle,
    rid: u32,
) -> Result<ApiResponse<RankingResult>, String> {
    if ![0, 1, 3, 4, 36, 188, 160, 211, 181].contains(&rid) {
        return Ok(response(Err("不支持的榜单分区".to_string())));
    }
    let result = async { load_ranking(&client(&cookie_header(&app))?, rid).await }.await;
    Ok(response(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use serde_json::json;

    #[test]
    fn hot_search_keeps_search_keyword_and_original_rank() {
        let items = parse_hot_search(&json!({"trending":{"list":[
            {"keyword":"", "show_name":"广告"},
            {"keyword":"实际搜索词", "show_name":"展示标题", "heat_score":12345}
        ]}}))
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].rank, 2);
        assert_eq!(items[0].keyword, "实际搜索词");
        assert_eq!(items[0].title, "展示标题");
        assert_eq!(items[0].heat, 12345);
    }

    #[test]
    fn ranking_supports_both_api_shapes_without_losing_order() {
        let result = parse_ranking(&json!({"note":"榜单说明", "list":[
            {"title":"无效项"},
            {"bvid":"BV123", "title":"新版", "pic":"//i0.hdslb.com/a.jpg", "duration":3661, "owner":{"name":"UP"}, "stat":{"view":123,"danmaku":4}},
            {"bvid":"BV456", "title":"兼容版", "pic":"http://i0.hdslb.com/b.jpg", "duration":"12:34", "author":"作者", "play":456,"video_review":7}
        ]})).unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].rank, 2);
        assert_eq!(result.items[0].duration, "1:01:01");
        assert_eq!(
            result.items[0].cover.as_deref(),
            Some("https://i0.hdslb.com/a.jpg")
        );
        assert_eq!(result.items[0].author, "UP");
        assert_eq!(result.items[1].play, 456);
        assert_eq!(result.items[1].danmaku, 7);
        assert_eq!(result.items[1].duration, "12:34");
        assert_eq!(result.note, "榜单说明");
        assert!(parse_ranking(&json!({})).is_err());
        assert!(parse_hot_search(&json!({})).is_err());
    }

    #[tokio::test]
    #[ignore = "requires live Bilibili network access"]
    async fn live_discovery_endpoints() {
        let client = client("").unwrap();
        let data = request(
            &client,
            "/x/web-interface/wbi/search/square",
            &[("limit", "30".into())],
        )
        .await
        .unwrap();
        let hot = parse_hot_search(&data).unwrap();
        assert!(!hot.is_empty());
        println!("Live hot searches: {}", hot.len());
        for rid in [0, 1, 3, 4, 36, 188, 160, 211, 181] {
            let ranking = load_ranking(&client, rid).await.unwrap();
            assert!(!ranking.items.is_empty(), "empty ranking for {rid}");
            println!("Live ranking {rid}: {} videos", ranking.items.len());
            if rid == 0 {
                let cover = ranking.items[0].cover.clone().expect("ranking cover");
                let encoded = crate::fetch_image_as_base64(cover).await.unwrap();
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded.split_once(',').unwrap().1)
                    .unwrap();
                let image = image::load_from_memory(&bytes).expect("valid proxied cover image");
                println!("Live proxied cover: {} x {}", image.width(), image.height());
            }
        }
    }
}
