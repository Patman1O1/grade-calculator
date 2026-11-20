//! HTTP-level tests of the JSON API.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn call(method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let req = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = grade_calculator::app(None).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

#[tokio::test]
async fn health() {
    assert_eq!(
        call("GET", "/api/health", None).await,
        (StatusCode::OK, json!({ "status": "ok" }))
    );
}

#[tokio::test]
async fn lists_and_builds_examples() {
    let (status, list) = call("GET", "/api/examples", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 5);
    assert_eq!(list[0]["key"], "cs401");

    let (status, course) = call("GET", "/api/examples/cs411", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(course["scheme"], "weighted");
    assert_eq!(course["categories"][0]["dropLowest"], 2.0);
    assert!(course["id"].as_str().unwrap().starts_with("course-"));
    // Ungraded courses carry no computed fields.
    assert!(course.get("percent").is_none());
    assert!(course["categories"][0].get("earned").is_none());
    assert!(course["categories"][0]["items"][0].get("counted").is_none());

    let (status, err) = call("GET", "/api/examples/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(err["error"].is_string());

    let (status, err) = call("GET", "/api/typo", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(err["error"], "no such API route");
}

#[tokio::test]
async fn evaluates_a_course_and_solves_targets() {
    let course = json!({
        "id": "c", "name": "Test", "scheme": "points",
        "scale": [{ "letter": "A", "min": 90 }, { "letter": "F", "min": 0 }],
        "categories": [{
            "id": "k", "name": "All", "weight": 0,
            "items": [
                { "id": "a", "name": "A", "score": 60, "outOf": 100 },
                { "id": "b", "name": "B", "score": null, "outOf": 100 }
            ]
        }]
    });
    let (status, out) = call(
        "POST",
        "/api/evaluate",
        Some(json!({ "course": course, "targets": [80, 95, 30] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{out}");
    let graded = &out["course"];
    assert_eq!(graded["name"], "Test");
    assert_eq!(graded["percent"], 60.0);
    assert_eq!(graded["letter"], "F");
    assert_eq!(graded["earned"], 60.0);
    assert_eq!(graded["categories"][0]["percent"], 60.0);
    let items = &graded["categories"][0]["items"];
    assert_eq!(items[0]["counted"], true);
    assert_eq!(items[0]["earned"], 60.0);
    assert_eq!(items[1]["counted"], false);
    assert_eq!(items[1]["score"], Value::Null);
    assert_eq!(out["remaining"], 1);
    assert_eq!(out["needs"][0]["target"], 80.0);
    assert_eq!(out["needs"][0]["needed"]["kind"], "needs");
    assert!((out["needs"][0]["needed"]["fraction"].as_f64().unwrap() - 1.0).abs() < 1e-6);
    assert_eq!(out["needs"][1]["needed"]["kind"], "impossible");
    assert_eq!(out["needs"][2]["needed"], json!({ "kind": "secured" }));
}

#[tokio::test]
async fn summarises_every_course() {
    let (_, c1) = call("GET", "/api/examples/cs401", None).await;
    let (_, c2) = call("GET", "/api/courses/blank", None).await;
    let (status, out) = call(
        "POST",
        "/api/summaries",
        Some(json!({ "courses": [c1, c2] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(out[0]["id"], c1["id"]);
    assert!((out[0]["percent"].as_f64().unwrap() - 56.95).abs() < 0.005);
    assert_eq!(out[1]["percent"], Value::Null);
}

#[tokio::test]
async fn import_validates_and_reassigns_ids() {
    let (_, course) = call("GET", "/api/examples/math218", None).await;
    let (status, out) = call(
        "POST",
        "/api/import",
        Some(json!([course, { "not": "a course" }])),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let imported = &out["courses"];
    assert_eq!(imported.as_array().unwrap().len(), 1);
    assert_eq!(imported[0]["name"], course["name"]);
    assert_ne!(imported[0]["id"], course["id"]);

    // Exports from the old TypeScript-only app have the same shape (ids optional).
    let legacy = json!({ "name": "Old", "scheme": "points", "scale": [], "categories": [
        { "name": "C", "weight": 0, "items": [{ "name": "I", "score": 1, "outOf": 2, "neverDrop": true }] }
    ]});
    let (status, out) = call("POST", "/api/import", Some(legacy)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        out["courses"][0]["categories"][0]["items"][0]["neverDrop"],
        true
    );

    // Computed fields sent by a client are ignored, not trusted.
    let forged = json!({ "name": "F", "scheme": "points", "scale": [], "percent": 100, "letter": "A",
        "categories": [{ "name": "C", "weight": 0, "earned": 99,
            "items": [{ "name": "I", "score": 0, "outOf": 10, "earned": 10, "counted": true }] }] });
    let (_, out) = call("POST", "/api/import", Some(forged.clone())).await;
    assert!(out["courses"][0].get("percent").is_none());
    assert!(
        out["courses"][0]["categories"][0]["items"][0]
            .get("earned")
            .is_none()
    );
    let (_, out) = call("POST", "/api/evaluate", Some(json!({ "course": forged }))).await;
    assert_eq!(out["course"]["percent"], 0.0);
    assert_eq!(out["course"]["letter"], Value::Null);
    assert_eq!(out["course"]["categories"][0]["items"][0]["earned"], 0.0);

    let (status, err) = call("POST", "/api/import", Some(json!({ "hello": 1 }))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(err["error"].as_str().unwrap().contains("not a course"));
}

#[tokio::test]
async fn malformed_bodies_get_json_errors() {
    let (status, err) = call("POST", "/api/evaluate", Some(json!({ "course": 5 }))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(err["error"].is_string());
}
