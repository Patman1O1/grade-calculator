//! JSON HTTP API used by the React front-end.
//!
//! | Method | Path                    | Body                          | Response                                  |
//! |--------|-------------------------|-------------------------------|-------------------------------------------|
//! | GET    | `/api/health`           |                               | `{ "status": "ok" }`                      |
//! | GET    | `/api/examples`         |                               | `[{ key, label }]`                        |
//! | GET    | `/api/examples/{key}`   |                               | `Course` (fresh ids)                      |
//! | GET    | `/api/courses/blank`    |                               | `Course` (fresh ids)                      |
//! | POST   | `/api/evaluate`         | `{ course, targets? }`        | `{ course, remaining, needs }`            |
//! | POST   | `/api/summaries`        | `{ courses }`                 | `[{ id, percent, letter }]`               |
//! | POST   | `/api/import`           | `Course` or `[Course]`        | `{ courses }` (validated, fresh ids)      |

use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::examples::{EXAMPLES, blank_course, example, re_id};
use crate::{Course, Needed};

/// Most targets accepted by one `/api/evaluate` call (each runs a 60-step search).
pub const MAX_TARGETS: usize = 32;
/// Most courses accepted by one `/api/summaries` or `/api/import` call.
pub const MAX_COURSES: usize = 200;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/examples", get(list_examples))
        .route("/examples/{key}", get(get_example))
        .route("/courses/blank", get(get_blank))
        .route("/evaluate", post(evaluate))
        .route("/summaries", post(summaries))
        .route("/import", post(import))
        .fallback(|| async { ApiError(StatusCode::NOT_FOUND, "no such API route".into()) })
}

/// An error response: `{ "error": "..." }` with a status code.
#[derive(Debug)]
pub struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        ApiError(rejection.status(), rejection.body_text())
    }
}

/// `axum::Json`, but malformed bodies produce our JSON error shape.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// Grading is CPU-bound; keep it off the async worker threads.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(f).await.map_err(|_| {
        ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "calculation failed".into(),
        )
    })
}

async fn health() -> Json<Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[derive(Serialize)]
struct ExampleInfo {
    key: &'static str,
    label: &'static str,
}

async fn list_examples() -> Json<Vec<ExampleInfo>> {
    Json(
        EXAMPLES
            .iter()
            .map(|e| ExampleInfo {
                key: e.key,
                label: e.label,
            })
            .collect(),
    )
}

async fn get_example(Path(key): Path<String>) -> Result<Json<Course>, ApiError> {
    example(&key)
        .map(Json)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no example named {key:?}")))
}

async fn get_blank() -> Json<Course> {
    Json(blank_course())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateRequest {
    pub course: Course,
    /// Target percentages to solve "what do I need?" for.
    #[serde(default)]
    pub targets: Vec<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetNeed {
    pub target: f64,
    pub needed: Needed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponse {
    /// The course with its computed fields (grade, category and item results) filled in.
    pub course: Course,
    /// Ungraded items that still have points available.
    pub remaining: usize,
    /// One entry per requested target, in the same order.
    pub needs: Vec<TargetNeed>,
}

pub fn evaluate_course(req: EvaluateRequest) -> EvaluateResponse {
    let course = req.course.graded();
    let needs = req
        .targets
        .iter()
        .map(|&target| TargetNeed {
            target,
            needed: req.course.needed_on_remaining(target),
        })
        .collect();
    EvaluateResponse {
        course,
        remaining: req.course.remaining_count(),
        needs,
    }
}

async fn evaluate(Json(req): Json<EvaluateRequest>) -> Result<Json<EvaluateResponse>, ApiError> {
    if req.targets.len() > MAX_TARGETS {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("at most {MAX_TARGETS} targets"),
        ));
    }
    Ok(Json(blocking(move || evaluate_course(req)).await?))
}

#[derive(Deserialize)]
pub struct SummariesRequest {
    pub courses: Vec<Course>,
}

#[derive(Debug, Serialize)]
pub struct CourseSummary {
    pub id: String,
    pub percent: Option<f64>,
    pub letter: Option<String>,
}

async fn summaries(
    Json(req): Json<SummariesRequest>,
) -> Result<Json<Vec<CourseSummary>>, ApiError> {
    if req.courses.len() > MAX_COURSES {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("at most {MAX_COURSES} courses"),
        ));
    }
    let out = blocking(move || {
        req.courses
            .into_iter()
            .map(|mut c| {
                c.grade();
                CourseSummary {
                    id: c.id,
                    percent: c.percent,
                    letter: c.letter,
                }
            })
            .collect()
    })
    .await?;
    Ok(Json(out))
}

#[derive(Serialize)]
struct ImportResponse {
    courses: Vec<Course>,
}

/// Validate a JSON export (one course or a list of them) and give every
/// course fresh ids so it can be added next to existing ones. Also used to
/// duplicate a course.
async fn import(Json(data): Json<Value>) -> Result<Json<ImportResponse>, ApiError> {
    let list = match data {
        Value::Array(list) => list,
        other => vec![other],
    };
    if list.len() > MAX_COURSES {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("at most {MAX_COURSES} courses"),
        ));
    }
    let courses: Vec<Course> = list
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Course>(v).ok())
        .map(re_id)
        .collect();
    if courses.is_empty() {
        return Err(ApiError(
            StatusCode::UNPROCESSABLE_ENTITY,
            "That file is not a course exported from this calculator.".into(),
        ));
    }
    Ok(Json(ImportResponse { courses }))
}
