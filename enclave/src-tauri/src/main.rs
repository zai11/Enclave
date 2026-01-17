// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod p2p;

use p2p::{P2PNode, P2PEvent, FriendInfo};
use tauri::Emitter;
use tokio::sync::Mutex;
use std::sync::Arc;
use libp2p::{PeerId, Multiaddr};

struct AppState {
    p2p_node: Arc<Mutex<Option<P2PNode>>>
}

#[tauri::command]
async fn start_p2p(state: tauri::State<'_, AppState>, app: tauri::AppHandle) -> Result<String, String> {
    let (node, mut event_receiver) = P2PNode::new()
        .await
        .map_err(|err| err.to_string())?;

    let peer_id = node.get_peer_id().to_string();

    *state.p2p_node.lock().await = Some(node);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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
    if let Some(node) = node_guard.as_ref() {
        let peer_id = node.get_peer_id().to_string();
        let addresses = node.get_listen_addresses().await;

        let multiaddr = addresses
            .first()
            .ok_or("No listening addresses")?
            .to_string();

        Ok(
            FriendInfo {
                peer_id,
                multiaddr
            }
        )
    }
    else {
        Err("P2P node not started".into())
    }
}

#[tauri::command]
async fn send_friend_request(
    state: tauri::State<'_, AppState>,
    peer_id: String,
    multiaddr: String,
    message: String
) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let peer = peer_id.parse::<PeerId>().map_err(|err| err.to_string())?;
        let address = multiaddr.parse::<Multiaddr>().map_err(|err| err.to_string())?;
        node.send_friend_request(peer, address, message).map_err(|err| err.to_string())?;
        Ok(())
    }
    else {
        Err("P2P node not started".into())
    }
}

#[tauri::command]
async fn accept_friend_request(state: tauri::State<'_, AppState>, peer_id: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let peer = peer_id.parse::<PeerId>().map_err(|err| err.to_string())?;
        node.accept_friend_request(peer).map_err(|err| err.to_string())?;
        Ok(())
    }
    else {
        Err("P2P node not started".into())
    }
}

#[tauri::command]
async fn deny_friend_request(state: tauri::State<'_, AppState>, peer_id: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let peer = peer_id.parse::<PeerId>().map_err(|err| err.to_string())?;
        node.deny_friend_request(peer).map_err(|err| err.to_string())?;
        Ok(())
    }
    else {
        Err("P2P node not started".into())
    }
}

#[tauri::command]
async fn send_message(state: tauri::State<'_, AppState>, content: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        node.send_message(content).map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn send_direct_message(state: tauri::State<'_, AppState>, peer_id: String, content: String) -> Result<(), String> {
    let node_guard = state.p2p_node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let peer = peer_id.parse::<PeerId>().map_err(|err| err.to_string())?;
        node.send_direct_message(peer, content).map_err(|err| err.to_string())?;
        Ok(())
    }
    else {
        Err("P2P node not started".into())
    }
}

#[tauri::command]
async fn get_friend_list(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let node_guard = state.p2p_node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let friends = node.get_friend_list().await.map_err(|err| err.to_string())?;
        Ok(friends.iter().map(|p| p.to_string()).collect())
    }
    else {
        Err("P2P node not started".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
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
            get_friend_list
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}
