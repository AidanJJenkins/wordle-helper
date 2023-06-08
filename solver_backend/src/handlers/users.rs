use crate::AppState;
use actix_web::{put, delete, get, post, web, HttpResponse, Responder};
use chrono::Local;
use sqlx::Row;
use log::error;
use crate::models::users_models::{NewUser, UserResponse, LoginCredentials, Token};
use crate::utils::bcrypt_utils::{hash_password, verify_password};
use crate::utils::jwt_utils::generate_token;
use std::time::Instant;

pub fn user_routes(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/users")
        .service(create_user)
        .service(get_all_users)
        .service(get_user_by_id)
        .service(update_user)
        .service(delete_user)
        .service(login_user)
        .service(revoke_token);

    conf.service(scope);
}
//This is an attribute macro that indicates that this function is associated with the HTTP POST method and the "/user" route
//It's used by actix web framework to handle incoming POST requests to the "/user" endpoint.
#[post("/register")]
pub async fn create_user(pool: web::Data<AppState>, new_user: web::Json<NewUser>) -> impl Responder {
    let now = Local::now().naive_local();
    //This line hashes the user's password using the hash_password function
    //It uses the match control flow construct to handle the result of the hash_password
    //the match control flow construct allows you to match a value against a series of patterns and execute code based on the matched pattern
    let hashed_password = match hash_password(&new_user.password) {
        //is the hashing is successful is assigns the hashed password to hashed_password
        Ok(hashed) => hashed,
        //if it is not successful an error message is logged
        Err(error) => {
            error!("Failed to hash password: {}", error);
            return HttpResponse::InternalServerError().body("Failed to create user");
        }
    };
    // sqlx::query(r#"..."#): This starts building an SQL query using a raw string literal 
    match sqlx::query(
            r#"
            INSERT INTO users (username, email, password, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#)
        .bind(&new_user.username)
        .bind(&new_user.email)
        .bind(&hashed_password)
        .bind(now)
        .bind(now)
        .fetch_one(&pool.db)
        .await {
            //if the query is successful, it returns an httpresponse
            //the "_" wildcard pattern is a catch-all for unmatched cases. If no patterns match and there is no "_" arm
            //the match expression will be considered incomplete and the compiler will raise an error.
            Ok(_) => HttpResponse::Ok().body("User created"),
            //if not successful, it logs an error
            Err(error) => {
                error!("Failed to insert new user: {}", error);
                HttpResponse::InternalServerError().body("Failed to create user")
            }
        }
}

#[get("/")]
// defineing function, it take the application state as param, which allows you to share app data
// "impl Responder" means mean the function is returning a value that can be converted to an Http
// response
pub async fn get_all_users(pool: web::Data<AppState>) -> impl Responder {
    //This creates a row vairale with a SQL query to the database to retrieve all of the records in the users table.
    let rows = sqlx::query("SELECT id, username, email, password, created_at, updated_at FROM users")
    //The fetch_all() method sends the query to the database and returns a vector of rows representing 
    //the results of the query. We store this vector of rows in a variable called rows
        .fetch_all(&pool.db)
        .await
        // unwrap  returns the values from the query
        .unwrap();
    // users is a variable that stores the data from our query, 
    // where each row returned by the query is represented as a struct UserResponse
    let users: Vec<UserResponse> = rows
        //.into_iter() creates an iterator over the rows vector so that we can process each row individually
        .into_iter()
        //map() method applies a transformation to each element of the iterator, in this case, 
        //we are constructing a new UserResponse object for each row.
        .map(|row| {
            UserResponse {
                //.get() is a method provided by the Row struct of the sqlx crate. It's used to retrieve the value of a column from a row.
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password: row.get("password"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        })
    //collect() method is called on the iterator to collect all the transformed elements into a new vector of type Vec<UserResponse>.
    .collect();

    //HTTP response with a status code of 200 Ok, indicating that the request has been successfully processed. 
    //The json() method serializes the users variable into a JSON string
    HttpResponse::Ok().json(users)
}

#[get("/{id}")]
pub async fn get_user_by_id(pool: web::Data<AppState>, path: web::Path<(i32,)>) -> impl Responder {
    let (id,) = path.into_inner();

    let query = sqlx::query_as::<_, UserResponse>(
            "SELECT id, username, email, password, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_one(&pool.db)
        .await;

    match query {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(_) => HttpResponse::NotFound().json("User not found"),
    }
}

#[put("/get/{id}")]
pub async fn update_user(pool: web::Data<AppState>, path: web::Path<(i32,)>, updated_user: web::Json<UserResponse>) -> impl Responder{
    //the into_inner() method is used to access the inner value, which is a tuple containing a single i32 value 
    //the tuple is then destructured, and the id is bound to the variable id.
    let (id,) = path.into_inner();
    let user = updated_user.into_inner();

    let query = sqlx::query(
            "UPDATE users SET username = $1, email = $2, password = $3 WHERE id = $4"
        )
        .bind(user.username)
        .bind(user.email)
        .bind(user.password)
        .bind(id)
        .execute(&pool.db)
        .await;

    match query {
        Ok(_) => HttpResponse::Ok().json("User updated successfully"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to update user"),
    }
}

#[delete("/delete/{id}")]
pub async fn delete_user(pool: web::Data<AppState>, path: web::Path<(i32,)>) -> impl Responder {
    let (id,) = path.into_inner();

    let query = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&pool.db)
        .await;

    match query {
        Ok(_) => HttpResponse::Ok().json("User deleted successfully"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to delete user"),
    }
}

async fn validate_credentials(pool: &web::Data<AppState>, username: &str, password: &str) -> Option<i32> {
    // Fetch the user's hashed password from the database
    // if I dont need to bind anything, use the query! macro instead of query
    let query_result = sqlx::query!(
        r#"
        SELECT id, password FROM users WHERE username = $1
        "#,
        username
    )
    .fetch_optional(&pool.db)
    .await
    .expect("Failed to execute SQL query");

    if let Some(row) = query_result {
        let stored_password = row.password;

        // Verify the provided password against the stored hashed password
    let start_time = Instant::now();
        if verify_password(password, &stored_password) {
            // Return the user ID if the credentials are valid
    let total_duration = start_time.elapsed();
    println!("Total validation time: {:?}", total_duration);
            return Some(row.id);
        }
    }

    None
}

#[post("/login")]
pub async fn login_user(pool: web::Data<AppState>, credentials: web::Json<LoginCredentials>) -> HttpResponse {
    // Validate user credentials against the database
    let user_id = match validate_credentials(&pool, &credentials.username, &credentials.password).await {
        Some(user_id) => user_id,
        None => return HttpResponse::Unauthorized().body("Invalid credentials"),
    };

    // Generate a JWT token for the authenticated user
    let token = match generate_token(user_id) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to generate token"),
    };

    // Return the JWT token in the response
    HttpResponse::Ok().body(token)
}

#[post("/revoke_token")]
pub async fn revoke_token(pool: web::Data<AppState>, token: web::Json<Token>) -> HttpResponse {
    let now = Local::now().naive_local();

    match sqlx::query(
            r#"
            INSERT INTO revoked_tokens (token, created_at)
            VALUES ($1, $2)
            "#)
        .bind(&token.token)
        .bind(now)
        .execute(&pool.db)
        .await {
            Ok(_) => HttpResponse::Ok().body("Token added"),
            Err(error) => {
                println!("erro: {}", error);
                error!("Failed to insert new token: {}", error);
                HttpResponse::InternalServerError().body("Failed to add token")
            }
        }
}

