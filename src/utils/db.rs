use surrealdb::engine::local::Db;
use surrealdb::sql::Thing;
use surrealdb::{Response, Surreal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct RecordId {
    #[allow(dead_code)]
    id: Thing,
}

#[derive(Serialize, Deserialize)]
struct User {
    username: String,
    email: String,
    password: String
}

pub async fn login(database: &Surreal<Db>, username: &String, password: &String) -> surrealdb::Result<String>{
    let mut result: Response = database.query(format!(r#"
        let $user = SELECT password, id FROM user WHERE username='{}' LIMIT 1;
        IF $user[0].password == '{}' {{
            RETURN type::string($user[0].id);
        }} ELSE {{
            RETURN type::string(-1);;
        }};
    "#, username, password)).await?;

    let id: Option<String> = result.take(1).unwrap();

    return Ok(id.unwrap());
}

pub async fn register(database: &Surreal<Db>, username: &String, email: &String, password: &String) -> surrealdb::Result<String> {
    let mut result: Response = database.query(format!(r#"
    RETURN type::string((CREATE user CONTENT {{
        username: '{}',
        email: '{}',
        password: '{}'
    }}).id);
    "#, username, email, password)).await?;

    let user: Option<String> = result.take(0).unwrap();
    let mut id = user.unwrap();

    id = id.get(1..id.len()-1).unwrap_or("-1").to_string();

    return Ok(id);
}