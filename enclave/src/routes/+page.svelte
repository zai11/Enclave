<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  interface Message {
    from: string;
    content: string;
    timestamp: number;
  }

  interface FriendInfo {
    peer_id: string;
    multiaddr: string;
  }

  interface FriendRequest {
    from_peer_id: string;
    from_multiaddr: string;
    message: string;
  }

  let myPeerId = '';
  let myInfo: FriendInfo | null = null;
  let messages: Message[] = [];
  let connectedPeers: string[] = [];
  let friendList: string[] = [];
  let pendingRequests: Array<{peerId: string, request: FriendRequest}> = [];
  let messageInput = '';
  let addFriendPeerId = '';
  let addFriendAddress = '';
  let addFriendMessage = '';
  let relayAddress = '';
  let isStarted = false;
  let showAddFriend = false;
  let showFriendRequests = false;
  let showRelaySettings = false;

  onMount(async () => {
    if (!("__TAURI_INTERNALS__" in window)) {
      console.warn("Tauri API not available");
      return;
    }

    // Listen for P2P events:
    await listen('message-received', (event: any) => {
      messages = [...messages, event.payload];
    });

    await listen('peer-connected', (event: any) => {
      const peerId = event.payload as string;
      if (!connectedPeers.includes(peerId)) {
        connectedPeers = [...connectedPeers, peerId];
      }
    });

    await listen('peer-disconnected', (event: any) => {
      const peerId = event.payload as string;
      connectedPeers = connectedPeers.filter(p => p !== peerId);
    });

    await listen('friend-request-received', (event: any) => {
      console.log('Friend request received event:', event.payload);
      const [ peerId, request ] = event.payload;
      console.log('Parsed - peerId:', peerId, 'request:', request);
      if (!pendingRequests.some(r => r.peerId === peerId)) {
        pendingRequests = [...pendingRequests, {peerId, request}];
        showFriendRequests = true;
        console.log('Added to pending requests. Total:', pendingRequests.length);
      } else {
        console.log('Request already exists in pending list.');
      }
    });

    await listen('friend-request-accepted', async (event: any) => {
      const peerId = event.payload as string;
      alert('Friend request was accepted');
      await refreshFriendList();
    });

    await listen('friend-request-denied', (event: any) => {
      alert('Friend request was denied');
    });
  });

  async function startP2P() {
    try {
      myPeerId = await invoke<string>('start_p2p');
      myInfo = await invoke<FriendInfo>('get_my_info');
      isStarted = true;
    }
    catch (error) {
      console.error('Failed to start P2P:', error);
      alert('Failed to start P2P: ' + error);
    }
  }

  async function connectRelay() {
    if (!relayAddress.trim()) {
      alert('Please enter a relay address');
      return;
    }

    try {
      await invoke('connect_to_relay', {relayAddress});
      showRelaySettings = false;
      alert('Connected to relay successfully!');
      myInfo = await invoke<FriendInfo>('get_my_info');
    } catch (error) {
      console.log('Failed to connect to relay:', error);
      alert('Failed to connect to relay: ' + error);
    }
  }

  async function sendMessage() {
    if (!messageInput.trim()) return;

    try {
      await invoke('send_message', { content: messageInput });
      messageInput = '';
    } catch (error) {
      console.error('Failed to send message:', error);
      alert('Failed to send message: ' + error);
    }
  }

  async function sendFriendRequest() {
    if (!addFriendPeerId.trim() || !addFriendAddress.trim()) {
      alert('Please enter both peer ID and address');
      return;
    }

    try {
      await invoke('send_friend_request', {
        peerId: addFriendPeerId,
        multiaddr: addFriendAddress,
        message: addFriendMessage || 'Hi, let\'s connect on Enclave!'
      });
      addFriendPeerId = '';
      addFriendAddress = '';
      addFriendMessage = '';
      showAddFriend = false;
      alert('Friend request sent!');
    } catch (error) {
      console.error('Failed to send friend request:', error);
      alert('Failed to send friend request: ' + error);
    }
  }

  async function acceptFriendRequest(peerId: string) {
    try {
      await invoke('accept_friend_request', { peerId });
      pendingRequests = pendingRequests.filter(r => r.peerId !== peerId);
      if (pendingRequests.length === 0) {
        showFriendRequests = false;
      }
      await refreshFriendList();
    } catch (error) {
      console.error('Failed to accept friend request:', error);
      alert('Failed to accept friend request: ' + error);
    }
  }

  async function denyFriendRequest(peerId: string) {
    try {
      await invoke('deny_friend_request', { peerId });
      pendingRequests = pendingRequests.filter(r => r.peerId !== peerId);
      if (pendingRequests.length === 0) {
        showFriendRequests = false;
      }
    } catch (error) {
      console.error('Failed to deny friend request:', error);
      alert('Failed to deny friend request: ' + error);
    }
  }

  async function refreshFriendList() {
    try {
      friendList = await invoke<string[]>('get_friend_list');
    } catch (error) {
      console.error('Failed to get friend list:', error);
    }
  }

  function copyMyInfo() {
    if (!myInfo) return;
    const text = `enclave://add-friend?peer=${myInfo.peer_id}&addr=${myInfo.multiaddr}`;
    navigator.clipboard.writeText(text);
    alert('Connection info copied to clipboard!');
  }

  function formatTime(timestamp: number) {
    return new Date(timestamp * 1000).toLocaleTimeString();
  }
</script>

<main>
  <div class="container">
    <header>
      <h1>Enclave</h1>
      <p class="subtitle">The P2P social media platform - where you control your data.</p>
    </header>

    {#if !isStarted}
      <div class="welcome">
        <p>Start your private network to connect with friends</p>
        <button on:click={startP2P} class="btn-primary">Start Enclave</button>
      </div>
    {:else}
      <div class="app">
        <!-- Sidebar -->
        <aside class="sidebar">
          <div class="my-info">
            <h3>My Info</h3>
            <p class="peer-id">{myPeerId.slice(0, 16)}...</p>
            <button on:click={copyMyInfo} class="btn-secondary">Copy Connection Info</button>
            <button on:click={() => showRelaySettings = true} class="btn-secondary" style="margin-top: 8px;">Relay Settings</button>
          </div>
          <div class="friends">
            <h3>Friends ({friendList.length})</h3>
            {#if friendList.length === 0}
              <p class="empty">No friends yet</p>
            {:else}
              <ul>
                {#each friendList as friend}
                  <li class="peer-item">{friend.slice(0, 16)}...</li>
                {/each}
              </ul>
            {/if}
            <h3 style="margin-top: 16px;">Connected ({connectedPeers.length})</h3>
            {#if connectedPeers.length === 0}
              <p class="empty">No friends connected</p>
            {:else}
              <ul>
                {#each connectedPeers as peer}
                  <li class="peer-item">{peer.slice(0, 16)}...</li>
                {/each}
              </ul>
            {/if}
            <button on:click={() => showAddFriend = !showAddFriend} class="btn-secondary">Send Friend Request</button>
            {#if pendingRequests.length > 0}
              <button on:click={() => showFriendRequests = !showFriendRequests} class="btn-primary" style="margin-top: 8px;">Friend Requests ({pendingRequests.length})</button>
            {/if}
          </div>
        </aside>

        <!-- Main Content -->
        <div class="main-content">
          <div class="messages">
            {#if messages.length === 0}
              <div class="empty-messages">
                <p>No messages yet. Start a conversation!</p>
              </div>
            {:else}
              {#each messages as message}
                <div class="message">
                  <div class="message-header">
                    <span class="message-from">{message.from.slice(0, 16)}...</span>
                    <span class="message-time">{formatTime(message.timestamp)}</span>
                  </div>
                  <div class="message-content">{message.content}</div>
                </div>
              {/each}
            {/if}
          </div>
          <div class="message-input">
            <input type="text" bind:value={messageInput} on:keypress={(e) => e.key === 'Enter' && sendMessage()} placeholder='Type a message...' />
            <button on:click={sendMessage} class="btn-primary">Send</button>
          </div>
        </div>
      </div>

      <!-- Relay Settings Modal -->
      {#if showRelaySettings}
        <div class="modal-overlay">
          <div class="modal">
            <h2>Relay Server Settings</h2>
            <p>Connect to a relay server to enable connections over the internet</p>
            <label>
              Relay Address
              <input type="text" bind:value={relayAddress} placeholder="/ip4/127.0.0.1/tcp/4001/p2p/12D3Koo..." />
            </label>
            <div class="help-text">
              <strong>Format:</strong> /ip4/IP_ADDRESS/tcp/PORT/p2p/PEER_ID
              <br />
              <string>Example:</string> /ip4/127.0.0.1/tcp/4001/p2p/12D3KooWABC...
            </div>
            <div class="modal-actions">
              <button on:click={() => showRelaySettings = false} class="btn-secondary">Cancel</button>
              <button on:click={connectRelay} class="btn-primary">Connect</button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Send Friend Request Modal -->
      {#if showAddFriend}
      <div class="modal-overlay">
        <div class="modal">
          <h2>Send Friend Request</h2>
          <p>Enter your friend's connection information</p>
          <label>
            Peer ID
            <input type="text" bind:value={addFriendPeerId} placeholder="12D3KooW..." />
          </label>
          <label>
            Address
            <input type="text" bind:value={addFriendAddress} placeholder="/ip4/192.168.1.100/tcp/54321" />
          </label>
          <label>
            Message
            <input type="text" bind:value={addFriendMessage} placeholder="Hi, let\'s connect on Enclave!" />
          </label>
          <div class="modal-actions">
            <button on:click={() => showAddFriend = false} class="btn-secondary">Cancel</button>
            <button on:click={sendFriendRequest} class="btn-primary">Send Request</button>
          </div>
        </div>
      </div>
      {/if}

      <!-- Friend Requests Modal -->
      {#if showFriendRequests}
        <div class="modal-overlay">
          <div class="modal">
            <h2>Friend Requests</h2>
            {#each pendingRequests as { peerId, request }}
              <div class="friend-request">
                <p class="request-peer">{peerId.slice(0, 24)}...</p>
                <p class="request-message">"{request.message}"</p>
                <div class="request-actions">
                  <button on:click={() => acceptFriendRequest(peerId)} class="btn-primary">Accept</button>
                  <button on:click={() => denyFriendRequest(peerId)} class="btn-secondary">Deny</button>
                </div>
              </div>
            {/each}

            <div class="modal-actions" style="margin-top: 24px;">
              <button on:click={() => showFriendRequests = false} class="btn-secondary">Close</button>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
    background: #f5f5f5;
  }

  .container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
  }

  header {
    text-align: center;
    margin-bottom: 40px;
  }

  h1 {
    font-size: 48px;
    margin: 0;
    color: #333;
  }

  .subtitle {
    color: #666;
    margin-top: 8px;
  }

  .welcome {
    text-align: center;
    padding: 60px 20px;
  }

  .welcome p {
    font-size: 18px;
    color: #666;
    margin-bottom: 24px;
  }

  .app {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 20px;
    height: calc(100vh - 200px);
  }

  .sidebar {
    background-color: white;
    border-radius: 12px;
    padding: 20px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    display: flex;
    flex-direction: column;
    gap: 24px;
    overflow-y: auto;
  }

  .my-info h3,
  .friends h3 {
    margin: 0 0 12px 0;
    font-size: 16px;
    font-weight: 600;
    color: #333;
  }

  .peer-id {
    font-family: monospace;
    font-size: 12px;
    color: #666;
    background-color: #f5f5f5;
    padding: 8px;
    border-radius: 4px;
    margin-bottom: 12px;
    word-break: break-all;
  }

  .friends ul {
    list-style: none;
    padding: 0;
    margin: 0 0 12px 0;
  }

  .peer-item {
    padding: 8px;
    background-color: #f5f5f5;
    border-radius: 4px;
    margin-bottom: 4px;
    font-size: 12px;
    font-family: monospace;
  }

  .main-content {
    background-color: white;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    display: flex;
    flex-direction: column;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .empty-messages {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #999;
  }

  .message {
    margin-bottom: 16px;
    padding: 12px;
    background-color: #f5f5f5;
    border-radius: 8px;
  }

  .message-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 4px;
    font-size: 12px;
  }

  .message-from {
    font-weight: 600;
    color: #333;
  }

  .message-time {
    color: #999;
  }

  .message-content {
    color: #333;
  }

  .message-input {
    display: flex;
    gap: 8px;
    padding: 20px;
    border-top: 1px solid #eee;
  }

  .message-input input {
    flex: 1;
    padding: 12px;
    border: 1px solid #ddd;
    border-radius: 8px;
    font-size: 14px;
  }

  .btn-primary {
    background-color: #007aff;
    color: white;
    border: none;
    padding: 12px 24px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .btn-primary:hover {
    background-color: #0051df;
  }

  .btn-secondary {
    background-color: white;
    color: #333;
    border: 1px solid #ddd;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s;
    width: 100%;
  }

  .btn-secondary:hover {
    background-color: #f5f5f5;
  }

  .empty {
    color: #999;
    font-size: 14px;
    text-align: center;
    padding: 20px 0;
  }

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background-color: white;
    padding: 32px;
    border-radius: 12px;
    max-width: 500px;
    width: 90%;
    max-height: 80vh;
    overflow-y: auto;
  }

  .modal h2 {
    margin: 0 0 8px 0;
  }

  .modal p {
    color: #666;
    margin: 0 0 24px 0;
  }

  .modal label {
    display: block;
    margin-bottom: 16px;
    font-weight: 600px;
    color: #333;
  }

  .modal input {
    display: block;
    width: 100%;
    padding: 12px;
    border: 1px solid #ddd;
    border-radius: 8px;
    font-size: 14px;
    margin-top: 8px;
    box-sizing: border-box;
  }

  .modal-actions {
    display: flex;
    gap: 12px;
    margin-top: 24px;
  }

  .modal-actions button {
    flex: 1;
  }

  .friend-request {
    padding: 16px;
    background-color: #f5f5f5;
    border-radius: 8px;
    margin-bottom: 12px;
  }

  .request-peer {
    font-family: monospace;
    font-size: 12px;
    color: #333;
    margin: 0 0 8px 0;
  }

  .request-message {
    color: #666;
    font-style: italic;
    margin: 0 0 12px 0;
  }

  .request-actions {
    display: flex;
    gap: 8px;
  }

  .request-actions button {
    flex: 1;
  }

  .help-text {
    background-color: #f5f5f5;
    padding: 12px;
    border-radius: 4px;
    font-size: 12px;
    color: #666;
    margin-top: 12px;
    font-family: monospace;
  }

  .help-text strong {
    color: #333;
  }
</style>