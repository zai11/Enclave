use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::db::models::{blocked_user::BlockedUser, direct_message::DirectMessage, friend::Friend, friend_request::FriendRequest, identity::Identity, post::Post, user::User};

pub mod models;

pub static DATABASE: once_cell::sync::Lazy<Arc<std::sync::Mutex<Connection>>> =
    once_cell::sync::Lazy::new(|| {
        init_db("./enclave.db").unwrap()
    });

pub fn init_db(path: &str) -> anyhow::Result<Arc<Mutex<Connection>>> {
    log::info!("Initilising database...");

    let db = Connection::open(path)?;
    log::info!("Created enclave database.");

    db.execute("PRAGMA foreign_keys = ON", ())?;

    if !db.table_exists(None, "tbl_identity")? {
        db.execute("CREATE TABLE tbl_identity (
                            id INTEGER PRIMARY KEY CHECK (id=1),
                            keypair BLOB NOT NULL,
                            peer_id TEXT NOT NULL,
                            port_number INTEGER NOT NULL,
                            created_at INTEGER NOT NULL
                        );", ())?;
        log::info!("Created identity table.");
    }

    if !db.table_exists(None, "tbl_users")? {
        db.execute("CREATE TABLE tbl_users (
                            id INTEGER PRIMARY KEY,
                            peer_id TEXT NOT NULL,
                            multiaddr TEXT NOT NULL,
                            nickname TEXT,
                            is_identity BOOLEAN DEFAULT 0,
                            created_at INTEGER NOT NULL
                        );", ())?;
        log::info!("Created connections table.");
    }

    if !db.table_exists(None, "tbl_friend_requests")? {
        db.execute("CREATE TABLE tbl_friend_requests (
                            id INTEGER PRIMARY KEY,
                            from_user_id INTEGER NOT NULL,
                            message TEXT,
                            created_at INTEGER NOT NULL,
                            FOREIGN KEY (from_user_id) REFERENCES tbl_users(id)
                        );", ())?;
        log::info!("Created friend requests table.");
    }

    if !db.table_exists(None, "tbl_friends")? {
        db.execute("CREATE TABLE tbl_friends (
                            id INTEGER PRIMARY KEY,
                            user_id INTEGER NOT NULL,
                            created_at INTEGER NOT NULL,
                            FOREIGN KEY (user_id) REFERENCES tbl_users(id),
                            UNIQUE(user_id)
                        );", ())?;
        log::info!("Created friends table.");
    }

    if !db.table_exists(None, "tbl_direct_messages")? {
        db.execute("CREATE TABLE tbl_direct_messages (
                            id INTEGER PRIMARY KEY,
                            from_peer_id TEXT NOT NULL,
                            to_peer_id TEXT NOT NULL,
                            content TEXT NOT NULL,
                            created_at INTEGER NOT NULL,
                            edited_at INTEGER,
                            read BOOLEAN DEFAULT 0
                        );", ())?;
        log::info!("Created direct messages table.");
    }

    if !db.table_exists(None, "tbl_posts")? {
        db.execute("CREATE TABLE tbl_posts (
                            id INTEGER PRIMARY KEY,
                            author_user_id INTEGER NOT NULL,
                            content TEXT NOT NULL,
                            created_at INTEGER NOT NULL,
                            edited_at INTEGER,
                            FOREIGN KEY (author_user_id) REFERENCES tbl_users(id)
                        );", ())?;
        log::info!("Created posts table.");
    }

    if !db.table_exists(None, "tbl_blocked_users")? {
        db.execute("CREATE TABLE tbl_blocked_users (
                            id INTEGER PRIMARY KEY,
                            user_id INTEGER NOT NULL,
                            blocked_at INTEGER NOT NULL,
                            FOREIGN KEY (user_id) REFERENCES tbl_users(id),
                            UNIQUE(user_id)
                        );", ())?;
        log::info!("Created blocked users table.");
    }

    db.execute("CREATE INDEX IF NOT EXISTS idx_posts_author ON tbl_posts(author_user_id);", ())?;
    db.execute("CREATE INDEX IF NOT EXISTS idx_users_peer_id ON tbl_users(peer_id);", ())?;

    Ok(Arc::new(Mutex::new(db)))
}

pub fn fetch_identity(db: Arc<Mutex<Connection>>) -> anyhow::Result<Identity> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, keypair, peer_id, port_number, created_at FROM tbl_identity")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No identity data was found."));
    }

    let (id, keypair, peer_id, port_number, created_at): (i64, Vec<u8>, String, i64, i64) = query.query_row((), |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    })?;

    Ok(
        Identity::new(
            id, 
            keypair, 
            peer_id, 
            port_number,
            created_at
        )
    )
}

pub fn create_identity(db: Arc<Mutex<Connection>>, keypair: Vec<u8>, peer_id: String, port_number: i64) -> anyhow::Result<i64> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "INSERT INTO tbl_identity (keypair, peer_id, port_number, created_at) VALUES (?1, ?2, ?3, ?4)", 
        rusqlite::params![
            keypair,
            peer_id,
            port_number,
            0
        ]
    )?;

    Ok(db_guard.last_insert_rowid())
}

pub fn fetch_user_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<User> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, peer_id, multiaddr, nickname, is_identity, created_at FROM tbl_users WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("No user with the id {id} was found."));
    }
    
    let (id, peer_id, multiaddr, nickname, is_identity, created_at): (i64, String, String, Option<String>, bool, i64) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
    })?;

    Ok(
        User::new(
            id, 
            peer_id, 
            multiaddr,
            nickname,
            is_identity,
            created_at
        )
    )
}

pub fn fetch_user_by_peer_id(db: Arc<Mutex<Connection>>, peer_id: String) -> anyhow::Result<User> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, peer_id, multiaddr, nickname, is_identity, created_at FROM tbl_users WHERE peer_id=?1;")?;

    if !query.exists(rusqlite::params![peer_id.to_string()])? {
        return Err(anyhow::anyhow!("No user with the peer_id {peer_id} was found."));
    }

    let (id, peer_id, multiaddr, nickname, is_identity, created_at): (i64, String, String, Option<String>, bool, i64) = query.query_row(rusqlite::params![peer_id.to_string()], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
    })?;

    Ok(
        User::new(
            id, 
            peer_id, 
            multiaddr,
            nickname,
            is_identity,
            created_at
        )
    )
}

pub fn fetch_all_users(db: Arc<Mutex<Connection>>) -> anyhow::Result<Vec<User>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, peer_id, multiaddr, nickname, is_identity, created_at FROM tbl_users;")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No user data was found."));
    }

    let rows = query.query_map((), |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;

        Ok(
            User::new(
                row.0,
                row.1,
                row.2,
                row.3,
                row.4,
                row.5
            )
        )
    }).collect::<anyhow::Result<Vec<User>>>()
}

pub fn create_user(db: Arc<Mutex<Connection>>, peer_id: String, multiaddr: String, is_identity: bool) -> anyhow::Result<i64> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let created_at = chrono::Utc::now().timestamp();

    db_guard.execute(
        "INSERT INTO tbl_users (peer_id, multiaddr, is_identity, created_at) VALUES (?1, ?2, ?3, ?4)", 
        rusqlite::params![peer_id.to_string(), multiaddr.to_string(), is_identity, created_at]
    )?;

    Ok(db_guard.last_insert_rowid())
}

pub fn update_user(db: Arc<Mutex<Connection>>, id: i64, multiaddr: Option<String>, nickname: Option<String>) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    if let Some(multiaddr) = multiaddr {
        db_guard.execute(
            "UPDATE tbl_users SET multiaddr=?1 WHERE id=?2;",
            rusqlite::params![multiaddr.to_string(), id]
        )?;
    }

    if let Some(nickname) = nickname {
        db_guard.execute(
            "UPDATE tbl_users SET nickname=?1 WHERE id=?2;",
            rusqlite::params![nickname.to_string(), id]
        )?;
    }

    Ok(())
}

pub fn delete_user(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "DELETE FROM tbl_users WHERE id=?1;", 
        rusqlite::params![id]
    )?;

    Ok(())
}

pub fn fetch_friend_request_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<FriendRequest> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, from_user_id, message, created_at FROM tbl_friend_requests WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("A friend request with id {id} was not found."));
    }

    let (id, from_user_id, message, created_at): (i64, i64, String, i64) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;

    Ok(
        FriendRequest::new(
            id,
            from_user_id,
            message,
            created_at
        )
    )
}

pub fn fetch_friend_request_by_from_user_id(db: Arc<Mutex<Connection>>, from_user_id: i64) -> anyhow::Result<FriendRequest> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, from_user_id, message, created_at FROM tbl_friend_requests WHERE from_user_id=?1;")?;

    if !query.exists(rusqlite::params![from_user_id])? {
        return Err(anyhow::anyhow!("A friend request with from_user_id {from_user_id} was not found."));
    }

    let (id, from_user_id, message, created_at): (i64, i64, String, i64) = query.query_row(rusqlite::params![from_user_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;

    Ok(
        FriendRequest::new(
            id,
            from_user_id,
            message,
            created_at
        )
    )
}

pub fn fetch_all_friend_requests(db: Arc<Mutex<Connection>>) -> anyhow::Result<Vec<FriendRequest>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, from_user_id, message, created_at FROM tbl_friend_requests;")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No friend request data was found."));
    }

    let rows = query.query_map((), |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;

        Ok(
            FriendRequest::new(
                row.0,
                row.1,
                row.2,
                row.3
            )
        )
    }).collect::<anyhow::Result<Vec<FriendRequest>>>()
}

pub fn create_friend_request(db: Arc<Mutex<Connection>>, from_user_id: i64, message: String) -> anyhow::Result<i64> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let created_at = chrono::Utc::now().timestamp();

    db_guard.execute(
        "INSERT INTO tbl_friend_requests (from_user_id, message, created_at) VALUES (?1, ?2, ?3);",
        rusqlite::params![from_user_id, message, created_at]
    )?;

    Ok(db_guard.last_insert_rowid())
}

pub fn delete_friend_request(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "DELETE FROM tbl_friend_requests WHERE id=?1;", 
        rusqlite::params![id]
    )?;

    Ok(())
}

pub fn fetch_friend_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<Friend> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, created_at FROM tbl_friends WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("A friend with id {id} was not found."));
    }

    let (id, user_id, created_at): (i64, i64, i64) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    Ok(
        Friend::new(
            id,
            user_id,
            created_at
        )
    )
}

pub fn fetch_friend_by_user_id(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<Friend> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, created_at FROM tbl_friends WHERE user_id=?1;")?;

    if !query.exists(rusqlite::params![user_id])? {
        return Err(anyhow::anyhow!("A friend with user_id {user_id} was not found."));
    }

    let (id, user_id, created_at): (i64, i64, i64) = query.query_row(rusqlite::params![user_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    Ok(
        Friend::new(
            id,
            user_id,
            created_at
        )
    )
}

pub fn fetch_all_friends(db: Arc<Mutex<Connection>>) -> anyhow::Result<Vec<Friend>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, created_at FROM tbl_friends;")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No friend data was found."));
    }

    let rows = query.query_map((), |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;

        Ok(
            Friend::new(
                row.0,
                row.1,
                row.2
            )
        )
    }).collect::<anyhow::Result<Vec<Friend>>>()
}

pub fn create_friend(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<i64> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let created_at = chrono::Utc::now().timestamp();

    db_guard.execute(
        "INSERT INTO tbl_friends (user_id, created_at) VALUES (?1, ?2);",
        rusqlite::params![user_id, created_at]
    )?;

    Ok(db_guard.last_insert_rowid())
}

pub fn delete_friend(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "DELETE FROM tbl_friends WHERE id=?1;", 
        rusqlite::params![id]
    )?;

    Ok(())
}

pub fn fetch_direct_message_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<DirectMessage> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, from_peer_id, to_peer_id, content, created_at, edited_at, read FROM tbl_direct_messages WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("A direct message with id {id} was not found."));
    }

    let (id, from_peer_id, to_peer_id, content, created_at, edited_at, read): (i64, String, String, String, i64, Option<i64>, bool) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
    })?;

    Ok(
        DirectMessage::new (
            id, 
            from_peer_id, 
            to_peer_id, 
            content, 
            created_at, 
            edited_at,
            read 
        )
    )
}

pub fn fetch_direct_messages_with_peer(db: Arc<Mutex<Connection>>, peer_id: String) -> anyhow::Result<Vec<DirectMessage>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, from_peer_id, to_peer_id, content, created_at, edited_at, read FROM tbl_direct_messages WHERE from_peer_id=?1 OR to_peer_id=?1;")?;

    if !query.exists(rusqlite::params![peer_id])? {
        return Err(anyhow::anyhow!("A direct message with user_id {peer_id} was not found."));
    }

    let rows = query.query_map(rusqlite::params![peer_id], |row| {
        Ok((
            row.get(0)?, 
            row.get(1)?, 
            row.get(2)?, 
            row.get(3)?, 
            row.get(4)?, 
            row.get(5)?, 
            row.get(6)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;

        Ok(DirectMessage::new(
            row.0, 
            row.1, 
            row.2, 
            row.3, 
            row.4, 
            row.5, 
            row.6
        ))
    }).collect::<anyhow::Result<Vec<DirectMessage>>>()
}

pub fn fetch_all_direct_messages(db: Arc<Mutex<Connection>>) -> anyhow::Result<Vec<DirectMessage>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, from_peer_id, to_peer_id, content, created_at, edited_at, read FROM tbl_direct_messages;")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No direct message data was found."));
    }

    let rows = query.query_map((), |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;

        Ok(
            DirectMessage::new(
                row.0,
                row.1,
                row.2,
                row.3,
                row.4,
                row.5,
                row.6
            )
        )
    }).collect::<anyhow::Result<Vec<DirectMessage>>>()
}

pub fn create_direct_message(db: Arc<Mutex<Connection>>, from_peer_id: String, to_peer_id: String, content: String) -> anyhow::Result<i64> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let created_at = chrono::Utc::now().timestamp();

    db_guard.execute(
        "INSERT INTO tbl_direct_messages (from_peer_id, to_peer_id, content, created_at) VALUES (?1, ?2, ?3, ?4);", 
        rusqlite::params![from_peer_id, to_peer_id, content, created_at]
    )?;
    
    Ok(db_guard.last_insert_rowid())
}

pub fn update_direct_message(db: Arc<Mutex<Connection>>, id: i64, content: String) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let edited_at = chrono::Utc::now().timestamp();

    db_guard.execute(
        "UPDATE tbl_direct_messages SET content=?1, edited_at=?2 WHERE id=?3;", 
        rusqlite::params![content, edited_at, id]
    )?;
    
    Ok(())
}

pub fn delete_direct_message(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "DELETE FROM tbl_direct_messages WHERE id=?1;",
        rusqlite::params![id]
    )?;

    Ok(())
}

pub fn fetch_post_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<Post> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, author_user_id, content, created_at, edited_at FROM tbl_posts WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("A post with id {id} was not found."));
    }

    let (id, author_user_id, content, created_at, edited_at): (i64, i64, String, i64, Option<i64>) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    })?;

    Ok(
        Post::new(
            id,
            author_user_id,
            content,
            created_at,
            edited_at
        )
    )
}

pub fn fetch_all_posts(db: Arc<Mutex<Connection>>) -> anyhow::Result<Vec<Post>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, author_user_id, content, created_at, edited_at FROM tbl_posts;")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No post data was found."));
    }

    let rows = query.query_map((), |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;

        Ok(
            Post::new(
                row.0,
                row.1,
                row.2,
                row.3,
                row.4
            )
        )
    }).collect::<anyhow::Result<Vec<Post>>>()
}

pub fn fetch_posts_from_user(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<Vec<Post>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, author_user_id, content, created_at, edited_at FROM tbl_posts WHERE author_user_id=?1;")?;

    if !query.exists(rusqlite::params![user_id])? {
        return Err(anyhow::anyhow!("No posts were found from user {user_id}."));
    }

    let rows = query.query_map(rusqlite::params![user_id], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;

        Ok(
            Post::new(
                row.0,
                row.1,
                row.2,
                row.3,
                row.4
            )
        )
    }).collect::<anyhow::Result<Vec<Post>>>()
}

pub fn create_post(db: Arc<Mutex<Connection>>, author_user_id: i64, content: String) -> anyhow::Result<i64> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let created_at = chrono::Utc::now().timestamp();

    db_guard.execute(
        "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);", 
        rusqlite::params![author_user_id, content, created_at]
    )?;

    Ok(db_guard.last_insert_rowid())
}

pub fn update_post(db: Arc<Mutex<Connection>>, id: i64, content: String) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let edited_at = chrono::Utc::now().timestamp();

    db_guard.execute(
        "UPDATE tbl_posts SET content=?1, edited_at=?2 WHERE id=?3;", 
        rusqlite::params![content, edited_at, id]
    )?;

    Ok(())
}

pub fn delete_post(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "DELETE FROM tbl_posts WHERE id=?1;", 
        rusqlite::params![id]
    )?;

    Ok(())
}

pub fn fetch_blocked_users(db: Arc<Mutex<Connection>>) -> anyhow::Result<Vec<BlockedUser>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, blocked_at FROM tbl_blocked_users;")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No blocked user data was found."));
    }

    let rows = query.query_map((), |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;
        
        Ok(BlockedUser::new(
            row.0,
            row.1,
            row.2
        ))
    }).collect::<anyhow::Result<Vec<BlockedUser>>>()

}

pub fn fetch_blocked_user_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<BlockedUser> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, blocked_at FROM tbl_blocked_users WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("A blocked user with id {id} was not found."));
    }

    let (id, user_id, blocked_at) = query.query_row(rusqlite::params![id], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?
        ))
    })?;

    Ok(BlockedUser::new(
        id,
        user_id,
        blocked_at
    ))
}

pub fn fetch_blocked_user_by_user_id(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<BlockedUser> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, blocked_at FROM tbl_blocked_users WHERE user_id=?1;")?;

    if !query.exists(rusqlite::params![user_id])? {
        return Err(anyhow::anyhow!("A blocked user with user_id {user_id} was not found."));
    }

    let (id, user_id, blocked_at) = query.query_row(rusqlite::params![user_id], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?
        ))
    })?;

    Ok(BlockedUser::new(
        id,
        user_id,
        blocked_at
    ))
}

pub fn is_user_blocked(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<bool> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, blocked_at FROM tbl_blocked_users WHERE user_id=?1;")?;

    query.exists(rusqlite::params![user_id])
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

pub fn create_blocked_user(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<i64> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let blocked_at = chrono::Utc::now().timestamp();

    db_guard.execute(
        "INSERT INTO tbl_blocked_users (user_id, blocked_at) VALUES (?1, ?2);", 
        rusqlite::params![user_id, blocked_at]
    )?;

    Ok(db_guard.last_insert_rowid())
}

pub fn delete_blocked_user(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "DELETE FROM tbl_blocked_users WHERE id=?1;",
        rusqlite::params![id]
    )?;

    Ok(())
}

#[cfg(test)]
pub mod test {

    use rusqlite::params;

    use super::*;

    #[test]
    pub fn test_fetch_identity_errors_no_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let result = fetch_identity(db);

        assert!(result.is_err(), "expected error when no identity exists");
    }

    #[test]
    pub fn test_fetch_identity_correctly_fetches_identity_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        {
            let db_guard = db.lock().unwrap();

            db_guard.execute(
                "INSERT INTO tbl_identity (id, keypair, peer_id, port_number, created_at) VALUES (?1, ?2, ?3, ?4, ?5);",
                rusqlite::params![1i64, vec![1u8, 2, 3, 4], "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK", 5555, 0]
            ).expect("insert identity failed");
        }

        let identity = fetch_identity(db).expect("fetch_identity failed");

        assert_eq!(identity.id, 1);
        assert_eq!(identity.keypair, vec![1u8, 2, 3, 4]);
        assert_eq!(identity.peer_id.to_string(), "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK");
        assert_eq!(identity.port_number, 5555);
        assert_eq!(identity.created_at, 0);
    }

    #[test]
    pub fn test_create_identity_fails_when_identity_already_exists() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let keypair1 = vec![1u8, 2, 3];

        create_identity(db.clone(), keypair1, "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".into(), 5555)
            .expect("first create_identity failed");

        let keypair2 = vec![9u8, 8, 7];

        let second_result = create_identity(db.clone(), keypair2, "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".into(), 5555);

        assert!(second_result.is_err(), "expected create_identity to fail on second insert");
    }

    #[test]
    pub fn test_create_identity_correctly_inserts_identity_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let keypair = vec![10u8, 20, 30, 40];

        let result = create_identity(db.clone(), keypair.clone(), "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".into(), 5555);

        assert!(result.is_ok(), "create_identity failed");

        let identity = fetch_identity(db).expect("fetch_identity failed");

        assert_eq!(identity.id, 1);
        assert_eq!(identity.keypair, keypair);
        assert_eq!(identity.peer_id, "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string());
        assert_eq!(identity.port_number, 5555);
    }

    #[test]
    pub fn test_fetch_user_by_id_errors_invalid_id() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let result = fetch_user_by_id(db, 999);

        assert!(result.is_err(), "expected error when fetching non-existent user id");
    }

    #[test]
    pub fn test_fetch_user_by_id_correctly_fetches_user_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        let user_id = {
            let conn = db.lock().unwrap();

            conn.execute(
                "INSERT INTO tbl_users (peer_id, multiaddr, is_identity, created_at) VALUES (?1, ?2, ?3, ?4);",
                rusqlite::params![peer_id, multiaddr, false, 0]
            ).expect("insert user failed");

            conn.last_insert_rowid()
        };

        let user = fetch_user_by_id(db, user_id).expect("fetch_user_by_id failed");

        assert_eq!(user.id, user_id);
        assert_eq!(user.peer_id, peer_id);
        assert_eq!(user.multiaddr, multiaddr);
        assert_eq!(user.nickname, None);
        assert_eq!(user.is_identity, false);
        assert_eq!(user.created_at, 0)
    }

    #[test]
    pub fn test_fetch_user_by_peer_id_errors_invalid_peer_id() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        let result = fetch_user_by_peer_id(db, peer_id);

        assert!(result.is_err(), "expected error when fetching non-existent peer_id");
    }

    #[test]
    pub fn test_fetch_user_by_peer_id_correctly_fetches_user_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        let user_id = {
            let conn = db.lock().unwrap();

            conn.execute(
                "INSERT INTO tbl_users (peer_id, multiaddr, is_identity, created_at) VALUES (?1, ?2, ?3, ?4);",
                rusqlite::params![peer_id.clone(), multiaddr.clone(), false, 0]
            ).expect("insert user failed");

            conn.last_insert_rowid()
        };

        let user = fetch_user_by_peer_id(db, peer_id.clone()).expect("fetch_user_by_peer_id failed");

        assert_eq!(user.id, user_id);
        assert_eq!(user.peer_id, peer_id);
        assert_eq!(user.multiaddr, multiaddr);
        assert_eq!(user.nickname, None);
        assert_eq!(user.is_identity, false);
        assert_eq!(user.created_at, 0)

    }

    #[test]
    pub fn test_fetch_all_users_errors_no_friend_request_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_all_users(db.clone());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No user data was found"));
    }
    
    #[test]
    pub fn test_fetch_all_users_correctly_fetches_all_user_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id_1 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr_1 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let peer_id_2 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();
        let multiaddr_2 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();

        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tbl_users (peer_id, multiaddr, is_identity, created_at) VALUES (?1, ?2, ?3, ?4);",
            rusqlite::params![peer_id_1, multiaddr_1, false, 0]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_users (peer_id, multiaddr, is_identity, created_at) VALUES (?1, ?2, ?3, ?4);",
            rusqlite::params![peer_id_2, multiaddr_2, false, 0]
        ).unwrap();
        drop(conn);

        let users = fetch_all_users(db.clone()).expect("fetch_all_users failed");

        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|u| u.peer_id == peer_id_1));
        assert!(users.iter().any(|u| u.peer_id == peer_id_2));
        assert!(users.iter().any(|u| u.multiaddr == multiaddr_1));
        assert!(users.iter().any(|u| u.multiaddr == multiaddr_2));
    }

    #[test]
    pub fn test_create_user_correctly_inserts_user_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false)
            .expect("create_user failed");

        let user = fetch_user_by_peer_id(db, peer_id.clone()).expect("fetch_user_by_peer_id failed");

        assert_eq!(user.peer_id, peer_id);
        assert_eq!(user.multiaddr, multiaddr);
        assert_eq!(user.is_identity, false);
        assert_eq!(user.nickname, None);
        assert!(user.created_at > 0);
    }

    #[test]
    pub fn test_update_user_correctly_updates_multiaddr_value() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let initial_addr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), initial_addr, false)
            .expect("create_user failed");

        let user = fetch_user_by_peer_id(db.clone(), peer_id.clone())
            .expect("fetch_user_by_peer_id failed");

        let updated_addr = "/ip4/192.168.1.10/tcp/9000/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        update_user(db.clone(), user.id, Some(updated_addr.clone()), None)
            .expect("update_user failed");

        let updated_user = fetch_user_by_id(db, user.id)
            .expect("fetch_user_by_id failed");

        assert_eq!(updated_user.multiaddr, updated_addr);
    }

    #[test]
    pub fn test_update_user_correctly_updates_nickname_value() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let initial_addr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), initial_addr, false)
            .expect("create_user failed");

        let user = fetch_user_by_peer_id(db.clone(), peer_id.clone())
            .expect("fetch_user_by_peer_id failed");

        update_user(db.clone(), user.id, None, Some("Test Nickname".into()))
            .expect("update_user failed");

        let updated_user = fetch_user_by_id(db, user.id)
            .expect("fetch_user_by_id failed");

        assert_eq!(updated_user.nickname, Some("Test Nickname".into()));
    }

    #[test]
    pub fn test_delete_user_correctly_deletes_user_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr, false)
            .expect("create_user failed");

        let user = fetch_user_by_peer_id(db.clone(), peer_id)
            .expect("fetch_user_by_peer_id failed");

        delete_user(db.clone(), user.id)
            .expect("delete_user failed");

        let result = fetch_user_by_id(db, user.id);

        assert!(result.is_err(), "expected error when fetching deleted user");
    }

    #[test]
    pub fn test_fetch_friend_request_by_id_errors_invalid_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_friend_request_by_id(db.clone(), 999);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"), "Unexpected error message");
    }

    #[test]
    pub fn test_fetch_friend_request_by_id_correctly_fetches_friend_request_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false)
            .unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        let friend_request_id: i64 = {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tbl_friend_requests (from_user_id, message, created_at) VALUES (?1, ?2, ?3);",
                params![user_id, "Test", 0]
            ).unwrap();
            conn.last_insert_rowid()
        };

        let friend_request = fetch_friend_request_by_id(db.clone(), friend_request_id)
            .expect("Friend request fetch failed");

        assert_eq!(friend_request.id, friend_request_id);
        assert_eq!(friend_request.from_user_id, user_id);
    }

    #[test]
    pub fn test_fetch_friend_request_by_user_id_errors_invalid_user_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_friend_request_by_from_user_id(db.clone(), 999);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"), "Unexpected error message");
    }

    #[test]
    pub fn test_fetch_friend_request_by_user_id_correctly_fetches_friend_request_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false)
            .unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        let friend_request_id: i64 = {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tbl_friend_requests (from_user_id, message, created_at) VALUES (?1, ?2, ?3);",
                params![user_id, false, 0]
            ).unwrap();
            conn.last_insert_rowid()
        };

        let friend_request = fetch_friend_request_by_from_user_id(db.clone(), user_id)
            .expect("Friend fetch failed");

        assert_eq!(friend_request.id, friend_request_id);
        assert_eq!(friend_request.from_user_id, user_id);
    }

    #[test]
    pub fn test_fetch_all_friend_requests_errors_no_friend_request_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_all_friend_requests(db.clone());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No friend request data was found"));
    }
    
    #[test]
    pub fn test_fetch_all_friend_requests_correctly_fetches_all_friend_request_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tbl_friend_requests (from_user_id, message, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "Message 1".to_string(), 0]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_friend_requests (from_user_id, message, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "Message 2".to_string(), 0]
        ).unwrap();
        drop(conn);

        let friend_requests = fetch_all_friend_requests(db.clone()).expect("fetch_all_friend_requests failed");

        assert_eq!(friend_requests.len(), 2);
        assert!(friend_requests.iter().any(|fr| fr.message == "Message 1".to_string()));
        assert!(friend_requests.iter().any(|fr| fr.message == "Message 2".to_string()));
    }

    #[test]
    pub fn test_create_friend_request_correctly_inserts_friend_request_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false)
            .expect("User creation failed");

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        create_friend_request(db.clone(), user_id, "Message".to_string()).expect("create_friend_request failed");

        let (stored_id, stored_from_user_id, stored_message): (i64, i64, String) = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id, from_user_id, message FROM tbl_friend_requests LIMIT 1;",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            ).unwrap()
        };

        assert_eq!(stored_from_user_id, user_id);
        assert_eq!(stored_message, "Message".to_string());
        assert!(stored_id > 0, "Friend request id should be greater than 0");
    }

    #[test]
    pub fn test_delete_friend_request_correctly_deletes_friend_request_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_friend_request(db.clone(), user_id, "Message".to_string()).unwrap();

        let friend_request_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_friend_requests LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        delete_friend_request(db.clone(), friend_request_id).expect("delete_friend_request failed");

        let remaining_count: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM tbl_friend_requests;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        assert_eq!(remaining_count, 0, "Friend request table should be empty after deletion");
    }

    #[test]
    pub fn test_fetch_friend_by_id_errors_invalid_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_friend_by_id(db.clone(), 999);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"), "Unexpected error message");
    }

    #[test]
    pub fn test_fetch_friend_by_id_correctly_fetches_friend_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false)
            .unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        let friend_id: i64 = {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tbl_friends (user_id, created_at) VALUES (?1, ?2);",
                [user_id, 0]
            ).unwrap();
            conn.last_insert_rowid()
        };

        let friend = fetch_friend_by_id(db.clone(), friend_id)
            .expect("Friend fetch failed");

        assert_eq!(friend.id, friend_id);
        assert_eq!(friend.user_id, user_id);
    }

    #[test]
    pub fn test_fetch_friend_by_user_id_errors_invalid_user_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_friend_by_user_id(db.clone(), 999);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"), "Unexpected error message");
    }

    #[test]
    pub fn test_fetch_friend_by_user_id_correctly_fetches_friend_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false)
            .unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        let friend_id: i64 = {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tbl_friends (user_id, created_at) VALUES (?1, ?2);",
                [user_id, 0]
            ).unwrap();
            conn.last_insert_rowid()
        };

        let friend = fetch_friend_by_user_id(db.clone(), user_id)
            .expect("Friend fetch failed");

        assert_eq!(friend.id, friend_id);
        assert_eq!(friend.user_id, user_id);
    }

    #[test]
    pub fn test_fetch_all_friends_errors_no_friend_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_all_friends(db.clone());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No friend data was found"));
    }
    
    #[test]
    pub fn test_fetch_all_friends_correctly_fetches_all_friend_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id_1 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr_1 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let peer_id_2 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();
        let multiaddr_2 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();

        create_user(db.clone(), peer_id_1.clone(), multiaddr_1.clone(), false).unwrap();
        create_user(db.clone(), peer_id_2.clone(), multiaddr_2.clone(), false).unwrap();

        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tbl_friends (user_id, created_at) VALUES (1, 0);",
            ()
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_friends (user_id, created_at) VALUES (2, 0);",
            ()
        ).unwrap();
        drop(conn);

        let friends = fetch_all_friends(db.clone()).expect("fetch_all_friends failed");

        assert_eq!(friends.len(), 2);
        assert_eq!(friends[0].user_id, 1);
        assert_eq!(friends[1].user_id, 2);
    }

    #[test]
    pub fn test_create_friend_correctly_inserts_friend_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false)
            .expect("User creation failed");

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        create_friend(db.clone(), user_id).expect("create_friend failed");

        let (stored_id, stored_user_id): (i64, i64) = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id, user_id FROM tbl_friends LIMIT 1;",
                [],
                |r| Ok((r.get(0)?, r.get(1)?))
            ).unwrap()
        };

        assert_eq!(stored_user_id, user_id);
        assert!(stored_id > 0, "Friend id should be greater than 0");
    }

    #[test]
    pub fn test_delete_friend_correctly_deletes_friend_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_friend(db.clone(), user_id).unwrap();

        let friend_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_friends LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        delete_friend(db.clone(), friend_id).expect("delete_friend failed");

        let remaining_count: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM tbl_friends;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        assert_eq!(remaining_count, 0, "Friend table should be empty after deletion");
    }

    #[test]
    pub fn test_fetch_direct_message_by_id_errors_invalid_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_direct_message_by_id(db.clone(), 999);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"), "Unexpected error message");
    }

    #[test]
    pub fn test_fetch_direct_message_by_id_correctly_fetches_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();
        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let dm_id: i64 = {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tbl_direct_messages (from_peer_id, to_peer_id, content, created_at, read) VALUES (?1, ?2, ?3, ?4, ?5);",
                rusqlite::params![1, 2, "Hello", 0, false],
            ).unwrap();
            conn.last_insert_rowid()
        };

        let dm = fetch_direct_message_by_id(db.clone(), dm_id).expect("fetch failed");

        assert_eq!(dm.id, dm_id);
        assert_eq!(dm.from_peer_id, "2D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string());
        assert_eq!(dm.to_peer_id, "2D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string());
        assert_eq!(dm.content, "Hello");
        assert_eq!(dm.read, false);
        assert_eq!(dm.created_at, 0);
        assert!(dm.edited_at.is_none());
    }

    #[test]
    pub fn test_fetch_direct_messages_with_user_errors_invalid_user_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_direct_messages_with_peer(db.clone(), "Invalid peer id".into());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"), "Unexpected error message");
    }

    #[test]
    pub fn test_fetch_direct_messages_with_user_correctly_fetches_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id_1 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr_1 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let peer_id_2 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();
        let multiaddr_2 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();

        create_user(db.clone(), peer_id_1.clone(), multiaddr_1.clone(), false).unwrap();
        create_user(db.clone(), peer_id_2.clone(), multiaddr_2.clone(), false).unwrap();

        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tbl_direct_messages (from_peer_id, to_peer_id, content, created_at, read) VALUES (?1, ?2, ?3, ?4, ?5);",
            rusqlite::params![peer_id_1, peer_id_2, "Hello 1", 0, false]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_direct_messages (from_peer_id, to_peer_id, content, created_at, read) VALUES (?1, ?2, ?3, ?4, ?5);",
            rusqlite::params![peer_id_2, peer_id_1, "Hello 2", 0, false]
        ).unwrap();
        drop(conn);

        let dms = fetch_direct_messages_with_peer(db.clone(), peer_id_1).expect("fetch failed");

        assert_eq!(dms.len(), 2);
        assert!(dms.iter().any(|dm| dm.content == "Hello 1"));
        assert!(dms.iter().any(|dm| dm.content == "Hello 2"));
    }

    #[test]
    pub fn test_fetch_all_direct_messages_errors_no_post_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_all_direct_messages(db.clone());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No direct message data was found"));
    }

    #[test]
    pub fn test_fetch_all_direct_messages_correctly_fetches_all_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id_1 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr_1 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let peer_id_2 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();
        let multiaddr_2 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();

        create_user(db.clone(), peer_id_1.clone(), multiaddr_1.clone(), false).unwrap();
        create_user(db.clone(), peer_id_2.clone(), multiaddr_2.clone(), false).unwrap();

        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tbl_direct_messages (from_peer_id, to_peer_id, content, created_at) VALUES (?1, ?2, ?3, ?4);",
            rusqlite::params![peer_id_1.clone(), peer_id_2.clone(), "Direct message 1", 0]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_direct_messages (from_peer_id, to_peer_id, content, created_at) VALUES (?1, ?2, ?3, ?4);",
            rusqlite::params![peer_id_2.clone(), peer_id_1.clone(), "Direct message 2", 0]
        ).unwrap();
        drop(conn);

        let direct_messages = fetch_all_direct_messages(db.clone()).expect("fetch_all_direct_messages failed");

        assert_eq!(direct_messages.len(), 2);
        assert!(direct_messages.iter().any(|dm| dm.content == "Direct message 1"));
        assert!(direct_messages.iter().any(|dm| dm.content == "Direct message 2"));
    }

    #[test]
    pub fn test_create_direct_message_correctly_inserts_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id_1 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr_1 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let peer_id_2 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();
        let multiaddr_2 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();

        create_user(db.clone(), peer_id_1.clone(), multiaddr_1.clone(), false).unwrap();
        create_user(db.clone(), peer_id_2.clone(), multiaddr_2.clone(), false).unwrap();

        create_direct_message(db.clone(), peer_id_1.clone(), peer_id_2.clone(), "Hello DM".to_string())
            .expect("create_direct_message failed");

        let (dm_id, dm_from_peer_id, dm_to_peer_id, dm_content): (i64, String, String, String) = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id, from_peer_id, to_peer_id, content FROM tbl_direct_messages LIMIT 1;",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            ).unwrap()
        };

        assert_eq!(dm_from_peer_id, peer_id_1);
        assert_eq!(dm_to_peer_id, peer_id_2);
        assert_eq!(dm_content, "Hello DM");
        assert!(dm_id > 0);
    }

    #[test]
    pub fn test_update_direct_message_correctly_updates_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id_1 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr_1 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let peer_id_2 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();
        let multiaddr_2 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();

        create_user(db.clone(), peer_id_1.clone(), multiaddr_1.clone(), false).unwrap();
        create_user(db.clone(), peer_id_2.clone(), multiaddr_2.clone(), false).unwrap();

        let dm_id = create_direct_message(db.clone(), peer_id_1, peer_id_2, "Original Content".to_string()).unwrap();

        update_direct_message(db.clone(), dm_id, "Updated Content".to_string()).unwrap();

        let updated_content: String = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT content FROM tbl_direct_messages WHERE id=?1;",
                [dm_id],
                |r| r.get(0)
            ).unwrap()
        };

        assert_eq!(updated_content, "Updated Content");
    }

    #[test]
    pub fn test_delete_direct_message_correctly_deletes_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id_1 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr_1 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let peer_id_2 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();
        let multiaddr_2 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();

        create_user(db.clone(), peer_id_1.clone(), multiaddr_1.clone(), false).unwrap();
        create_user(db.clone(), peer_id_2.clone(), multiaddr_2.clone(), false).unwrap();

        let dm_id = create_direct_message(db.clone(), peer_id_1.clone(), peer_id_2.clone(), "To Be Deleted".to_string()).unwrap();

        delete_direct_message(db.clone(), dm_id).unwrap();

        let count: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM tbl_direct_messages;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        assert_eq!(count, 0, "Direct message table should be empty after deletion");
    }

    #[test]
    pub fn test_fetch_post_by_id_errors_invalid_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_post_by_id(db.clone(), 999);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"), "Unexpected error message");
    }

    #[test]
    pub fn test_fetch_post_by_id_correctly_fetches_post() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let post_id: i64 = {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
                rusqlite::params![user_id, "My first post", 0]
            ).unwrap();
            conn.last_insert_rowid()
        };

        let post = fetch_post_by_id(db.clone(), post_id).expect("fetch_post_by_id failed");

        assert_eq!(post.id, post_id);
        assert_eq!(post.author_user_id, user_id);
        assert_eq!(post.content, "My first post");
        assert_eq!(post.created_at, 0);
        assert!(post.edited_at.is_none());
    }

    #[test]
    pub fn test_fetch_all_posts_errors_no_post_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_all_posts(db.clone());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No post data was found"));
    }

    #[test]
    pub fn test_fetch_all_posts_correctly_fetches_all_post_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "Post 1", 0]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "Post 2", 0]
        ).unwrap();
        drop(conn);

        let posts = fetch_all_posts(db.clone()).expect("fetch_all_posts failed");

        assert_eq!(posts.len(), 2);
        assert!(posts.iter().any(|p| p.content == "Post 1"));
        assert!(posts.iter().any(|p| p.content == "Post 2"));
    }

    #[test]
    pub fn test_fetch_posts_from_user_errors_invalid_user_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_posts_from_user(db.clone(), 999);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No posts were found from user"));
    }

    #[test]
    pub fn test_fetch_posts_from_user_correctly_fetches_post_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "User Post 1", 0]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "User Post 2", 0]
        ).unwrap();
        drop(conn);

        let posts = fetch_posts_from_user(db.clone(), user_id).expect("fetch_posts_from_user failed");

        assert_eq!(posts.len(), 2);
        assert!(posts.iter().any(|p| p.content == "User Post 1"));
        assert!(posts.iter().any(|p| p.content == "User Post 2"));
    }

    #[test]
    pub fn test_create_post_correctly_inserts_post_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.to_string(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_post(db.clone(), user_id, "Hello World".to_string()).unwrap();

        let post_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_posts LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let post = fetch_post_by_id(db.clone(), post_id).expect("Failed to fetch post");
        assert_eq!(post.content, "Hello World");
        assert_eq!(post.author_user_id, user_id);
        assert!(post.created_at > 0);
        assert!(post.edited_at.is_none());
    }

    #[test]
    pub fn test_update_post_correctly_updates_post_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.to_string(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_post(db.clone(), user_id, "Original Content".to_string()).unwrap();

        let post_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_posts LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        update_post(db.clone(), post_id, "Updated Content".to_string()).unwrap();

        let post = fetch_post_by_id(db.clone(), post_id).expect("Failed to fetch post after update");
        assert_eq!(post.content, "Updated Content");
        assert!(post.edited_at.is_some());
    }

    #[test]
    pub fn test_delete_post_correctly_deletes_post_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_post(db.clone(), user_id, "To be deleted".to_string()).unwrap();

        let post_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_posts LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        delete_post(db.clone(), post_id).unwrap();

        let result = fetch_post_by_id(db.clone(), post_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"));
    }

    #[test]
    pub fn test_fetch_blocked_users_errors_no_blocked_user_data() {
        let db = init_db(":memory:".into()).unwrap();

        let result = fetch_blocked_users(db.clone());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No blocked user data was found"));
    }

    #[test]
    pub fn test_fetch_blocked_users_correctly_fetches_all_blocked_user_data() {
        let db = init_db(":memory:".into()).unwrap();

        let peer_id_1 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let peer_id_2 = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();
        
        let multiaddr_1 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr_2 = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsA".to_string();

        create_user(db.clone(), peer_id_1.clone(), multiaddr_1, false).unwrap();
        create_user(db.clone(), peer_id_2.clone(), multiaddr_2, false).unwrap();

        let user_ids: Vec<i64> = {
            let conn = db.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id FROM tbl_users;").unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap()
                .map(|id| id.unwrap())
                .collect()
        };

        for id in &user_ids {
            db.lock().unwrap().execute(
                "INSERT INTO tbl_blocked_users (user_id, blocked_at) VALUES (?1, 0);",
                rusqlite::params![id]
            ).unwrap();
        }

        let blocked_users = fetch_blocked_users(db.clone()).unwrap();
        assert_eq!(blocked_users.len(), user_ids.len());
        for bu in blocked_users {
            assert!(user_ids.contains(&bu.user_id));
        }
    }

    #[test]
    pub fn test_fetch_blocked_user_by_id_errors_invalid_id() {
        let db = init_db(":memory:".into()).unwrap();

        let result = fetch_blocked_user_by_id(db.clone(), 999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"));
    }

    #[test]
    pub fn test_fetch_blocked_user_by_id_correctly_fetches_blocked_user_data() {
        let db = init_db(":memory:".into()).unwrap();

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let db_guard = db.lock().unwrap();
        let user_id: i64 = db_guard.query_row(
            "SELECT id FROM tbl_users LIMIT 1;", 
            [], 
            |r| r.get(0)
        ).unwrap();
        db_guard.execute(
            "INSERT INTO tbl_blocked_users (user_id, blocked_at) VALUES (?1, 0);", 
            rusqlite::params![user_id]
        ).unwrap();
        
        let blocked_user_id: i64 = db_guard.query_row(
            "SELECT id FROM tbl_blocked_users LIMIT 1;", 
            [], 
            |r| r.get(0)
        ).unwrap();

        drop(db_guard);

        let blocked_user = fetch_blocked_user_by_id(db.clone(), blocked_user_id).unwrap();

        assert_eq!(blocked_user.id, blocked_user_id);
        assert_eq!(blocked_user.user_id, user_id);
    }


    #[test]
    pub fn test_fetch_blocked_user_by_user_id_errors_invalid_user_id() {
        let db = init_db(":memory:".into()).unwrap();

        let result = fetch_blocked_user_by_user_id(db.clone(), 999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"));
    }

    #[test]
    pub fn test_fetch_blocked_user_by_user_id_correctly_fetches_blocked_user_data() {
        let db = init_db(":memory:".into()).unwrap();

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string(); 
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();   
    
        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();
        
        let db_guard = db.lock().unwrap();
        let user_id: i64 = db_guard.query_row(
            "SELECT id FROM tbl_users LIMIT 1;", 
            [], 
            |r| r.get(0)
        ).unwrap();
        db_guard.execute(
            "INSERT INTO tbl_blocked_users (user_id, blocked_at) VALUES (?1, 0);", 
            rusqlite::params![user_id]
        ).unwrap();

        drop(db_guard);

        let blocked_user = fetch_blocked_user_by_user_id(db.clone(), user_id).unwrap();
        assert_eq!(blocked_user.user_id, user_id);
    }

    #[test]
    pub fn test_is_user_blocked_correctly_returns_true() {
        let db = init_db(":memory:".into()).unwrap();

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();
        
        let db_guard = db.lock().unwrap();

        let user_id: i64 = db_guard.query_row(
            "SELECT id FROM tbl_users LIMIT 1;", 
            [], 
            |r| r.get(0)
        ).unwrap();
        db_guard.execute(
            "INSERT INTO tbl_blocked_users (user_id, blocked_at) VALUES (?1, 0);", 
            rusqlite::params![user_id]
        ).unwrap();

        drop(db_guard);

        let blocked = is_user_blocked(db.clone(), user_id).unwrap();
        assert!(blocked);
    }

    #[test]
    pub fn test_is_user_blocked_correctly_returns_false() {
        let db = init_db(":memory:".into()).unwrap();

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();
        
        let db_guard = db.lock().unwrap();

        let user_id: i64 = db_guard.query_row(
            "SELECT id FROM tbl_users LIMIT 1;", 
            [], 
            |r| r.get(0)
        ).unwrap();

        drop(db_guard);

        let blocked = is_user_blocked(db.clone(), user_id).unwrap();
        assert!(!blocked);
    }

    #[test]
    pub fn test_create_blocked_user_correctly_inserts_blocked_user_data() {
        let db = init_db(":memory:".into()).unwrap();

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [],
                |row| row.get(0)
            ).unwrap()
        };

        let result = create_blocked_user(db.clone(), user_id);
        assert!(result.is_ok());

        let (count, stored_user_id): (i64, i64) = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT COUNT(*), user_id FROM tbl_blocked_users;",
                [],
                |row| Ok((row.get(0)?, row.get(1)?))
            ).unwrap()
        };

        assert_eq!(count, 1);
        assert_eq!(stored_user_id, user_id);
    }

    #[test]
    pub fn test_delete_blocked_user_correctly_deletes_blocked_user_data() {
        let db = init_db(":memory:".into()).unwrap();

        let peer_id = "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();
        let multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".to_string();

        create_user(db.clone(), peer_id.clone(), multiaddr.clone(), false).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [],
                |row| row.get(0)
            ).unwrap()
        };

        create_blocked_user(db.clone(), user_id).unwrap();

        let blocked_user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_blocked_users LIMIT 1;",
                [],
                |row| row.get(0)
            ).unwrap()
        };

        let result = delete_blocked_user(db.clone(), blocked_user_id);
        assert!(result.is_ok());

        let count: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM tbl_blocked_users;",
                [],
                |row| row.get(0)
            ).unwrap()
        };

        assert_eq!(count, 0);
    }
}