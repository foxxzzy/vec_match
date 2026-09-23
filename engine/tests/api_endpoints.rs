use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, Response, StatusCode},
    routing::get,
};
use tower::ServiceExt;

use matching_engine_demo::{api, db};

// ── Helper: get a real DB pool ──────────────────────────────────

async fn pool() -> sqlx::PgPool {
    db::init_pool()
        .await
        .expect("DATABASE_URL must be set and reachable")
}

fn write_tests_enabled() -> bool {
    std::env::var("RUN_DB_WRITE_TESTS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

macro_rules! app {
    ($pool:expr) => {
        api::router($pool.clone())
    };
}

async fn call(app: &Router, req: Request<Body>) -> Response<Body> {
    app.clone().oneshot(req).await.expect("request failed")
}

async fn read_body(resp: Response<Body>) -> axum::body::Bytes {
    to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read response body")
}

fn get_req(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("build GET request")
}

fn post_json_req(uri: &str, payload: serde_json::Value) -> Request<Body> {
    json_req("POST", uri, payload)
}

fn patch_json_req(uri: &str, payload: serde_json::Value) -> Request<Body> {
    json_req("PATCH", uri, payload)
}

fn json_req(method: &str, uri: &str, payload: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .expect("build JSON request")
}

// ═══════════════════════════════════════════════════════════════
// Health
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn health_ok() {
    let app = Router::new().route("/health", get(api::health::health));
    let req = get_req("/health");
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn db_health_ok() {
    let pool = pool().await;
    let app = app!(pool);
    let req = get_req("/db/health");
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ═══════════════════════════════════════════════════════════════
// Content export (GET, reads existing seeded content)
// ═══════════════════════════════════════════════════════════════


#[tokio::test]
async fn export_content_bad_axis_returns_400() {
    let pool = pool().await;
    let app = app!(pool);

    let req = get_req("/content/export?axis=NotARealAxis");
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ═══════════════════════════════════════════════════════════════
// Anchor types export (GET, reads existing computed anchors)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn export_anchor_types_for_axis_ok() {
    let pool = pool().await;
    let app = app!(pool);

    let req = get_req("/anchor_types/export?axis=ComfortWithCloseness");
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = read_body(resp).await;
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("Expected JSON array from /anchor_types/export");
    assert!(json.is_array(), "expected JSON array");
}

#[tokio::test]
async fn export_anchor_types_bad_axis_returns_400() {
    let pool = pool().await;
    let app = app!(pool);

    let req = get_req("/anchor_types/export?axis=Nonsense");
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ═══════════════════════════════════════════════════════════════
// User interaction (uses content already in the DB)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn user_interaction_with_existing_content_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    // grab an existing prompt_id from embedded_content
    let row = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT prompt_id FROM embedded_content_who_are_they LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .expect("query embedded_content");

    let prompt_id = match row {
        Some(id) => id,
        None => {
            eprintln!("[TEST] skipping user_interaction test – no seeded content");
            return;
        }
    };

    let user_id = uuid::Uuid::new_v4();
    let payload = serde_json::json!({
        "user_id": user_id.to_string(),
        "id": prompt_id.to_string(),
        "reaction": "Me"
    });

    let req = post_json_req("/user/interaction", payload);
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ═══════════════════════════════════════════════════════════════
// Compute final axis vector (uses content already in the DB)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn compute_final_axis_vector_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    // grab up to 10 existing prompt_ids for ComfortWithCloseness
    let prompt_ids = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT prompt_id FROM embedded_content_who_are_they WHERE axis = 'ComfortWithCloseness' LIMIT 10",
    )
    .fetch_all(&pool)
    .await
    .expect("query embedded_content for ComfortWithCloseness");

    if prompt_ids.is_empty() {
        eprintln!(
            "[TEST] skipping compute_final_axis_vector – no seeded ComfortWithCloseness content"
        );
        return;
    }

    let user_id = uuid::Uuid::new_v4();

    // seed deterministic interactions for this user using existing content
    for (i, pid) in prompt_ids.iter().enumerate() {
        let reaction = if i % 2 == 0 { "Me" } else { "NotMe" };
        let payload = serde_json::json!({
            "user_id": user_id.to_string(),
            "id": pid.to_string(),
            "reaction": reaction
        });
        let req = post_json_req("/user/interaction", payload);
        let resp = call(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "interaction for prompt {} failed",
            pid
        );
    }

    // compute the axis vector
    let compute_payload = serde_json::json!(user_id.to_string());
    let req = post_json_req("/vectors/compute-axis-vector", compute_payload);
    let resp = call(&app, req).await;
    let status = resp.status();
    let body = read_body(resp).await;
    let body_str = String::from_utf8_lossy(&body);

    assert_eq!(
        status,
        StatusCode::OK,
        "expected 200 OK, got {} with body: {}",
        status,
        body_str
    );

    // verify a user_info_vectors row was created
    let rows = sqlx::query("SELECT * FROM user_info_vectors WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(&pool)
        .await
        .expect("query user_info_vectors");

    assert!(
        !rows.is_empty(),
        "expected at least one user_info_vectors row for user {}, found 0",
        user_id
    );
}

// ═══════════════════════════════════════════════════════════════
// Users CRUD
// ═══════════════════════════════════════════════════════════════

async fn available_auth_user_id(pool: &sqlx::PgPool) -> Option<uuid::Uuid> {
    sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        SELECT au.id
        FROM auth.users au
        LEFT JOIN public.users u ON u.id = au.id
        WHERE u.id IS NULL
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

async fn first_gender_code(pool: &sqlx::PgPool) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT code FROM genders LIMIT 1")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

fn create_user_payload(
    user_id: uuid::Uuid,
    display_name: &str,
    gender_code: &str,
) -> serde_json::Value {
    serde_json::json!({
        "id": user_id.to_string(),
        "display_name": display_name,
        "birth_date": "1995-06-15",
        "location_text": "London",
        "profile": {
            "date_of_birth": "1995-06-15",
            "gender_code": gender_code
        },
        "preferred_genders": [gender_code]
    })
}

#[tokio::test]
async fn create_user_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    let user_id = match available_auth_user_id(&pool).await {
        Some(id) => id,
        None => {
            eprintln!("[TEST] skipping create_user_ok – no unused auth.users id");
            return;
        }
    };
    let gender_code = match first_gender_code(&pool).await {
        Some(code) => code,
        None => {
            eprintln!("[TEST] skipping create_user_ok – no rows in genders");
            return;
        }
    };
    let payload = create_user_payload(user_id, "Test User", &gender_code);

    let req = post_json_req("/users", payload);
    let resp = call(&app, req).await;
    let status = resp.status();
    let body = read_body(resp).await;

    assert_eq!(
        status,
        StatusCode::CREATED,
        "expected 201 Created, got {} with body: {}",
        status,
        String::from_utf8_lossy(&body)
    );

    let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
    assert!(
        json.get("user").is_some(),
        "response should contain user key"
    );
}

#[tokio::test]
async fn get_user_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    let user_id = match available_auth_user_id(&pool).await {
        Some(id) => id,
        None => {
            eprintln!("[TEST] skipping get_user_ok – no unused auth.users id");
            return;
        }
    };
    let gender_code = match first_gender_code(&pool).await {
        Some(code) => code,
        None => {
            eprintln!("[TEST] skipping get_user_ok – no rows in genders");
            return;
        }
    };
    let create_payload = create_user_payload(user_id, "Fetch Me", &gender_code);

    let create_req = post_json_req("/users", create_payload);
    let create_resp = call(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    // now GET the user
    let req = get_req(&format!("/users/{}", user_id));
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_user_not_found() {
    let pool = pool().await;
    let app = app!(pool);

    let fake_id = uuid::Uuid::new_v4();
    let req = get_req(&format!("/users/{}", fake_id));
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_user_profile_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    let user_id = match available_auth_user_id(&pool).await {
        Some(id) => id,
        None => {
            eprintln!("[TEST] skipping get_user_profile_ok – no unused auth.users id");
            return;
        }
    };
    let gender_code = match first_gender_code(&pool).await {
        Some(code) => code,
        None => {
            eprintln!("[TEST] skipping get_user_profile_ok – no rows in genders");
            return;
        }
    };
    let create_payload = create_user_payload(user_id, "Profile Test", &gender_code);

    let create_req = post_json_req("/users", create_payload);
    let create_resp = call(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let req = get_req(&format!("/users/{}/profile", user_id));
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_user_preferences_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    let user_id = match available_auth_user_id(&pool).await {
        Some(id) => id,
        None => {
            eprintln!("[TEST] skipping get_user_preferences_ok – no unused auth.users id");
            return;
        }
    };
    let gender_code = match first_gender_code(&pool).await {
        Some(code) => code,
        None => {
            eprintln!("[TEST] skipping get_user_preferences_ok – no rows in genders");
            return;
        }
    };
    let create_payload = create_user_payload(user_id, "Prefs Test", &gender_code);

    let create_req = post_json_req("/users", create_payload);
    let create_resp = call(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let req = get_req(&format!("/users/{}/preferences", user_id));
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_user_computed_ok_for_new_user() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    let user_id = match available_auth_user_id(&pool).await {
        Some(id) => id,
        None => {
            eprintln!(
                "[TEST] skipping get_user_computed_ok_for_new_user – no unused auth.users id"
            );
            return;
        }
    };
    let gender_code = match first_gender_code(&pool).await {
        Some(code) => code,
        None => {
            eprintln!("[TEST] skipping get_user_computed_ok_for_new_user – no rows in genders");
            return;
        }
    };
    let create_payload = create_user_payload(user_id, "Computed Test", &gender_code);

    let create_req = post_json_req("/users", create_payload);
    let create_resp = call(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    // computed row is created during user creation
    let req = get_req(&format!("/users/{}/computed", user_id));
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn update_user_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    let user_id = match available_auth_user_id(&pool).await {
        Some(id) => id,
        None => {
            eprintln!("[TEST] skipping update_user_ok – no unused auth.users id");
            return;
        }
    };
    let gender_code = match first_gender_code(&pool).await {
        Some(code) => code,
        None => {
            eprintln!("[TEST] skipping update_user_ok – no rows in genders");
            return;
        }
    };
    let create_payload = create_user_payload(user_id, "Before Update", &gender_code);

    let create_req = post_json_req("/users", create_payload);
    let create_resp = call(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let update_payload = serde_json::json!({
        "display_name": "After Update",
        "location_text": "Manchester"
    });
    let req = patch_json_req(&format!("/users/{}", user_id), update_payload);
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
    assert_eq!(json["display_name"], "After Update");
}

#[tokio::test]
async fn update_profile_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    let user_id = match available_auth_user_id(&pool).await {
        Some(id) => id,
        None => {
            eprintln!("[TEST] skipping update_profile_ok – no unused auth.users id");
            return;
        }
    };
    let gender_code = match first_gender_code(&pool).await {
        Some(code) => code,
        None => {
            eprintln!("[TEST] skipping update_profile_ok – no rows in genders");
            return;
        }
    };
    let create_payload = create_user_payload(user_id, "Profile Update Test", &gender_code);

    let create_req = post_json_req("/users", create_payload);
    let create_resp = call(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let update_payload = serde_json::json!({
        "bio": "I like hiking",
        "city": "Manchester"
    });
    let req = patch_json_req(&format!("/users/{}/profile", user_id), update_payload);
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
    assert_eq!(json["bio"], "I like hiking");
}

#[tokio::test]
async fn update_preferences_ok() {
    if !write_tests_enabled() {
        eprintln!("[TEST] skipping mutating test; set RUN_DB_WRITE_TESTS=1 to enable");
        return;
    }

    let pool = pool().await;
    let app = app!(pool);

    let user_id = match available_auth_user_id(&pool).await {
        Some(id) => id,
        None => {
            eprintln!("[TEST] skipping update_preferences_ok – no unused auth.users id");
            return;
        }
    };
    let gender_code = match first_gender_code(&pool).await {
        Some(code) => code,
        None => {
            eprintln!("[TEST] skipping update_preferences_ok – no rows in genders");
            return;
        }
    };
    let create_payload = create_user_payload(user_id, "Prefs Update Test", &gender_code);

    let create_req = post_json_req("/users", create_payload);
    let create_resp = call(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let update_payload = serde_json::json!({
        "max_distance_km": 25,
        "exclude_smokers": true
    });
    let req = patch_json_req(&format!("/users/{}/preferences", user_id), update_payload);
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
    assert_eq!(json["max_distance_km"], 25);
    assert_eq!(json["exclude_smokers"], true);
}

#[tokio::test]
async fn update_user_not_found() {
    let pool = pool().await;
    let app = app!(pool);

    let fake_id = uuid::Uuid::new_v4();
    let update_payload = serde_json::json!({
        "display_name": "Ghost"
    });
    let req = patch_json_req(&format!("/users/{}", fake_id), update_payload);
    let resp = call(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
