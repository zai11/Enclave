// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod logger;
mod p2p;

use chrono::Utc;
use log::LevelFilter;
use p2p::{P2PNode, P2PEvent, FriendInfo};
use tauri::Emitter;
use tokio::sync::Mutex;
use std::sync::Arc;
use libp2p::{PeerId, Multiaddr};

use crate::logger::Logger;

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

    let peer_id = node.get_peer_id().to_string();

    *state.p2p_node.lock().await = Some(node);

    tokio::spawn(async move {
        while let Some(event) = event_receiver.recv().await {
            match event {
                P2PEvent::MessageReceived(msg) => {
                    app.emit("message-received", msg).ok();
                },
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
                }
            }
        }
    });

    Ok(peer_id)
}

#[tauri::command]
async fn get_my_info(state: tauri::State<'_, AppState>) -> Result<FriendInfo, String> {
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

    Ok(FriendInfo {
        peer_id: node.get_peer_id().to_string(),
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

    let _ = match node.send_direct_message(peer, content) {
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
            connect_to_relay
        ])
        .run(tauri::generate_context!()) {
            log::error!("Error while running tauri application: {}", err.to_string());
        }
}
