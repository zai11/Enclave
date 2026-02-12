// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod logger;
mod p2p;

use chrono::Utc;
use log::LevelFilter;
use p2p::{P2PNode, P2PEvent};
use tauri::Emitter;
use tokio::sync::Mutex;
use std::{str::FromStr, sync::Arc};
use libp2p::{PeerId, Multiaddr};

use crate::{db::models::direct_message::DirectMessage, logger::Logger, p2p::{FriendRequest, MyInfo}};

static LOGGER: once_cell::sync::Lazy<Logger> =
    once_cell::sync::Lazy::new(|| {
        let date_string = Utc::now().format("%Y%m%d").to_string();
        Logger::new(format!("./logs/{date_string}.log").as_str(), LevelFilter::Info).expect("failed to create logger")
    });

struct AppState {
    p2p_node: Arc<Mutex<Option<P2PNode>>>,
}

#[tauri::command]
async fn start_p2p(state: tauri::State<'_, AppState>, app: tauri::AppHandle) -> Result<String, String> {
    let relay_address = None;

    let (node, mut event_receiver) = match P2PNode::new(relay_address).await {
        Ok((node, event_receiver)) => (node, event_receiver),
        Err(err) => {
            log::error!("start_p2p: {err}");
            return Err(err.to_string());
        }
    };

    *state.p2p_node.lock().await = Some(node);

    let MyInfo{peer_id, ..} = match get_my_info(state.clone()).await {
        Ok(info) => info,
        Err(err) => {
            log::error!("start_p2p: {err}");
            return Err(err);
        }
    };

    app.emit("refresh-inbound-friend-requests", ()).ok();
    app.emit("refresh-friend-list", ()).ok();

    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            match event {
                P2PEvent::DirectMessageReceived(msg) => {
                    app.emit("dm-received", msg).ok();
                },
                P2PEvent::DirectMessageSent(msg) => {
                    app.emit("dm-sent", msg).ok();
                }
                P2PEvent::PeerConnected(peer) => {
                    app.emit("peer-connected", peer.to_string()).ok();
                },
                P2PEvent::PeerDisconnected(peer) => {
                    app.emit("peer-disconnected", peer.to_string()).ok();
                },
                P2PEvent::FriendRequestReceived { from, request } => {
                    app.emit("friend-request-received", (from.to_string(), request)).ok();
                },
                P2PEvent::FriendRequestAccepted { peer } => {
                    app.emit("friend-request-accepted", peer.to_string()).ok();
                },
                P2PEvent::FriendRequestDenied { peer } => {
                    app.emit("friend-request-denied", peer.to_string()).ok();
                },
                P2PEvent::Error { context, error } => {
                    log::error!("{context}: {error}");
                }
            }
        }
    });

    Ok(peer_id)
}

#[tauri::command]
async fn get_my_info(state: tauri::State<'_, AppState>) -> Result<MyInfo, String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("get_my_info called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let addresses = node.get_listen_addresses().await;

    let multiaddr = match addresses.first() {
        Some(addr) => addr.to_string(),
        None => {
            log::error!(
                "get_my_info: node {} has no listening addresses",
                node.get_peer_id()
            );
            return Err("No listening addresses".into());
        }
    };

    let keypair = node.get_keypair().to_protobuf_encoding()
        .map_err(|err| err.to_string())?;

    Ok(MyInfo {
        peer_id: node.get_peer_id().to_string(),
        keypair,
        multiaddr,
    })
}

#[tauri::command]
async fn send_friend_request(
    state: tauri::State<'_, AppState>,
    peer_id: String,
    multiaddr: String,
    message: String
) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("send_friend_request called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let peer = match peer_id.parse::<PeerId>() {
        Ok(peer) => peer,
        Err(err) => {
            log::error!("send_friend_request: {}", err.to_string());
            return Err(err.to_string());
        }
    };

    let address = match multiaddr.parse::<Multiaddr>(){
        Ok(address) => address,
        Err(err) => {
            log::error!("send_friend_request: {}", err.to_string());
            return Err(err.to_string());
        }
    };

    let _ = match node.send_friend_request(peer, address, message) {
        Ok(_) => (),
        Err(err) => {
            log::error!("send_friend_request: {}", err.to_string());
            return Err(err.to_string());
        }
    };

    Ok(())
}

#[tauri::command]
async fn accept_friend_request(state: tauri::State<'_, AppState>, peer_id: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("accept_friend_request called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let peer = match peer_id.parse::<PeerId>() {
        Ok(peer) => peer,
        Err(err) => {
            log::error!("accept_friend_request: {}", err.to_string());
            return Err(err.to_string());
        }
    };

    let _ = match node.accept_friend_request(peer) {
        Ok(_) => (),
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    };

    Ok(())
}

#[tauri::command]
async fn deny_friend_request(state: tauri::State<'_, AppState>, peer_id: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("deny_friend_request called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let peer = match peer_id.parse::<PeerId>() {
        Ok(peer) => peer,
        Err(err) => {
            log::error!("deny_friend_request: {}", err.to_string());
            return Err(err.to_string());
        }
    };

    let _ = match node.deny_friend_request(peer) {
        Ok(_) => (),
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    };

    Ok(())
}

#[tauri::command]
async fn send_message(state: tauri::State<'_, AppState>, content: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("send_message called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let _ = match node.send_message(content) {
        Ok(_) => (),
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    };

    Ok(())
}

#[tauri::command]
async fn send_direct_message(state: tauri::State<'_, AppState>, peer_id: String, content: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("send_direct_message called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let peer = match peer_id.parse::<PeerId>() {
        Ok(peer) => peer,
        Err(err) => {
            log::error!("send_direct_message: {}", err.to_string());
            return Err(err.to_string());
        }
    };

    let address = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer_id) {
        Ok(user) => match Multiaddr::from_str(&user.multiaddr) {
            Ok(address) => address,
            Err(err) => {
                log::error!("send_direct_message: {}", err.to_string());
                return Err(err.to_string())
            }
        },
        Err(err) => {
            log::error!("send_direct_message: {}", err.to_string());
            return Err(err.to_string())
        }
    };

    let _ = match node.send_direct_message(peer, address, content) {
        Ok(_) => (),
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    };
    
    Ok(())
}

#[tauri::command]
async fn get_friend_list(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("get_friend_list called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let friends = match node.get_friend_list().await {
        Ok(friends) => friends,
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    };

    Ok(friends.iter().map(|p| p.to_string()).collect())
}

#[tauri::command]
async fn get_inbound_friend_requests(state: tauri::State<'_, AppState>) -> Result<Vec<(String, FriendRequest)>, String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("get_inbound_friend_requests called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let friend_requests = match node.get_inbound_friend_requests().await {
        Ok(friend_requests) => friend_requests,
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    }
        .iter()
        .map(|(peer_id, friend_request)| (peer_id.to_string(), friend_request.clone()))
        .collect::<Vec<(String, FriendRequest)>>();

    Ok(friend_requests)
}

#[tauri::command]
async fn get_direct_messages(state: tauri::State<'_, AppState>, peer_id: String) -> Result<Vec<DirectMessage>, String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("get_direct_messages called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let peer_id = match PeerId::from_str(&peer_id) {
        Ok(p) => p,
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    };

    let direct_messages = match node.get_direct_messages(peer_id).await {
        Ok(dms) => dms,
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    };

    Ok(direct_messages)
}

#[tauri::command]
async fn connect_to_relay(state: tauri::State<'_, AppState>, relay_address: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;

    let node = match node_guard.as_ref() {
        Some(node) => node,
        None => {
            log::warn!("connect_to_relay called but P2P node not started");
            return Err("P2P node not started".into());
        }
    };

    let address = match relay_address.parse::<Multiaddr>() {
        Ok(address) => address,
        Err(err) => {
            log::error!("connect_to_relay: {}", err.to_string());
            return Err(err.to_string());
        }
    };

    let _ = match node.connect_to_relay(address) {
        Ok(_) => (),
        Err(err) => {
            log::error!("{}", err.to_string());
            return Err(err.to_string());
        }
    };

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    log::set_logger(&*LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);
    
    log::info!("Application Started");

    if let Err(err) = tauri::Builder::default()
        .manage(AppState {
            p2p_node: Arc::new(Mutex::new(None))
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            start_p2p,
            get_my_info,
            send_friend_request,
            accept_friend_request,
            deny_friend_request,
            send_message,
            send_direct_message,
            get_friend_list,
            get_inbound_friend_requests,
            get_direct_messages,
            connect_to_relay
        ])
        .run(tauri::generate_context!()) {
            log::error!("Error while running tauri application: {}", err.to_string());
        }
}
