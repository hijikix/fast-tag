use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use uuid::Uuid;
use crate::auth::User;

/// Sets up a test database with clean schema and migrations
pub async fn setup_test_db() -> Pool<Postgres> {
    // Load test environment
    dotenvy::from_filename("api/.env.test").ok();
    
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env.test");
    
    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");
    
    // Drop all tables and recreate schema
    sqlx::query("DROP SCHEMA IF EXISTS public CASCADE")
        .execute(&pool)
        .await
        .ok();
    
    sqlx::query("CREATE SCHEMA IF NOT EXISTS public")
        .execute(&pool)
        .await
        .ok();
    
    // Grant permissions on the schema
    sqlx::query("GRANT ALL ON SCHEMA public TO fast_tag_test_user")
        .execute(&pool)
        .await
        .ok();
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    
    pool
}

/// Creates a test user in the database and returns the user ID
pub async fn create_test_user(pool: &Pool<Postgres>) -> Uuid {
    let user_id = Uuid::new_v4();
    let unique_email = format!("test-{}@example.com", user_id);

    sqlx::query!(
        "INSERT INTO users (id, email, name, provider, provider_id) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        unique_email,
        "Test User",
        "test",
        user_id.to_string()
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

/// Creates a test project owned by the specified user and returns the project ID
pub async fn create_test_project(pool: &Pool<Postgres>, user_id: Uuid) -> Uuid {
    let project_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO projects (id, name, description, owner_id) VALUES ($1, $2, $3, $4)",
        project_id,
        "Test Project",
        Some("Test Description"),
        user_id
    )
    .execute(pool)
    .await
    .expect("Failed to create test project");

    sqlx::query!(
        "INSERT INTO project_members (project_id, user_id, role) VALUES ($1, $2, $3)",
        project_id,
        user_id,
        "owner"
    )
    .execute(pool)
    .await
    .expect("Failed to add user to project");

    project_id
}

/// Creates a test user and project, returning both IDs
pub async fn setup_test_user_and_project(pool: &Pool<Postgres>) -> (Uuid, Uuid) {
    let user_id = create_test_user(pool).await;
    let project_id = create_test_project(pool, user_id).await;
    (user_id, project_id)
}

/// Creates a test project with storage configuration (for storage tests)
pub async fn create_test_project_with_storage(pool: &Pool<Postgres>, user_id: Uuid) -> Uuid {
    let project_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO projects (id, name, description, owner_id, storage_config) 
        VALUES ($1, $2, $3, $4, $5)
        "#,
        project_id,
        "Test Project with Storage",
        Some("Test Description"),
        user_id,
        Some(serde_json::json!({
            "type": "local",
            "base_path": "/tmp/fast_tag_test"
        }))
    )
    .execute(pool)
    .await
    .expect("Failed to create test project with storage");

    sqlx::query!(
        "INSERT INTO project_members (project_id, user_id, role) VALUES ($1, $2, $3)",
        project_id,
        user_id,
        "owner"
    )
    .execute(pool)
    .await
    .expect("Failed to add user to project");

    project_id
}

/// Creates a test project with sync-compatible storage configuration
pub async fn create_test_project_with_sync_storage(pool: &Pool<Postgres>, user_id: Uuid) -> Uuid {
    let project_id = Uuid::new_v4();
    
    let storage_config = serde_json::json!({
        "provider": "local",
        "config": {
            "path": "/tmp/test-storage"
        }
    });

    sqlx::query!(
        "INSERT INTO projects (id, name, description, owner_id, storage_config) VALUES ($1, $2, $3, $4, $5)",
        project_id,
        "Test Project",
        "Test project description",
        user_id,
        storage_config
    )
    .execute(pool)
    .await
    .expect("Failed to create test project");

    sqlx::query!(
        "INSERT INTO project_members (project_id, user_id, role) VALUES ($1, $2, $3)",
        project_id,
        user_id,
        "owner"
    )
    .execute(pool)
    .await
    .expect("Failed to add user to project");

    project_id
}

/// Creates a test user and project with storage configuration, returning both IDs
pub async fn setup_test_user_and_project_with_storage(pool: &Pool<Postgres>) -> (Uuid, Uuid) {
    let user_id = create_test_user(pool).await;
    let project_id = create_test_project_with_storage(pool, user_id).await;
    (user_id, project_id)
}

/// Creates a test user and returns the full User object (for projects that need User type)
pub async fn create_test_user_with_details(pool: &Pool<Postgres>) -> User {
    let user_id = Uuid::new_v4();
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, email, name, avatar_url, provider, provider_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, email, name, avatar_url, provider, provider_id, created_at, updated_at
        "#
    )
    .bind(user_id)
    .bind("test@example.com")
    .bind("Test User")
    .bind("https://example.com/avatar.jpg")
    .bind("google")
    .bind("google-id-123")
    .fetch_one(pool)
    .await
    .expect("Failed to create test user")
}

/// Creates a test user and project with sync-compatible storage, returning both IDs
pub async fn setup_test_user_and_project_with_sync_storage(pool: &Pool<Postgres>) -> (Uuid, Uuid) {
    let user_id = create_test_user(pool).await;
    let project_id = create_test_project_with_sync_storage(pool, user_id).await;
    (user_id, project_id)
}