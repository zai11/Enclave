use std::{str::FromStr, sync::{Arc, Mutex}};

use chrono::{DateTime, Utc};
use libp2p::{Multiaddr, PeerId};
use rusqlite::Connection;

use crate::db::models::{blocked_user::BlockedUser, direct_message::DirectMessage, friend::Friend, identity::Identity, nickname::Nickname, post::Post, user::User};

pub mod models;

pub fn init_db(path: String) -> anyhow::Result<Arc<Mutex<Connection>>> {
    log::info!("Initilising database...");

    let db = Connection::open(path)?;
    log::info!("Created enclave database.");

    db.execute("PRAGMA foreign_keys = ON", ())?;

    if !db.table_exists(None, "tbl_identity")? {
        db.execute("CREATE TABLE tbl_identity (
                            id INTEGER PRIMARY KEY CHECK (id=1),
                            keypair BLOB NOT NULL,
                            peer_id TEXT NOT NULL,
                            created_at TEXT NOT NULL
                        );", ())?;
        log::info!("Created identity table.");
    }

    if !db.table_exists(None, "tbl_users")? {
        db.execute("CREATE TABLE tbl_users (
                            id INTEGER PRIMARY KEY,
                            peer_id TEXT NOT NULL,
                            multiaddr TEXT NOT NULL
                        );", ())?;
        log::info!("Created connections table.");
    }
    
    if !db.table_exists(None, "tbl_nicknames")? {
        db.execute("CREATE TABLE tbl_nicknames (
                            id INTEGER PRIMARY KEY,
                            user_id INTEGER NOT NULL,
                            nickname TEXT,
                            FOREIGN KEY (user_id) REFERENCES tbl_users(id)
                        );", ())?;
        log::info!("Created nicknames table.");
    }

    if !db.table_exists(None, "tbl_friends")? {
        db.execute("CREATE TABLE tbl_friends (
                            id INTEGER PRIMARY KEY,
                            user_id INTEGER NOT NULL,
                            FOREIGN KEY (user_id) REFERENCES tbl_users(id)
                        );", ())?;
        log::info!("Created friends table.");
    }

    if !db.table_exists(None, "tbl_direct_messages")? {
        db.execute("CREATE TABLE tbl_direct_messages (
                            id INTEGER PRIMARY KEY,
                            user_id INTEGER NOT NULL,
                            from_me BOOLEAN NOT NULL,
                            content TEXT NOT NULL,
                            created_at TEXT NOT NULL,
                            edited_at TEXT,
                            read BOOLEAN DEFAULT 0,
                            FOREIGN KEY (user_id) REFERENCES tbl_users(id)
                        );", ())?;
        log::info!("Created direct messages table.");
    }

    if !db.table_exists(None, "tbl_posts")? {
        db.execute("CREATE TABLE tbl_posts (
                            id INTEGER PRIMARY KEY,
                            author_user_id INTEGER NOT NULL,
                            content TEXT NOT NULL,
                            created_at TEXT NOT NULL,
                            edited_at TEXT,
                            FOREIGN KEY (author_user_id) REFERENCES tbl_users(id)
                        );", ())?;
        log::info!("Created posts table.");
    }

    if !db.table_exists(None, "tbl_blocked_users")? {
        db.execute("CREATE TABLE tbl_blocked_users (
                            id INTEGER PRIMARY KEY,
                            user_id INTEGER NOT NULL,
                            FOREIGN KEY (user_id) REFERENCES tbl_users(id)
                        );", ())?;
        log::info!("Created blocked users table.");
    }

    Ok(Arc::new(Mutex::new(db)))
}

pub fn fetch_identity(db: Arc<Mutex<Connection>>) -> anyhow::Result<Identity> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, keypair, peer_id, created_at FROM tbl_identity")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No identity data was found."));
    }

    let (id, keypair, peer_id, created_at): (i64, Vec<u8>, String, String) = query.query_row((), |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;

    Ok(
        Identity::new(
            id, 
            keypair, 
            PeerId::from_str(&peer_id)?, 
            DateTime::parse_from_rfc3339(&created_at)?.to_utc()
        )
    )
}

pub fn create_identity(db: Arc<Mutex<Connection>>, keypair: Vec<u8>, peer_id: PeerId) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let created_at = Utc::now();

    db_guard.execute(
        "INSERT INTO tbl_identity (keypair, peer_id, created_at) VALUES (?1, ?2, ?3)", 
        rusqlite::params![
            keypair,
            peer_id.to_string(),
            created_at.to_rfc3339()
        ]
    )?;

    Ok(())
}

pub fn fetch_user_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<User> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, peer_id, multiaddr FROM tbl_users WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("No user with the id {id} was found."));
    }
    
    let (id, peer_id, multiaddr): (i64, String, String) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    Ok(
        User::new(
            id, 
            PeerId::from_str(&peer_id)?, 
            Multiaddr::from_str(&multiaddr)?
        )
    )
}

pub fn fetch_user_by_peer_id(db: Arc<Mutex<Connection>>, peer_id: PeerId) -> anyhow::Result<User> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, peer_id, multiaddr FROM tbl_users WHERE peer_id=?1;")?;

    if !query.exists(rusqlite::params![peer_id.to_string()])? {
        return Err(anyhow::anyhow!("No user with the peer_id {peer_id} was found."));
    }

    let (id, peer_id, multiaddr): (i64, String, String) = query.query_row(rusqlite::params![peer_id.to_string()], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    Ok(
        User::new(
            id, 
            PeerId::from_str(&peer_id)?, 
            Multiaddr::from_str(&multiaddr)?
        )
    )
}

pub fn create_user(db: Arc<Mutex<Connection>>, peer_id: PeerId, multiaddr: Multiaddr) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "INSERT INTO tbl_users (peer_id, multiaddr) VALUES (?1, ?2)", 
        rusqlite::params![peer_id.to_string(), multiaddr.to_string()]
    )?;

    Ok(())
}

pub fn update_user(db: Arc<Mutex<Connection>>, id: i64, multiaddr: Multiaddr) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "UPDATE tbl_users SET multiaddr=?1 WHERE id=?2;",
        rusqlite::params![multiaddr.to_string(), id]
    )?;

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

pub fn fetch_nickname_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<Nickname> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, nickname FROM tbl_nicknames WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("No nickname with id {id} was found."));
    }

    let (id, user_id, nickname): (i64, i64, String) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    
    Ok(
        Nickname::new (
            id,
            user_id,
            nickname
        )
    )
}

pub fn fetch_nickname_by_user_id(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<Nickname> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, nickname FROM tbl_nicknames WHERE user_id=?1;")?;

    if !query.exists(rusqlite::params![user_id])? {
        return Err(anyhow::anyhow!("No nickname with user_id {user_id} was found."));
    }

    let (id, user_id, nickname): (i64, i64, String) = query.query_row(rusqlite::params![user_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    Ok(
        Nickname::new(
            id, 
            user_id, 
            nickname 
        )
    )
}

pub fn create_nickname(db: Arc<Mutex<Connection>>, user_id: i64, nickname: String) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "INSERT INTO tbl_nicknames (user_id, nickname) VALUES (?1, ?2)",
        rusqlite::params![user_id, nickname]
    )?;

    Ok(())
}

pub fn update_nickname(db: Arc<Mutex<Connection>>, id: i64, nickname: String) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "UPDATE tbl_nicknames SET nickname=?1 WHERE id=?2;",
        rusqlite::params![nickname, id]
    )?;

    Ok(())
}

pub fn delete_nickname(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "DELETE FROM tbl_nicknames WHERE id=?1;",
        rusqlite::params![id]
    )?;

    Ok(())
}

pub fn fetch_friend_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<Friend> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id FROM tbl_friends WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("A friend with id {id} was not found."));
    }

    let (id, user_id): (i64, i64) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;

    Ok(
        Friend::new(
            id,
            user_id
        )
    )
}

pub fn fetch_friend_by_user_id(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<Friend> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id FROM tbl_friends WHERE user_id=?1;")?;

    if !query.exists(rusqlite::params![user_id])? {
        return Err(anyhow::anyhow!("A friend with user_id {user_id} was not found."));
    }

    let (id, user_id): (i64, i64) = query.query_row(rusqlite::params![user_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;

    Ok(
        Friend::new(
            id,
            user_id
        )
    )
}

pub fn create_friend(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "INSERT INTO tbl_friends (user_id) VALUES (?1);",
        rusqlite::params![user_id]
    )?;

    Ok(())
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

    let mut query = db_guard.prepare("SELECT id, user_id, from_me, content, created_at, edited_at, read FROM tbl_direct_messages WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("A direct message with id {id} was not found."));
    }

    let (id, user_id, from_me, content, created_at, edited_at, read): (i64, i64, bool, String, String, String, bool) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
    })?;

    let created_at = DateTime::parse_from_rfc3339(&created_at)?.to_utc();
    let edited_at = if edited_at != "" {
        Some(DateTime::parse_from_rfc3339(&edited_at)?.to_utc())
    } else {
        None
    };

    Ok(
        DirectMessage::new (
            id, 
            user_id, 
            from_me, 
            content, 
            created_at, 
            edited_at,
            read 
        )
    )
}

pub fn fetch_direct_messages_with_user(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<Vec<DirectMessage>> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id, from_me, content, created_at, edited_at, read FROM tbl_direct_messages WHERE user_id=?1;")?;

    if !query.exists(rusqlite::params![user_id])? {
        return Err(anyhow::anyhow!("A direct message with user_id {user_id} was not found."));
    }

    let rows = query.query_map(rusqlite::params![user_id], |row| {
        Ok((
            row.get(0)?, 
            row.get(1)?, 
            row.get(2)?, 
            row.get(3)?, 
            row.get::<_, String>(4)?, 
            row.get::<_, String>(5)?, 
            row.get(6)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;

        let created_at = DateTime::parse_from_rfc3339(&row.4)?.to_utc();
        let edited_at = if row.5 != "" {
            Some(DateTime::parse_from_rfc3339(&row.5)?.to_utc())
        } else {
            None
        };

        Ok(DirectMessage::new(
            row.0, 
            row.1, 
            row.2, 
            row.3, 
            created_at, 
            edited_at, 
            row.6
        ))
    }).collect::<anyhow::Result<Vec<DirectMessage>>>()
}

pub fn create_direct_message(db: Arc<Mutex<Connection>>, user_id: i64, from_me: bool, content: String) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let created_at = Utc::now();

    db_guard.execute(
        "INSERT INTO tbl_direct_messages (user_id, from_me, content, created_at) VALUES (?1, ?2, ?3, ?4);", 
        rusqlite::params![user_id, from_me, content, created_at.to_rfc3339()]
    )?;
    
    Ok(())
}

pub fn update_direct_message(db: Arc<Mutex<Connection>>, id: i64, content: String) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let edited_at = Utc::now();

    db_guard.execute(
        "UPDATE tbl_direct_messages SET content=?1, edited_at=?2 WHERE id=?3;", 
        rusqlite::params![content, edited_at.to_rfc3339(), id]
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

    let (id, author_user_id, content, created_at, edited_at): (i64, i64, String, String, Option<String>) = query.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    })?;

    let created_at = DateTime::parse_from_rfc3339(&created_at)?.to_utc();
    let edited_at = if let Some(edited_at) = edited_at {
        Some(DateTime::parse_from_rfc3339(&edited_at)?.to_utc())
    } else {
        None
    };

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
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;
        
        let created_at = DateTime::parse_from_rfc3339(&row.3)?.to_utc();
        let edited_at = if let Some(edited_at_raw) = row.4 {
            Some(DateTime::parse_from_rfc3339(&edited_at_raw)?.to_utc())
        } else {
            None
        };

        Ok(
            Post::new(
                row.0,
                row.1,
                row.2,
                created_at,
                edited_at
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
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;
        
        let created_at = DateTime::parse_from_rfc3339(&row.3)?.to_utc();
        let edited_at = if let Some(edited_at_raw)  = row.4 {
            Some(DateTime::parse_from_rfc3339(&edited_at_raw)?.to_utc())
        } else {
            None
        };

        Ok(
            Post::new(
                row.0,
                row.1,
                row.2,
                created_at,
                edited_at
            )
        )
    }).collect::<anyhow::Result<Vec<Post>>>()
}

pub fn create_post(db: Arc<Mutex<Connection>>, author_user_id: i64, content: String) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let created_at = Utc::now().to_rfc3339();

    db_guard.execute(
        "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);", 
        rusqlite::params![author_user_id, content, created_at]
    )?;

    Ok(())
}

pub fn update_post(db: Arc<Mutex<Connection>>, id: i64, content: String) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let edited_at = Utc::now().to_rfc3339();

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

    let mut query = db_guard.prepare("SELECT id, user_id FROM tbl_blocked_users;")?;

    if !query.exists(())? {
        return Err(anyhow::anyhow!("No blocked user data was found."));
    }

    let rows = query.query_map((), |row| {
        Ok((
            row.get(0)?,
            row.get(1)?
        ))
    })?;

    rows.map(|row_result| {
        let row = row_result?;
        
        Ok(BlockedUser::new(
            row.0,
            row.1
        ))
    }).collect::<anyhow::Result<Vec<BlockedUser>>>()

}

pub fn fetch_blocked_user_by_id(db: Arc<Mutex<Connection>>, id: i64) -> anyhow::Result<BlockedUser> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id FROM tbl_blocked_users WHERE id=?1;")?;

    if !query.exists(rusqlite::params![id])? {
        return Err(anyhow::anyhow!("A blocked user with id {id} was not found."));
    }

    let (id, user_id) = query.query_row(rusqlite::params![id], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?
        ))
    })?;

    Ok(BlockedUser::new(
        id,
        user_id
    ))
}

pub fn fetch_blocked_user_by_user_id(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<BlockedUser> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id FROM tbl_blocked_users WHERE user_id=?1;")?;

    if !query.exists(rusqlite::params![user_id])? {
        return Err(anyhow::anyhow!("A blocked user with user_id {user_id} was not found."));
    }

    let (id, user_id) = query.query_row(rusqlite::params![user_id], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?
        ))
    })?;

    Ok(BlockedUser::new(
        id,
        user_id
    ))
}

pub fn is_user_blocked(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<bool> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    let mut query = db_guard.prepare("SELECT id, user_id FROM tbl_blocked_users WHERE user_id=?1;")?;

    query.exists(rusqlite::params![user_id])
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

pub fn create_blocked_user(db: Arc<Mutex<Connection>>, user_id: i64) -> anyhow::Result<()> {
    let db_guard = db.lock()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    db_guard.execute(
        "INSERT INTO tbl_blocked_users (user_id) VALUES (?1);", 
        rusqlite::params![user_id]
    )?;

    Ok(())
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
                "INSERT INTO tbl_identity (id, keypair, peer_id, created_at) VALUES (?1, ?2, ?3, ?4);",
                rusqlite::params![1i64, vec![1u8, 2, 3, 4], "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK", "2024-01-01T00:00:00Z"]
            ).expect("insert identity failed");
        }

        let identity = fetch_identity(db).expect("fetch_identity failed");

        assert_eq!(identity.id, 1);
        assert_eq!(identity.keypair, vec![1u8, 2, 3, 4]);
        assert_eq!(identity.peer_id.to_string(), "12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK");
        assert_eq!(identity.created_at.to_rfc3339(), "2024-01-01T00:00:00+00:00");
    }

    #[test]
    pub fn test_create_identity_fails_when_identity_already_exists() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let keypair1 = vec![1u8, 2, 3];
        let peer_id1 = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        create_identity(db.clone(), keypair1, peer_id1)
            .expect("first create_identity failed");

        let keypair2 = vec![9u8, 8, 7];
        let peer_id2 = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        let second_result = create_identity(db.clone(), keypair2, peer_id2);

        assert!(second_result.is_err(), "expected create_identity to fail on second insert");
    }

    #[test]
    pub fn test_create_identity_correctly_inserts_identity_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let keypair = vec![10u8, 20, 30, 40];
        let peer_id = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        let result = create_identity(db.clone(), keypair.clone(), peer_id.clone());

        assert!(result.is_ok(), "create_identity failed");

        let identity = fetch_identity(db).expect("fetch_identity failed");

        assert_eq!(identity.id, 1);
        assert_eq!(identity.keypair, keypair);
        assert_eq!(identity.peer_id, peer_id);
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

        let peer_id = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        let multiaddr = Multiaddr::from_str("/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid multiaddr");

        let user_id = {
            let conn = db.lock().unwrap();

            conn.execute(
                "INSERT INTO tbl_users (peer_id, multiaddr) VALUES (?1, ?2);",
                rusqlite::params![peer_id.to_string(), multiaddr.to_string()]
            ).expect("insert user failed");

            conn.last_insert_rowid()
        };

        let user = fetch_user_by_id(db, user_id).expect("fetch_user_by_id failed");

        assert_eq!(user.id, user_id);
        assert_eq!(user.peer_id, peer_id);
        assert_eq!(user.multiaddr, multiaddr);
    }

    #[test]
    pub fn test_fetch_user_by_peer_id_errors_invalid_peer_id() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        let result = fetch_user_by_peer_id(db, peer_id);

        assert!(result.is_err(), "expected error when fetching non-existent peer_id");
    }

    #[test]
    pub fn test_fetch_user_by_peer_id_correctly_fetches_user_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        let multiaddr = Multiaddr::from_str("/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid multiaddr");

        let user_id = {
            let conn = db.lock().unwrap();

            conn.execute(
                "INSERT INTO tbl_users (peer_id, multiaddr) VALUES (?1, ?2);",
                rusqlite::params![peer_id.to_string(), multiaddr.to_string()]
            ).expect("insert user failed");

            conn.last_insert_rowid()
        };

        let user = fetch_user_by_peer_id(db, peer_id).expect("fetch_user_by_peer_id failed");

        assert_eq!(user.id, user_id);
        assert_eq!(user.peer_id, peer_id);
        assert_eq!(user.multiaddr, multiaddr);
    }

    #[test]
    pub fn test_create_user_correctly_inserts_user_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        let multiaddr = Multiaddr::from_str("/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid multiaddr");

        create_user(db.clone(), peer_id.clone(), multiaddr.clone())
            .expect("create_user failed");

        let user = fetch_user_by_peer_id(db, peer_id).expect("fetch_user_by_peer_id failed");

        assert_eq!(user.peer_id, peer_id);
        assert_eq!(user.multiaddr, multiaddr);
    }

    #[test]
    pub fn test_update_user_correctly_updates_user_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        let initial_addr = Multiaddr::from_str("/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid multiaddr");

        create_user(db.clone(), peer_id.clone(), initial_addr)
            .expect("create_user failed");

        let user = fetch_user_by_peer_id(db.clone(), peer_id.clone())
            .expect("fetch_user_by_peer_id failed");

        let updated_addr = Multiaddr::from_str("/ip4/192.168.1.10/tcp/9000/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid updated multiaddr");

        update_user(db.clone(), user.id, updated_addr.clone())
            .expect("update_user failed");

        let updated_user = fetch_user_by_id(db, user.id)
            .expect("fetch_user_by_id failed");

        assert_eq!(updated_user.multiaddr, updated_addr);
    }


    #[test]
    pub fn test_delete_user_correctly_deletes_user_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let peer_id = PeerId::from_str("12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid peer id");

        let multiaddr = Multiaddr::from_str("/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK")
            .expect("invalid multiaddr");

        create_user(db.clone(), peer_id.clone(), multiaddr)
            .expect("create_user failed");

        let user = fetch_user_by_peer_id(db.clone(), peer_id)
            .expect("fetch_user_by_peer_id failed");

        delete_user(db.clone(), user.id)
            .expect("delete_user failed");

        let result = fetch_user_by_id(db, user.id);

        assert!(result.is_err(), "expected error when fetching deleted user");
    }

    #[test]
    pub fn test_fetch_nickname_by_id_errors_invalid_id() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let result = fetch_nickname_by_id(db, 999);

        assert!(result.is_err(), "expected error when fetching non-existent nickname id");
    }

    #[test]
    pub fn test_fetch_nickname_by_id_correctly_fetches_nickname_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let user_id = {
            let conn = db.lock().unwrap();

            conn.execute(
                "INSERT INTO tbl_users (peer_id, multiaddr) VALUES (?1, ?2)",
                rusqlite::params!["12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK", "/ip4/127.0.0.1/tcp/4001"]
            ).expect("insert user failed");

            conn.last_insert_rowid()
        };

        let nickname_id = {
            let conn = db.lock().unwrap();

            conn.execute(
                "INSERT INTO tbl_nicknames (user_id, nickname) VALUES (?1, ?2)",
                rusqlite::params![user_id, "Alice"]
            ).expect("insert nickname failed");

            conn.last_insert_rowid()
        };

        let nickname = fetch_nickname_by_id(db, nickname_id)
            .expect("fetch_nickname_by_id failed");

        assert_eq!(nickname.id, nickname_id);
        assert_eq!(nickname.user_id, user_id);
        assert_eq!(nickname.nickname, "Alice");
    }

    #[test]
    pub fn test_fetch_nickname_by_user_id_errors_invalid_user_id() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let result = fetch_nickname_by_user_id(db, 999);

        assert!(result.is_err(), "expected error when fetching nickname for non-existent user_id");
    }

    #[test]
    pub fn test_fetch_nickname_by_user_id_correctly_fetches_nickname_data() {
        let db = init_db(":memory:".into()).expect("db init failed");

        let user_id = {
            let conn = db.lock().unwrap();

            conn.execute("INSERT INTO tbl_users (peer_id, multiaddr) VALUES (?1, ?2)",
                rusqlite::params!["12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK", "/ip4/127.0.0.1/tcp/4001"]
            ).expect("insert user failed");

            conn.last_insert_rowid()
        };

        let nickname_id = {
            let conn = db.lock().unwrap();

            conn.execute(
                "INSERT INTO tbl_nicknames (user_id, nickname) VALUES (?1, ?2)",
                rusqlite::params![user_id, "Bob"]
            ).expect("insert nickname failed");

            conn.last_insert_rowid()
        };

        let nickname = fetch_nickname_by_user_id(db, user_id)
            .expect("fetch_nickname_by_user_id failed");

        assert_eq!(nickname.id, nickname_id);
        assert_eq!(nickname.user_id, user_id);
        assert_eq!(nickname.nickname, "Bob");
    }

    #[test]
    pub fn test_create_nickname_correctly_inserts_nickname_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap())
            .expect("User creation failed");

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;",
                [], 
                |row| row.get(0)
            ).unwrap()
        };

        create_nickname(db.clone(), user_id, "alice".to_string())
            .expect("Nickname creation failed");

        let (stored_user_id, stored_nickname): (i64, String) = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT user_id, nickname FROM tbl_nicknames LIMIT 1;",
                [],
                |row| Ok((row.get(0)?, row.get(1)?))
            ).unwrap()
        };

        assert_eq!(stored_user_id, user_id);
        assert_eq!(stored_nickname, "alice");
    }

    #[test]
    pub fn test_update_nickname_correctly_updates_nickname_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap())
            .unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_nickname(db.clone(), user_id, "old_name".to_string()).unwrap();

        let nickname_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_nicknames LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        update_nickname(db.clone(), nickname_id, "new_name".to_string())
            .expect("Nickname update failed");

        let updated_nickname: String = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT nickname FROM tbl_nicknames WHERE id=?1;",
                [nickname_id],
                |r| r.get(0)
            ).unwrap()
        };

        assert_eq!(updated_nickname, "new_name");
    }

    #[test]
    pub fn test_delete_nickname_correctly_deletes_nickname_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap())
            .unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_nickname(db.clone(), user_id, "to_be_deleted".to_string()).unwrap();

        let nickname_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_nicknames LIMIT 1;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        delete_nickname(db.clone(), nickname_id).expect("Nickname delete failed");

        let remaining_count: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM tbl_nicknames;",
                [],
                |r| r.get(0)
            ).unwrap()
        };

        assert_eq!(remaining_count, 0);
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap())
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
                "INSERT INTO tbl_friends (user_id) VALUES (?1);",
                [user_id]
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap())
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
                "INSERT INTO tbl_friends (user_id) VALUES (?1);",
                [user_id]
            ).unwrap();
            conn.last_insert_rowid()
        };

        let friend = fetch_friend_by_user_id(db.clone(), user_id)
            .expect("Friend fetch failed");

        assert_eq!(friend.id, friend_id);
        assert_eq!(friend.user_id, user_id);
    }

    #[test]
    pub fn test_create_friend_correctly_inserts_friend_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap())
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let dm_id: i64 = {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO tbl_direct_messages (user_id, from_me, content, created_at, edited_at, read) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                rusqlite::params![user_id, true, "Hello", Utc::now().to_rfc3339(), "", false],
            ).unwrap();
            conn.last_insert_rowid()
        };

        let dm = fetch_direct_message_by_id(db.clone(), dm_id).expect("fetch failed");

        assert_eq!(dm.id, dm_id);
        assert_eq!(dm.user_id, user_id);
        assert_eq!(dm.content, "Hello");
        assert_eq!(dm.from_me, true);
        assert_eq!(dm.read, false);
        assert!(dm.edited_at.is_none());
    }

    #[test]
    pub fn test_fetch_direct_messages_with_user_errors_invalid_user_id() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        let result = fetch_direct_messages_with_user(db.clone(), 999);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("was not found"), "Unexpected error message");
    }

    #[test]
    pub fn test_fetch_direct_messages_with_user_correctly_fetches_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let conn = db.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO tbl_direct_messages (user_id, from_me, content, created_at, edited_at, read) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            rusqlite::params![user_id, true, "Hello 1", now, "", false]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_direct_messages (user_id, from_me, content, created_at, edited_at, read) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            rusqlite::params![user_id, false, "Hello 2", now, "", false]
        ).unwrap();
        drop(conn);

        let dms = fetch_direct_messages_with_user(db.clone(), user_id).expect("fetch failed");

        assert_eq!(dms.len(), 2);
        assert!(dms.iter().any(|dm| dm.content == "Hello 1"));
        assert!(dms.iter().any(|dm| dm.content == "Hello 2"));
    }

    #[test]
    pub fn test_create_direct_message_correctly_inserts_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_direct_message(db.clone(), user_id, true, "Hello DM".to_string())
            .expect("create_direct_message failed");

        let (dm_id, dm_user_id, content, from_me): (i64, i64, String, bool) = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id, user_id, content, from_me FROM tbl_direct_messages LIMIT 1;",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            ).unwrap()
        };

        assert_eq!(dm_user_id, user_id);
        assert_eq!(content, "Hello DM");
        assert!(from_me);
        assert!(dm_id > 0);
    }

    #[test]
    pub fn test_update_direct_message_correctly_updates_direct_message_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_direct_message(db.clone(), user_id, true, "Original Content".to_string()).unwrap();

        let dm_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_direct_messages LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        create_direct_message(db.clone(), user_id, true, "To Be Deleted".to_string()).unwrap();

        let dm_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_direct_messages LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

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
                rusqlite::params![user_id, "My first post", Utc::now().to_rfc3339()]
            ).unwrap();
            conn.last_insert_rowid()
        };

        let post = fetch_post_by_id(db.clone(), post_id).expect("fetch_post_by_id failed");

        assert_eq!(post.id, post_id);
        assert_eq!(post.author_user_id, user_id);
        assert_eq!(post.content, "My first post");
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let conn = db.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "Post 1", now]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "Post 2", now]
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

        let user_id: i64 = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT id FROM tbl_users LIMIT 1;", 
                [], 
                |r| r.get(0)
            ).unwrap()
        };

        let conn = db.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "User Post 1", now]
        ).unwrap();
        conn.execute(
            "INSERT INTO tbl_posts (author_user_id, content, created_at) VALUES (?1, ?2, ?3);",
            rusqlite::params![user_id, "User Post 2", now]
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

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
        assert!(post.edited_at.is_none());
    }

    #[test]
    pub fn test_update_post_correctly_updates_post_data() {
        let db = init_db(":memory:".into()).expect("DB init failed");

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1234/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1000/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();
        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

        let user_ids: Vec<i64> = {
            let conn = db.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id FROM tbl_users;").unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap()
                .map(|id| id.unwrap())
                .collect()
        };

        for id in &user_ids {
            db.lock().unwrap().execute(
                "INSERT INTO tbl_blocked_users (user_id) VALUES (?1);",
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1002/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();
        let db_guard = db.lock().unwrap();
        let user_id: i64 = db_guard.query_row(
            "SELECT id FROM tbl_users LIMIT 1;", 
            [], 
            |r| r.get(0)
        ).unwrap();
        db_guard.execute(
            "INSERT INTO tbl_blocked_users (user_id) VALUES (?1);", 
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1003/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();
        
        let db_guard = db.lock().unwrap();
        let user_id: i64 = db_guard.query_row(
            "SELECT id FROM tbl_users LIMIT 1;", 
            [], 
            |r| r.get(0)
        ).unwrap();
        db_guard.execute(
            "INSERT INTO tbl_blocked_users (user_id) VALUES (?1);", 
            rusqlite::params![user_id]
        ).unwrap();

        drop(db_guard);

        let blocked_user = fetch_blocked_user_by_user_id(db.clone(), user_id).unwrap();
        assert_eq!(blocked_user.user_id, user_id);
    }

    #[test]
    pub fn test_is_user_blocked_correctly_returns_true() {
        let db = init_db(":memory:".into()).unwrap();

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1004/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();
        
        let db_guard = db.lock().unwrap();

        let user_id: i64 = db_guard.query_row(
            "SELECT id FROM tbl_users LIMIT 1;", 
            [], 
            |r| r.get(0)
        ).unwrap();
        db_guard.execute(
            "INSERT INTO tbl_blocked_users (user_id) VALUES (?1);", 
            rusqlite::params![user_id]
        ).unwrap();

        drop(db_guard);

        let blocked = is_user_blocked(db.clone(), user_id).unwrap();
        assert!(blocked);
    }

    #[test]
    pub fn test_is_user_blocked_correctly_returns_false() {
        let db = init_db(":memory:".into()).unwrap();

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/1005/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();
        
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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/9000/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

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

        create_user(db.clone(), PeerId::random(), "/ip4/127.0.0.1/tcp/9001/p2p/12D3KooWHGLsSWMsiU35gg5zUD9zmHpLrdwpnftASGFwpArLkTsK".parse().unwrap()).unwrap();

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