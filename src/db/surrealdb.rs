use actix_web::web::Json;
use surrealdb::engine::local::Db;
use surrealdb::{Response, Surreal};
use crate::model::post::Post;
use crate::model::user::User;

pub async fn user_search(database: &Surreal<Db>, user_id: &String, username: &String) -> surrealdb::Result<Vec<User>> {
    let mut result = database.query(format!(r#"
        SELECT record::id(id) AS id, username FROM user WHERE username = /^{}.*/ AND id != {};
    "#, username, user_id)).await?;

    let user: Vec<User> = result.take(0)?;

    Ok(user)
}

pub async fn follow(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<()> {
    database.query(format!(r#"
    RELATE {}->friend->user:{} SET accepted=false;
    "#, user_id, friend_id)).await?;

    Ok(())
}

pub async fn unfollow(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<()> {
    database.query(format!(r#"
    $user = {};
    $friend = user:{};
    DELETE friend WHERE in=$user AND out=$friend;
    DELETE friend WHERE in=$friend AND out=$user;
    "#, user_id, friend_id)).await?;

    Ok(())
}

pub async fn follow_accept(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<()> {
    // sorguda ekleren bir kez yapıp defalarca sorguladığımız için yükü buraya veriyoruz
    database.query(format!(r#"
    $friend = user:{};
    $user = {};
    UPDATE friend SET accepted=true WHERE in=$friend AND out=$user;
    RELATE $user -> friend -> $friend SET accepted=true;
     "#, friend_id, user_id)).await?;

    Ok(())
}

pub async fn follow_reject(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<()> {
    println!("{}", user_id);
    println!("{}", friend_id);
    database.query(format!(r#"
    DELETE friend WHERE out={} AND in=user:{};
    "#, user_id, friend_id)).await?;

    Ok(())
}

pub async fn follow_pendings(database: &Surreal<Db>, user_id: &String) -> surrealdb::Result<Vec<User>> {
    let mut result = database.query(format!(r#"
    (SELECT <-(friend WHERE accepted=false)<-user AS friends FROM {})[0].friends.map(|$f| {{username: $f.username, id: record::id($f.id)}});
    "#, user_id)).await?;

    let pendings: Vec<User> = result.take(0)?;

    Ok(pendings)
}

pub async fn follow_requests(database: &Surreal<Db>, user_id: &String) -> surrealdb::Result<Vec<User>> {
    let mut result = database.query(format!(r#"
        (SELECT ->(friend WHERE accepted=false)->user AS friends FROM {})[0].friends.map(|$f| {{username: $f.username, id: record::id($f.id)}});
        "#, user_id)).await.expect("err");
    let requests: Vec<User> = result.take(0)?;

    Ok(requests)
}

pub async fn friend_post(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<Vec<Post>> {
    let mut result = database.query(format!(r#"(SELECT ->(friend WHERE out=user:{})->user[0].posts as posts FROM {})[0].posts"#, friend_id, user_id)).await?;
    let posts: Vec<Post> = result.take(0)?;

    Ok(posts)
}

pub async fn friends(database: &Surreal<Db>, user_id: &String) -> surrealdb::Result<Vec<User>> {
    let mut result = database.query(format!(r#"(SELECT ->(friend WHERE accepted=true)->user AS friends FROM {})[0].friends.map(|$f| {{username: $f.username, id: record::id($f.id)}});"#, user_id)).await?;

    let friends: Vec<User> = result.take(0)?;

    Ok(friends)
}

/*
tüm arkadaşların gönderilerini listele
pub async fn friend_post(database: &Surreal<Db>, user_id: &String, friend_id: &String) -> surrealdb::Result<String> {
    database.query(format!(r#"SELECT array::complement(<->friend<->user, [id]).username AS friends FROM {};"#, user_id)).await?;

    Ok("".to_string())
}
*/

pub async fn post_delete(database: &Surreal<Db>, user_id: &String, post_id: &String) -> surrealdb::Result<Option<String>> {
    let mut result = database.query(format!(r#"
    let $post = UPDATE {} SET posts -= posts[WHERE id = '{}'] RETURN BEFORE;
    let $res = $post[0].posts[WHERE id = '{}'].image;
    $res[0];
    "#, user_id, post_id, post_id)).await?;

    let image: Option<String> = result.take(2)?;

    Ok(image)
}

pub async fn post_get_all(database: &Surreal<Db>, user_id: &String) -> surrealdb::Result<Json<Vec<Post>>> {
    let mut result: Response = database.query(format!(r#"
    let $user = SELECT posts FROM {};
    $user[0].posts;
    "#, user_id)).await?;

    let posts: Vec<Post> = result.take(1)?;

    Ok(Json(posts))
}

pub async fn post_add(database: &Surreal<Db>, ratio: String, image: String, user_id: &String) -> surrealdb::Result<String> {
    let mut result: Response = database.query(format!(r#"
    let $updated_data = UPDATE {} SET posts +=  {{
        id: type::string(rand::uuid::v7()),
        image: '{}',
        ratio: '{}'
    }};
    array::last($updated_data.posts[0]).id;

    "#, user_id, image, ratio)).await?;

    let id: Option<String> = result.take(1)?;

    Ok(id.unwrap())
}

pub async fn login(database: &Surreal<Db>, username: &String, password: &String) -> surrealdb::Result<String> {
    let mut result: Response = database.query(format!(r#"
        let $user = SELECT password, id FROM user WHERE username='{}' LIMIT 1;

        IF array::len($user) == 0 {{
            type::string(-1);
        }} ELSE IF crypto::argon2::compare($user[0].password,'{}') {{
            type::string($user[0].id);
        }} ELSE {{
            type::string(-1);
        }};
    "#, username, password)).await?;

    let id: Option<String> = result.take(1)?;

    Ok(id.unwrap())
}

pub async fn register(database: &Surreal<Db>, username: &String, email: &String, password: &String) -> surrealdb::Result<String> {
    let mut result: Response = database.query(format!(r#"
    type::string((CREATE user CONTENT {{
        username: '{}',
        email: '{}',
        password: crypto::argon2::generate('{}'),
        posts: []
    }}).id);
    "#, username, email, password)).await?;

    let user: Option<String> = result.take(0)?;
    let mut id = user.unwrap();

    id = id.get(1..id.len() - 1).unwrap_or("-1").to_string();

    Ok(id)
}