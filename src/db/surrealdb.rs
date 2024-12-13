
use crate::model::post::Post;
use crate::model::user::User;
use actix_web::web::Json;
use std::sync::LazyLock;
use surrealdb::engine::local::Db;
use surrealdb::{Response, Surreal};

pub static DB: LazyLock<Surreal<Db>> = LazyLock::new(Surreal::init);

pub async fn add_premium(
    user_id: &String,
    transaction: &String,
    transaction_date: &u64,
) -> surrealdb::Result<()> {
    DB.query(format!(
        r#"
        UPDATE {} SET transaction = '{}', transaction_date = time::from::unix({}), upload_limit = 20;
    "#,
        user_id, transaction, transaction_date
    ))
    .await?;

    Ok(())
}

pub async fn upload_limit(user_id: &String) -> surrealdb::Result<i64> {
    let mut result: Response = DB.query(
        format!(r#"
            (SELECT upload_limit FROM {})[0].upload_limit;
        "#, user_id)
    ).await?;

    let upload_limit: Option<i64> = result.take(0)?;

    Ok(upload_limit.unwrap())
}

pub async fn check_premium(user_id: &String) -> surrealdb::Result<i64> {
    let mut result: Response = DB.query(format!(r#"
        (SELECT time::millis(transaction_date + 30d) as ms FROM {} WHERE transaction_date != NONE AND (transaction_date + 30d) > time::now() AND upload_limit > 0)[0].ms;
    "#, user_id)).await?;

    let last_date: Option<i64> = result.take(0)?;

    Ok(last_date.unwrap_or(0))
}

pub async fn add_transaction(user_id: &String, transaction: &String) -> surrealdb::Result<()> {
    DB.query(format!("
        CREATE payment SET 
            user_id = '{}',
            transaction = '{}';
    ", &user_id, &transaction)).await?;

    Ok(())
}

pub async fn check_transaction(transaction: &String) -> surrealdb::Result<bool> {
    let mut result: Response = DB
        .query(format!(
            r#"
        SELECT true FROM user WHERE transaction='{}';
    "#,
            transaction
        ))
        .await?;

    let val: Option<bool> = result.take(0)?;

    Ok(val.unwrap_or(false))
}

pub async fn profile(user_id: &String) -> surrealdb::Result<User> {
    let mut result = DB
        .query(format!(
            r#"
        SELECT record::id(id) AS id, username, email FROM {};
    "#,
            user_id
        ))
        .await?;

    let user: Option<User> = result.take(0)?;

    Ok(user.unwrap())
}

pub async fn user_search(user_id: &String, username: &String) -> surrealdb::Result<Vec<User>> {
    let mut result = DB
        .query(format!(
            r#"
        SELECT record::id(id) AS id, username FROM user WHERE username = /^{}.*/ AND id != {} AND NOT (->friend->user OR <-friend<-user);
    "#,
            username, user_id
        ))
        .await?;

    let user: Vec<User> = result.take(0)?;

    Ok(user)
}

pub async fn follow(user_id: &String, friend_id: &String) -> surrealdb::Result<()> {
    DB.query(format!(
        r#"
    RELATE {}->friend->user:{} SET accepted=false;
    "#,
        user_id, friend_id
    ))
    .await?;

    Ok(())
}

pub async fn unfollow(user_id: &String, friend_id: &String) -> surrealdb::Result<()> {
    DB.query(format!(
        r#"
    $user = {};
    $friend = user:{};
    DELETE friend WHERE in=$user AND out=$friend;
    DELETE friend WHERE in=$friend AND out=$user;
    "#,
        user_id, friend_id
    ))
    .await?;

    Ok(())
}

pub async fn follow_accept(user_id: &String, friend_id: &String) -> surrealdb::Result<()> {
    // sorguda ekleren bir kez yapıp defalarca sorguladığımız için yükü buraya veriyoruz
    DB.query(format!(
        r#"
    $friend = user:{};
    $user = {};
    UPDATE friend SET accepted=true WHERE in=$friend AND out=$user;
    RELATE $user -> friend -> $friend SET accepted=true;
     "#,
        friend_id, user_id
    ))
    .await?;

    Ok(())
}

pub async fn follow_reject(user_id: &String, friend_id: &String) -> surrealdb::Result<()> {
    DB.query(format!(
        r#"
    DELETE friend WHERE out={} AND in=user:{};
    "#,
        user_id, friend_id
    ))
    .await?;

    Ok(())
}

pub async fn follow_pendings(user_id: &String) -> surrealdb::Result<Vec<User>> {
    let mut result = DB.query(format!(r#"
    (SELECT <-(friend WHERE accepted=false)<-user AS friends FROM {})[0].friends.map(|$f| {{username: $f.username, id: record::id($f.id)}});
    "#, user_id)).await?;

    let pendings: Vec<User> = result.take(0)?;

    Ok(pendings)
}

pub async fn follow_requests(user_id: &String) -> surrealdb::Result<Vec<User>> {
    let mut result = DB.query(format!(r#"
        (SELECT ->(friend WHERE accepted=false)->user AS friends FROM {})[0].friends.map(|$f| {{username: $f.username, id: record::id($f.id)}});
        "#, user_id)).await.expect("err");
    let requests: Vec<User> = result.take(0)?;

    Ok(requests)
}

pub async fn friend_post(user_id: &String, friend_id: &String) -> surrealdb::Result<Vec<Post>> {
    let mut result = DB
        .query(format!(
            r#"(SELECT ->(friend WHERE out=user:{})->user[0].posts as posts FROM {})[0].posts"#,
            friend_id, user_id
        ))
        .await?;
    let posts: Vec<Post> = result.take(0)?;

    Ok(posts)
}

pub async fn friends(user_id: &String) -> surrealdb::Result<Vec<User>> {
    let mut result = DB.query(format!(r#"(SELECT ->(friend WHERE accepted=true)->user AS friends FROM {})[0].friends.map(|$f| {{username: $f.username, id: record::id($f.id)}});"#, user_id)).await?;

    let friends: Vec<User> = result.take(0)?;

    Ok(friends)
}

/*
tüm arkadaşların gönderilerini listele
pub async fn friend_post(user_id: &String, friend_id: &String) -> surrealdb::Result<String> {
    DB.query(format!(r#"SELECT array::complement(<->friend<->user, [id]).username AS friends FROM {};"#, user_id)).await?;

    Ok("".to_string())
}
*/

pub async fn post_delete(user_id: &String, post_id: &String) -> surrealdb::Result<Option<String>> {
    let mut result = DB
        .query(format!(
            r#"
    let $post = UPDATE {} SET posts -= posts[WHERE id = '{}'] RETURN BEFORE;
    let $res = $post[0].posts[WHERE id = '{}'].image;
    $res[0];
    "#,
            user_id, post_id, post_id
        ))
        .await?;

    let image: Option<String> = result.take(2)?;

    Ok(image)
}

pub async fn post_get_all(user_id: &String) -> surrealdb::Result<Json<Vec<Post>>> {
    let mut result: Response = DB
        .query(format!(
            r#"
    let $user = SELECT posts FROM {};
    $user[0].posts;
    "#,
            user_id
        ))
        .await?;

    let posts: Vec<Post> = result.take(1)?;

    Ok(Json(posts))
}

pub async fn post_add(
    ratio: String,
    image: &String,
    user_id: &String,
) -> surrealdb::Result<String> {
    let mut result: Response = DB
        .query(format!(
            r#"
            let $user_id = {};
    let $id = type::string(rand::uuid::v7());
    let $updated_data = UPDATE $user_id SET posts +=  {{
        id: $id,
        image: '{}',
        ratio: '{}'
    }};
    $id;
    UPDATE $user_id SET upload_limit = upload_limit - 1;
    "#,
            user_id, image, ratio
        ))
        .await?;

    let id: Option<String> = result.take(3)?;

    Ok(id.unwrap())
}

pub async fn login(username: &String, password: &String) -> surrealdb::Result<String> {
    let mut result: Response = DB
        .query(format!(
            r#"
        let $user = SELECT password, id FROM user WHERE username='{}' LIMIT 1;

        IF array::len($user) == 0 {{
            type::string(-1);
        }} ELSE IF crypto::argon2::compare($user[0].password,'{}') {{
            type::string($user[0].id);
        }} ELSE {{
            type::string(-1);
        }};
    "#,
            username, password
        ))
        .await?;

    let id: Option<String> = result.take(1)?;

    Ok(id.unwrap())
}

pub async fn register(
    username: &String,
    email: &String,
    password: &String,
) -> surrealdb::Result<String> {
    let mut result: Response = DB
        .query(format!(
            r#"
    type::string((CREATE user CONTENT {{
        username: '{}',
        email: '{}',
        password: crypto::argon2::generate('{}'),
        posts: []
    }}).id);
    "#,
            username, email, password
        ))
        .await?;

    let user: Option<String> = result.take(0)?;
    let mut id = user.unwrap();

    id = id.get(1..id.len() - 1).unwrap_or("-1").to_string();

    Ok(id)
}

pub async fn change_password(
    user_id: &String,
    old: &String,
    new: &String,
) -> surrealdb::Result<()> {
    DB
        .query(format!(
            r#"
            $u = {};
            let $user = SELECT password, id FROM $u;
            IF crypto::argon2::compare($user[0].password,'{}') {{
                UPDATE $u SET password = crypto::argon2::generate('{}');
            }};
    "#,
            user_id, old, new
        ))
        .await?;

    Ok(())
}

pub async fn user_delete(user_id: &String) -> surrealdb::Result<()> {
    DB.query(format!(r#"DELETE {};"#, user_id)).await?;

    Ok(())
}

pub async fn friend_delete(user_id: &String) -> surrealdb::Result<()> {
    DB.query(format!(
        r#"
    $user = {};
    DELETE friend WHERE in=$user OR out=$user;
    "#,
        user_id
    ))
    .await?;

    Ok(())
}
