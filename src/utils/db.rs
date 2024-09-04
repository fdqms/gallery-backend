use surrealdb::engine::local::Db;
use surrealdb::{Response, Surreal};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Deserialize)]
struct RecordId {
    id: Thing,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    id: Option<String>,
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DbPost {
    id: String,
    image: String,
    ratio: String,
}

pub async fn user_search(database: &Surreal<Db>, username: &String) -> surrealdb::Result<Vec<User>> {

    let mut result = database.query(format!(r#"
        SELECT meta::id(id) AS id, username FROM user WHERE username = /^{}.*/;
    "#, username)).await?;
    
    let user: Vec<User> = result.take(0)?;


    Ok(user)
}

pub async fn friend_add(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<String> {
    database.query(format!(r#"
    RELATE {}->friend->{} SET accepted=false;
    "#, user_id, friend_id)).await?;

    Ok("".to_string())
}

pub async fn friend_accept(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<String> {
    database.query(format!(r#"
    UPDATE friend WHERE out={} AND in={} SET accepted=true;
    "#, user_id, friend_id)).await?;

    Ok("".to_string())
}

pub async fn friend_delete(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<String> {
    database.query(format!(r#"
    DELETE friend WHERE in={} AND out={};
    "#, user_id, friend_id)).await?;

    Ok("".to_string())
}

pub async fn friend_list(database: &Surreal<Db>, user_id: &String) -> surrealdb::Result<String> {
    let mut result = database.query(format!(r#"
       SELECT array::complement(<->friend<->user, [id]).username AS friends FROM {};
    "#, user_id)).await?;

    dbg!(result);

    Ok("".to_string())
}

pub async fn friend_post(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<String> {
    let mut result = database.query(format!(r#"
       SELECT array::complement(<->friend<->user, [id]).username AS friends FROM {};
    "#, user_id)).await?;

    Ok("".to_string())
}

pub async fn post_delete(database: &Surreal<Db>, user_id: &String, post_id: &String) -> surrealdb::Result<String> {
    let mut result = database.query(format!(r#"
    let $post = UPDATE {} SET posts -= posts[WHERE id = u'{}'] RETURN BEFORE;
    let $res = $post[0].posts[WHERE id = u'{}'].image;
    RETURN $res[0];
    "#, user_id, post_id, post_id)).await?;

    let image: Option<String> = result.take(2).unwrap();

    Ok(image.unwrap())
}

pub async fn post_get_all(database: &Surreal<Db>, user_id: &String) -> surrealdb::Result<String> {
    let mut result: Response = database.query(format!(r#"
    let $user = SELECT posts FROM {};
    RETURN $user[0].posts;
    "#, user_id)).await?;

    let posts: Vec<DbPost> = result.take(1).unwrap();

    Ok(serde_json::to_string(&posts).unwrap())
}

pub async fn post_add(database: &Surreal<Db>, ratio: String, image: String, user_id: &String) -> surrealdb::Result<String> {
    let mut result: Response = database.query(format!(r#"
    let $updated_data = UPDATE {} SET posts +=  {{
        id: rand::uuid::v7(),
        image: '{}',
        ratio: '{}'
    }};
    RETURN array::last($updated_data.posts[0]).id;

    "#, user_id, image, ratio)).await?;

    let id: Option<String> = result.take(1).unwrap();

    Ok(id.unwrap())
}

pub async fn login(database: &Surreal<Db>, username: &String, password: &String) -> surrealdb::Result<String> {
    let mut result: Response = database.query(format!(r#"
        let $user = SELECT password, id FROM user WHERE username='{}' LIMIT 1;

        IF array::len($user) == 0 {{
            RETURN type::string(-1);
        }} ELSE IF crypto::argon2::compare($user[0].password,'{}') {{
            RETURN type::string($user[0].id);
        }} ELSE {{
            RETURN type::string(-1);
        }};
    "#, username, password)).await?;

    let id: Option<String> = result.take(1).unwrap();

    Ok(id.unwrap())
}

pub async fn register(database: &Surreal<Db>, username: &String, email: &String, password: &String) -> surrealdb::Result<String> {
    let mut result: Response = database.query(format!(r#"
    RETURN type::string((CREATE user CONTENT {{
        username: '{}',
        email: '{}',
        password: crypto::argon2::generate('{}'),
        posts: []
    }}).id);
    "#, username, email, password)).await?;

    let user: Option<String> = result.take(0).unwrap();
    let mut id = user.unwrap();

    id = id.get(1..id.len() - 1).unwrap_or("-1").to_string();

    Ok(id)
}