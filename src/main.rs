use axum::{
    extract::State,
    response::IntoResponse,
    routing::{delete, get},
    Json, Router,
};
use futures::{future::ok, TryStreamExt};
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgPoolOptions, PgRow},
    PgPool, Row,
};

use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let db_url = "postgresql://postgres:Zignuts@123@localhost/todo-rust";

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("can't connect to database");

    let app = Router::new()
        .route("/todos", get(get_todos).post(create_todo))
        .route("/todo/:id", delete(delete_todo).get(find_todo).patch(update_todo))
        .with_state(pool);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    println!("Listening to 0.0.0.0:3000");

    Ok(())
}

///Show all todos
async fn get_todos(State(pool): State<PgPool>) -> impl IntoResponse {
    let mut _todos: Vec<Todo> = Vec::new();

    sqlx::query("SELECT * FROM todo")
        .map(|row: PgRow| {
            let todo = Todo {
                id: row.get("id"),
                title: row.get("content"),
            };
            _todos.push(todo);
        })
        .fetch_all(&pool)
        .await
        .unwrap();

    Json(_todos)
}

/// Create new Todo
async fn create_todo(
    State(pool): State<PgPool>,
    Json(input): Json<CreateTodo>,
) -> impl IntoResponse {
    let id = rand::random::<u64>().to_string();
    let todo = Todo {
        id,
        title: input.text,
    };

    sqlx::query(r#"insert into todo (id,content) values ($1,$2)"#)
        .bind(&todo.id)
        .bind(todo.title)
        .execute(&pool)
        .await
        .unwrap();

    Json(format!("Todo Created with ID : {}", todo.id.to_string()))
}

///Delete Todo
async fn delete_todo(State(pool): State<PgPool>, Json(id): Json<String>) -> impl IntoResponse {
    let result = sqlx::query(r#"delete from todo where id=$1"#)
        .bind(id)
        .execute(&pool)
        .await;
    match result {
        Ok(v) => {
            if v.rows_affected() > 0 {
                Json("Deleted Successfully")
            } else {
                Json("No element found!")
            }
        }
        Err(e) => Json("Something went wrong! {e}"),
    }
}

async fn find_todo(State(pool): State<PgPool>, Json(id): Json<String>) -> impl IntoResponse {
    let result = sqlx::query(r#"select * from todo where id=$1"#)
        .bind(id)
        .fetch_one(&pool)
        .await;

    let todo: Option<Response> = match result {
        Ok(val) => {
            let todo = Todo {
                id: val.get("id"),
                title: val.get("content"),
            };
            Some(Response {
                data: Some(todo),
                error: None,
                message: Some("Success".to_string())
            })
        }
        Err(e) => Some(Response {
            data: None,
            error: Some(CustomError {
                message: e.to_string(),
            }),
            message: Some("Failed".to_string())
        }),
    };
    Json(todo)
}

/// Update Todo
async fn update_todo(
    State(pool): State<PgPool>,
    Json(input): Json<CreateTodo>,
) -> impl IntoResponse {
    let result = sqlx::query(r#"update todo set content=$1 where id=$2"#)
        .bind(&input.text)
        .bind(&input.id)
        .execute(&pool)
        .await;

    let response: Response = match result {
        Err(e) => Response {
            data: None,
            error: Some(CustomError {
                message: e.to_string(),
            }),
            message: Some("Failed to update todo".to_string()),
        },
        Ok(val) => {
            if val.rows_affected() > 0 {
                Response {
                    data: None,
                    error: None,
                    message: Some("Successfull Updated".to_string()),
                }
            } else {
                Response {
                    data: None,
                    error: None,
                    message: Some("No such todo found!".to_string()),
                }
            }
        }
    };

    Json(response)
}

#[derive(Debug, Deserialize)]
struct CreateTodo {
    id: Option<String>,
    text: String,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
struct Todo {
    id: String,
    title: String,
}
#[derive(Debug, Serialize, Clone)]
struct Response {
    data: Option<Todo>,
    error: Option<CustomError>,
    message: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct CustomError {
    message: String,
}
