use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

/// This service's identity. `srvcs-mode` is a leaf: it depends on no other
/// service. It computes the statistical mode of a list of integers entirely
/// with local logic.
pub const SERVICE: &str = "srvcs-mode";
pub const CONCERN: &str = "comparison: most frequent value";
pub const DEPENDS_ON: &[&str] = &[];

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    /// The list of integers to reduce. Every element must be a JSON integer
    /// (i64). The list must be non-empty.
    #[schema(value_type = Object)]
    pub values: Vec<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct ModeResponse {
    #[schema(value_type = Object)]
    pub values: Vec<Value>,
    pub result: i64,
}

/// The single concern: the most frequent integer in `values`.
///
/// Returns `None` if the list is empty or any element is not a JSON integer.
/// Otherwise returns `Some(mode)` where `mode` is the most frequent element; on
/// a tie the smallest of the most-frequent elements is returned.
pub fn mode(values: &[Value]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    // Parse every element as an i64 first; any non-integer rejects the request.
    let mut ints: Vec<i64> = Vec::with_capacity(values.len());
    for v in values {
        match v.as_i64() {
            Some(n) => ints.push(n),
            None => return None,
        }
    }
    // Count occurrences of each value.
    let mut counts: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for &n in &ints {
        *counts.entry(n).or_insert(0) += 1;
    }
    // Pick the value with the highest count; break ties by the smallest value.
    counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(value, _count)| value)
}

/// `POST /` — the most frequent integer in the list.
///
/// Reads each element as a JSON integer (`i64`). If the list is empty or any
/// element is not an integer the request is rejected with `422`. Otherwise the
/// most frequent element is returned; on a tie the smallest of the most-frequent
/// elements wins.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = ModeResponse),
        (status = 422, description = "the list is empty or an element is not a valid integer")
    )
)]
pub async fn evaluate(Json(req): Json<EvalRequest>) -> Response {
    match mode(&req.values) {
        Some(result) => (
            StatusCode::OK,
            Json(json!({ "values": req.values, "result": result })),
        )
            .into_response(),
        None => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "values must be a non-empty list of integers" })),
        )
            .into_response(),
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, ModeResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some(), "GET / documented");
        assert!(root.post.is_some(), "POST / documented");
    }

    #[test]
    fn index_reports_identity() {
        // Identity constants are the public contract of this leaf service.
        assert_eq!(SERVICE, "srvcs-mode");
        assert_eq!(CONCERN, "comparison: most frequent value");
        assert!(DEPENDS_ON.is_empty());
    }

    #[test]
    fn mode_of_clear_winner() {
        assert_eq!(mode(&[json!(1), json!(2), json!(2), json!(3)]), Some(2));
        assert_eq!(mode(&[json!(5)]), Some(5));
        assert_eq!(mode(&[json!(7), json!(7), json!(7)]), Some(7));
    }

    #[test]
    fn mode_breaks_ties_with_smallest() {
        assert_eq!(mode(&[json!(4), json!(4), json!(5), json!(5)]), Some(4));
        assert_eq!(mode(&[json!(3), json!(1), json!(1), json!(3)]), Some(1));
        // Every element unique -> all tie at count 1 -> smallest wins.
        assert_eq!(mode(&[json!(9), json!(2), json!(5)]), Some(2));
        // Negative numbers compare correctly.
        assert_eq!(mode(&[json!(-1), json!(-1), json!(2), json!(2)]), Some(-1));
    }

    #[test]
    fn empty_list_is_rejected() {
        assert_eq!(mode(&[]), None);
    }

    #[test]
    fn non_integer_element_is_rejected() {
        for bad in [
            json!("2"),
            json!(2.5),
            json!(true),
            json!(null),
            json!([2]),
            json!({ "v": 2 }),
        ] {
            assert_eq!(
                mode(&[json!(1), bad.clone()]),
                None,
                "{bad} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn evaluate_returns_200_with_result() {
        let resp = evaluate(Json(EvalRequest {
            values: vec![json!(1), json!(2), json!(2), json!(3)],
        }))
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn evaluate_returns_422_for_non_integer() {
        let resp = evaluate(Json(EvalRequest {
            values: vec![json!(1), json!("nope")],
        }))
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn evaluate_returns_422_for_empty() {
        let resp = evaluate(Json(EvalRequest { values: vec![] })).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn index_reports_identity_over_http() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-mode");
        assert!(info.depends_on.is_empty());
    }
}
