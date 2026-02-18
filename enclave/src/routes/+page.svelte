<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";

    interface Post {
        author_peer_id: string;
        content: string;
        created_at: number;
        edited_at: number;
    }

    interface DirectMessage {
        from_peer_id: string;
        to_peer_id: string;
        content: string;
        created_at: number;
        edited_at: number;
        read: boolean;
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
    let connectedPeers: string[] = [];
    let friendList: string[] = [];
    let pendingRequests: Array<[string, FriendRequest]> = [];
    let postInput = '';
    let addFriendAddress = '';
    let addFriendMessage = '';
    let relayAddress = '';
    let isStarted = false;
    let showAddFriend = false;
    let showFriendRequests = false;
    let showRelaySettings = false;
    let contextMenuVisible = false;
    let contextMenuX = 0;
    let contextMenuY = 0;
    let contextMenuPeer: string | null = null;
    let showProfile = false;
    let profilePeer: { peerId: string, isFriend: boolean } | null = null;
    let nicknames = new Map<string, string>();
    let editingPeer: string | null = null;
    let draftNickname = '';
    let directMessageOpen = false;
    let activeDMPeerId: string | null = null;
    let directMessages: DirectMessage[] = [];
    let directMessageInput = '';
    let openBoardPeerId: string | null = null;
    let posts: Post[] = [];

    onMount(async () => {
        if (!("__TAURI_INTERNALS__" in window)) {
            console.warn("Tauri API not available");
            return;
        }

        // Listen for P2P events:
        await listen('post-received', (event: any) => {
            posts = [...posts, event.payload];
        });

        await listen('post-sent', (event: any) => {
            posts = [...posts, event.payload];
        });

        await listen('dm-received', (event: any) => {
            directMessages = [...directMessages, event.payload];
        });

        await listen('dm-sent', (event: any) => {
            directMessages = [...directMessages, event.payload];
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
            const [ peerId, request ] = event.payload;
            if (!pendingRequests.some(r => r[0] === peerId)) {
                pendingRequests = [...pendingRequests, [peerId, request]];
                showFriendRequests = true;
            } else {
                console.error('Request already exists in pending list.');
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

        await listen('refresh-inbound-friend-requests', (event: any) => {
            refreshInboundFriendRequests();
        });

        await listen('refresh-friend-list', (event: any) => {
            refreshFriendList();
        });

        await listen('load-feed', (event: any) => {
            loadFeed();
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
            console.error('Failed to connect to relay:', error);
            alert('Failed to connect to relay: ' + error);
        }
    }

    async function sendPost() {
        if (!postInput.trim()) return;

        try {
            await invoke('send_post', { content: postInput });
            postInput = '';
        } catch (error) {
            console.error('Failed to send post:', error);
            alert('Failed to send post: ' + error);
        }
    }

    async function sendFriendRequest() {
        if (!addFriendAddress.trim()) {
            alert('Please enter friend connection address address');
            return;
        }

        if (!validateMultiaddr(addFriendAddress)) {
            alert('The provided address was invalid');
            return;
        }

        const peerId = addFriendAddress.split('/p2p/')[1];
        const multiaddr = addFriendAddress.split('/p2p/')[0];

        try {
            await invoke('send_friend_request', {
                peerId: peerId,
                multiaddr: multiaddr,
                message: addFriendMessage || 'Hi, let\'s connect on Enclave!'
            });
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
            pendingRequests = pendingRequests.filter(r => r[0] !== peerId);
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
            pendingRequests = pendingRequests.filter(r => r[0] !== peerId);
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

    async function refreshInboundFriendRequests() {
        try {
            pendingRequests = await invoke('get_inbound_friend_requests');
        } catch (error) {
            console.error('Failed to get inbound friend requests:', error);
        }
    }

    async function loadFeed() {
        try {
            posts = await invoke('load_feed');
        } catch (error) {
            console.error('Failed to load feed:', error);
        }
    }

    async function loadBoard() {
        try {
            posts = await invoke('load_board', { peerId: openBoardPeerId });
        } catch (error) {
            console.error('Failed to load board:', error);
        }
    }
  
    async function handleViewProfile() {
        if (contextMenuPeer) {
            profilePeer = {
                peerId: contextMenuPeer,
                isFriend: friendList.includes(contextMenuPeer)
            };

            showProfile = true;
        }

        hideContextMenu();
    }

    function copyPeerId(peerId: string) {
        navigator.clipboard.writeText(peerId);
        alert('Peer ID copied to clipboard!');
    }

    function copyMyAddress() {
        if (!myInfo) return;
        const text = `${myInfo.multiaddr}/p2p/${myInfo.peer_id}`;
        navigator.clipboard.writeText(text);
        alert('Connection info copied to clipboard!');
    }

    function formatTime(timestamp: number) {
        return new Date(timestamp * 1000).toLocaleTimeString();
    }

    function formatDate(timestamp: number) {
        const date = new Date(timestamp * 1000);
        const today = new Date();
        const yesterday = new Date(today);
        yesterday.setDate(yesterday.getDate() - 1);

        if (date.toDateString() === today.toDateString()) {
            return 'Today';
        } else if (date.toDateString() === yesterday.toDateString()) {
            return 'Yesterday';
        } else {
            return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
        }
    }

    function showContextMenu(event: MouseEvent, peerId: string) {
        event.preventDefault();
        contextMenuPeer = peerId;
        contextMenuX = event.clientX;
        contextMenuY = event.clientY;
        contextMenuVisible = true;
    }

    function hideContextMenu() {
        contextMenuPeer = null;
        contextMenuVisible = false;
    }

    function handleRemoveFriend() {
        if (contextMenuPeer) {
            // TODO: Implement remove friend functionality
            alert(`Remove friend: ${contextMenuPeer.slice(0, 16)}...`);
        }
        hideContextMenu();
    }

    function handleBlockPeer() {
        if (contextMenuPeer) {
            // TODO: Implement peer block functionality
            alert(`Block friend: ${contextMenuPeer.slice(0, 16)}...`);
        }
        hideContextMenu();
    }

    function handleClickOutside(event: Event) {
        if (contextMenuVisible) {
            hideContextMenu();
        }
    }

    function validateMultiaddr(multiaddr: string): boolean {
        const multiaddrRegex = /^\/ip4\/(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\/tcp\/([1-9]\d{0,4})\/p2p\/([A-Za-z0-9]+)$/
        return multiaddr.match(multiaddrRegex) !== null
    }

    function updateNickname(newName: string) {
        if (editingPeer) {
            if (newName === '' && nicknames.has(editingPeer)) {
                nicknames.delete(editingPeer)
            }
            else {
                nicknames.set(editingPeer, newName);
            }
        }
    }

    function displayName(peerId: string): string {
        let name = nicknames.get(peerId) ?? peerId;
        if (name.length > 24) {
            return name.slice(0, 24) + '...' + (peerId === myPeerId ? ' (Me)' : '');
        }
        else {
            return name + (peerId === myPeerId ? ' (Me)' : '');
        }
    }

    function autoScroll(node: HTMLElement) {
        const observer = new MutationObserver(() => {
            node.scrollTop = node.scrollHeight;
        });

        observer.observe(node, { childList: true, subtree: true });

        return {
            destroy() {
                observer.disconnect();
            }
        };
    }

    async function sendDirectMessage() {
        if (!directMessageInput.trim()) return;

        try {
            await invoke('send_direct_message', { peerId: activeDMPeerId ?? '', content: directMessageInput });
            directMessageInput = '';
        } catch (error) {
            console.error('Failed to send message:', error);
            alert('Failed to send message: ' + error);
        }
    }

    async function openDirectMessages(peerId: string) {
        try {
            activeDMPeerId = peerId;
            directMessages = await invoke('get_direct_messages', { peerId: activeDMPeerId })
            directMessageOpen = true;
        } catch (error) {
            console.error('Failed to get direct messages:', error);
            alert('Failed to get direct messages: ' + error);
        }
    }

    function viewFriendBoard(peerId: string) {
        openBoardPeerId = peerId;
        loadBoard();
    }

    function viewFeed() {
        openBoardPeerId = null;
        loadFeed();
    }
</script>

<svelte:window 
    on:click={handleClickOutside} 
    on:keydown={(e) => {
        if (e.key === 'Escape') {
            hideContextMenu();
            showProfile = false;
            showAddFriend = false;
            showFriendRequests = false;
            showRelaySettings = false;
        }
    }} 
/>

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
                    <button on:click={copyMyAddress} class="btn-secondary">Copy Connection Info</button>
                    <button on:click={() => showRelaySettings = true} class="btn-secondary" style="margin-top: 8px;">Relay Settings</button>
                </div>
                <div class="friends">
                    <h3>Friends ({friendList.length})</h3>
                    {#if friendList.length === 0}
                        <p class="empty">No friends yet</p>
                    {:else}
                        <ul>
                            {#each friendList as friend}
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                                <li class="peer-item" 
                                    class:active={openBoardPeerId === friend}
                                    on:click={() => viewFriendBoard(friend)}
                                    on:dblclick={(e) => {
                                        e.stopPropagation();
                                        editingPeer = friend;
                                        draftNickname = nicknames.get(friend) ?? friend;
                                    }} 
                                    on:contextmenu={(e) => showContextMenu(e, friend)}
                                    role="button"
                                    tabindex="0"
                                >
                                    {#if editingPeer === friend}
                                        <input bind:value={draftNickname} 
                                            on:blur={() => {
                                                updateNickname(draftNickname.trim());
                                                editingPeer = null;
                                            }}
                                            on:keydown={(e) => {
                                                if (e.key === 'Enter') {
                                                    updateNickname(draftNickname.trim());
                                                    editingPeer = null;
                                                }
                                                if (e.key === 'Escape') {
                                                    editingPeer = null;
                                                }
                                            }}
                                        />
                                    {:else}
                                        {displayName(friend)}
                                    {/if}
                                </li>
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
                <!-- Posts Header with Feed/Board Toggle -->
                <div class="posts-header">
                    <div class="header-left">
                        {#if openBoardPeerId}
                            <button class="back-btn" on:click={viewFeed} title="Back to Feed">
                                <span class="back-arrow">←</span>
                            </button>
                        {/if}
                        <h2>{!openBoardPeerId ? 'Feed' : displayName(openBoardPeerId ?? '')}</h2>
                    </div>
                    {#if openBoardPeerId}
                        <span class="board-label">User Board</span>
                    {/if}
                </div>

                <!-- Posts Container -->
                <div class="posts" use:autoScroll>
                    {#if posts.length === 0}
                        <div class="empty-posts">
                            <div class="empty-content">
                                <p class="empty-emoji">📭</p>
                                <p class="empty-title">{!openBoardPeerId ? 'No posts yet' : 'No posts from this user'}</p>
                                <p class="empty-subtitle">{!openBoardPeerId ? 'Start a conversation!' : 'Be the first to hear from them'}</p>
                            </div>
                        </div>
                        {:else}
                            {#each posts as post}
                                <div class="post">
                                    <div class="post-header">
                                        <div class="post-author">
                                            {#if !openBoardPeerId}
                                                <button class="author-name" on:click={() => viewFriendBoard(post.author_peer_id)}>
                                                    {displayName(post.author_peer_id)}
                                                </button>
                                            {:else}
                                                <span class="author-name">{displayName(post.author_peer_id)}</span>
                                            {/if}
                                        </div>
                                        <span class="post-time">
                                            <span class="post-date">{formatDate(post.edited_at ?? post.created_at)}</span>
                                            <span class="post-clock">🕐</span>
                                        </span>
                                    </div>
                                    <div class="post-content">{post.content}</div>
                                </div>
                            {/each}
                        {/if}
                    </div>

                    <!-- Post Input -->
                    <div class="post-input">
                        <input type="text" bind:value={postInput} on:keypress={(e) => e.key === 'Enter' && sendPost()} placeholder="What's on your mind?" />
                        <button on:click={sendPost} class="btn-primary">Post</button>
                    </div>
                </div>
            </div>

            <!-- Context Menu -->
            {#if contextMenuVisible}
                <div class="context-menu" style="top: {contextMenuY}px; left: {contextMenuX}px;" on:click|stopPropagation on:keydown={(e) => e.key === 'Escape' && hideContextMenu()} role="menu" tabindex="-1">
                    <button class="context-menu-item" on:click={handleViewProfile}>View Profile</button>
                    <button class="context-menu-item" on:click={handleRemoveFriend}>Remove Friend</button>
                    <button class="context-menu-item danger" on:click={handleBlockPeer}>Block</button>
                </div>
            {/if}

            <!-- Relay Settings Modal -->
            {#if showRelaySettings}
                <div class="modal-overlay" on:click={() => showRelaySettings = false} on:keydown={(e) => e.key === 'Escape' && (showRelaySettings = false)} role="presentation">
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div class="modal" on:click|stopPropagation role="dialog" aria-labelledby="relay-settings-modal-title" aria-modal="true" tabindex=0>
                        <h2 id="relay-settings-modal-title">Relay Server Settings</h2>
                        <p>Connect to a relay server to enable connections over the internet</p>
                        <label>
                            Relay Address
                            <input type="text" bind:value={relayAddress} placeholder="/ip4/127.0.0.1/tcp/4001/p2p/12D3Koo..." />
                        </label>
                        <div class="help-text">
                            <strong>Format:</strong> /ip4/IP_ADDRESS/tcp/PORT/p2p/PEER_ID
                            <br />
                            <strong>Example:</strong> /ip4/127.0.0.1/tcp/4001/p2p/12D3KooWABC...
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
                <div class="modal-overlay" on:click={() => showAddFriend = false} on:keydown={(e) => e.key === 'Escape' && (showAddFriend = false)} role="presentation">
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div class="modal" on:click|stopPropagation role="dialog" aria-labelledby="friend-request-modal-title" aria-modal="true" tabindex=0>
                        <h2 id="friend-request-modal-title">Send Friend Request</h2>
                        <p>Enter your friend's connection information</p>
                        <label>
                            Address
                            <input type="text" bind:value={addFriendAddress} placeholder="/ip4/192.168.1.100/tcp/54321/p2p/12D3KooWKNseoB5NSPVcxbPYL..." />
                        </label>
                        <label>
                            Message
                            <input type="text" bind:value={addFriendMessage} placeholder="Hi, let's connect on Enclave!" />
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
                <div class="modal-overlay" on:click={() => showFriendRequests = false} on:keydown={(e) => e.key === 'Escape' && (showFriendRequests = false)} role="presentation">
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div class="modal" on:click|stopPropagation role="dialog" aria-labelledby="friend-requests-modal-title" aria-modal="true" tabindex=0>
                        <h2 id="friend-requests-modal-title">Friend Requests</h2>
                        {#each pendingRequests as peerIdRequestPair}
                            <div class="friend-request">
                                <p class="request-peer">{peerIdRequestPair[0].slice(0, 24)}...</p>
                                <p class="request-message">"{peerIdRequestPair[1].message}"</p>
                                <div class="request-actions">
                                    <button on:click={() => acceptFriendRequest(peerIdRequestPair[0])} class="btn-primary">Accept</button>
                                    <button on:click={() => denyFriendRequest(peerIdRequestPair[0])} class="btn-secondary">Deny</button>
                                </div>
                            </div>
                        {/each}

                        <div class="modal-actions" style="margin-top: 24px;">
                            <button on:click={() => showFriendRequests = false} class="btn-secondary">Close</button>
                        </div>
                    </div>
                </div>
            {/if}

            <!-- Profile Modal -->
            {#if showProfile && profilePeer}
                <div class="modal-overlay" on:click={() => showProfile = false} on:keydown={(e) => e.key === 'Escape' && (showProfile = false)} role="presentation">
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div class="modal" on:click|stopPropagation role="dialog" aria-labelledby="profile-modal-title" aria-modal="true" tabindex=0>
                        <h2 id="profile-modal-title">User Profile</h2>
                        <div class="profile-section">
                            <p>Peer ID</p>
                            <div class="profile-value">
                                <code class="peer-id-full">{profilePeer.peerId}</code>
                                <button on:click={() => copyPeerId(profilePeer!.peerId)} class="btn-icon" title="Copy Peer ID">📋</button>
                            </div>
                        </div>
                        <div class="profile-section">
                            <p>Status</p>
                            <div class="profile-value">
                                {#if profilePeer.isFriend}
                                    <span class="status-badge friend">Friend</span>
                                {:else}
                                    <span class="status-badge">Not a friend</span>
                                {/if}
                            </div>
                        </div>
                        <div class="profile-section">
                            <p>Connection Status</p>
                            <div class="profile-value">
                                {#if connectedPeers.includes(profilePeer.peerId)}
                                    <span class="status-badge connected">Connected</span>
                                {:else}
                                    <span class="status-badge disconnected">Disconnected</span>
                                {/if}
                            </div>
                        </div>
                        <div class="modal-actions">
                            <button on:click={() => showProfile = false} class="btn-secondary">Close</button>
                        </div>
                    </div>
                </div>
            {/if}

            <!-- Direct Messages Launcher -->
            <div class="dm-launcher">
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <div class="dm-header" on:click={() => directMessageOpen = !directMessageOpen}>
                    <span>Messages</span>
                    <span class="chevron">{directMessageOpen ? '▾' : '▴'}</span>
                </div>
                {#if directMessageOpen}
                    <div class="dm-friend-list">
                        {#if friendList.length === 0}
                            <div class="dm-empty">No friends</div>
                        {:else}
                            {#each friendList as friend}
                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                <div class="dm-friend" on:click={() => openDirectMessages(friend)}>
                                    {displayName(friend)}
                                </div>
                            {/each}
                        {/if}
                    </div>
                {/if}
            </div>

            <!-- Active DM Window -->
            {#if activeDMPeerId}
                <div class="dm-window">
                    <div class="dm-window-header">
                        <span>{displayName(activeDMPeerId)}</span>
                        <button on:click={() => activeDMPeerId = null}>✕</button>
                    </div>

                    <div class="dm-messages" use:autoScroll>
                        {#each directMessages as msg}
                            <div class="dm-message {msg.from_peer_id === myPeerId ? 'outgoing' : 'incoming'}">
                                <div class="dm-content">{msg.content}</div>
                                <div class="dm-time">{formatTime(msg.created_at)}</div>
                            </div>
                        {/each}
                    </div>

                    <div class="dm-input">
                        <input type="text" bind:value={directMessageInput} placeholder="Type a message..." on:keydown={(e) => e.key === 'Enter' && sendDirectMessage()} />
                        <button on:click={sendDirectMessage}>Send</button>
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
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue', sans-serif;
    background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
    min-height: 100vh;
  }

  .container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
  }

  header {
    text-align: center;
    margin-bottom: 32px;
  }

  h1 {
    font-size: 48px;
    margin: 0;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    font-weight: 700;
    letter-spacing: -1px;
  }

  .subtitle {
    color: #64748b;
    margin-top: 8px;
    font-weight: 500;
  }

  .welcome {
    text-align: center;
    padding: 80px 20px;
  }

  .welcome p {
    font-size: 18px;
    color: #666;
    margin-bottom: 24px;
  }

  .app {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 20px;
    height: calc(100vh - 250px);
  }

  .sidebar {
    background-color: white;
    border-radius: 16px;
    padding: 24px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    display: flex;
    flex-direction: column;
    gap: 28px;
    overflow-y: auto;
    border: 1px solid rgba(0, 0, 0, 0.05);
  }

  .my-info h3,
  .friends h3 {
    margin: 0 0 12px 0;
    font-size: 15px;
    font-weight: 700;
    color: #1e293b;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .peer-id {
    font-family: 'Monaco', 'Courier New', monospace;
    font-size: 11px;
    color: #64748b;
    background-color: #f8fafc;
    padding: 8px;
    border-radius: 6px;
    margin-bottom: 12px;
    word-break: break-all;
    border: 1px solid #e2e8f0;
  }

  .friends ul {
    list-style: none;
    padding: 0;
    margin: 0 0 12px 0;
  }

  .peer-item {
    padding: 10px 12px;
    background-color: #f8fafc;
    border-radius: 8px;
    margin-bottom: 6px;
    font-size: 13px;
    font-family: 'Monaco', 'Courier New', monospace;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    border: 1px solid transparent;
    color: #475569;
  }
  
  .peer-item:hover {
    background-color: #f1f5f9;
    border-color: #cbd5e1;
  }

  .peer-item.active {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    border-color: transparent;
    font-weight: 600;
  }

  .peer-item input {
    width: 100%;
    padding: 4px 0;
    border: none;
    background: transparent;
    color: inherit;
    font-family: inherit;
    font-size: inherit;
  }

  .main-content {
    background-color: white;
    border-radius: 16px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    display: flex;
    flex-direction: column;
    border: 1px solid rgba(0, 0, 0, 0.05);
    overflow: hidden;
  }

  .posts-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid #f1f5f9;
    background: linear-gradient(to bottom, #fafbfc, #ffffff);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .back-btn {
    background: none;
    border: none;
    font-size: 20px;
    cursor: pointer;
    padding: 4px;
    color: #667eea;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .back-btn:hover {
    color: #764ba2;
    transform: translateX(-2px);
  }

  .back-arrow {
    display: inline-block;
  }

  .posts-header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    color: #1e293b;
  }

  .board-label {
    font-size: 12px;
    font-weight: 600;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .posts {
    flex: 1;
    overflow-y: auto;
    padding: 0;
    display: flex;
    flex-direction: column;
    padding: 20px;
  }

  .empty-posts {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-content {
    text-align: center;
  }

  .empty-emoji {
    font-size: 64px;
    margin: 0 0 16px 0;
    opacity: 0.6;
  }

  .empty-title {
    font-size: 20px;
    font-weight: 600;
    color: #475569;
    margin: 0 0 8px 0;
  }

  .empty-subtitle {
    font-size: 14px;
    color: #94a3b8;
    margin: 0;
  }

  .post {
    margin-bottom: 16px;
    padding: 16px;
    background-color: #f8fafc;
    border-radius: 10px;
    border: 1px solid #e2e8f0;
    transition: all 0.2s;
  }

  .post:hover {
    border-color: #cbd5e1;
    background-color: #ffffff;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  }

  .post-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .post-author {
    display: flex;
    align-items: center;
  }

  .author-name {
    font-weight: 600;
    color: #667eea;
    font-size: 14px;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    transition: color 0.2s;
  }

  .author-name:hover {
    color: #764ba2;
    text-decoration: underline;
  }

  .post-time {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: #94a3b8;
  }

  .post-date {
    font-weight: 500;
  }

  .post-clock {
    font-size: 14px;
  }

  .post-content {
    color: #475569;
    line-height: 1.5;
    word-break: break-word;
  }

  .post-input {
    display: flex;
    gap: 10px;
    padding: 20px 24px;
    border-top: 1px solid #e2e8f0;
    background: #f8fafc;
  }

  .post-input input {
    flex: 1;
    padding: 12px 16px;
    border: 1px solid #cbd5e1;
    border-radius: 10px;
    font-size: 14px;
    transition: all 0.2s;
    background: white;
    color: #1e293b;
  }

  .post-input input:focus {
    outline: none;
    border-color: #667eea;
    box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
  }

  .post-input input::placeholder {
    color: #cbd5e1;
  }

  .btn-primary {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    border: none;
    padding: 12px 24px;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 4px 8px rgba(102, 126, 234, 0.3);
  }

  .btn-primary:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 12px rgba(102, 126, 234, 0.4);
  }

  .btn-primary:active {
    transform: translateY(0);
  }

  .btn-secondary {
    background-color: white;
    color: #475569;
    border: 1px solid #cbd5e1;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.2s;
    width: 100%;
    font-weight: 500;
  }

  .btn-secondary:hover {
    background-color: #f1f5f9;
    border-color: #94a3b8;
  }

  .empty {
    color: #94a3b8;
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
    background-color: rgba(15, 23, 42, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal {
    background-color: white;
    padding: 32px;
    border-radius: 16px;
    max-width: 500px;
    width: 90%;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.15);
  }

  .modal h2 {
    margin: 0 0 8px 0;
    color: #1e293b;
    font-size: 24px;
  }

  .modal p {
    color: #64748b;
    margin: 0 0 24px 0;
  }

  .modal label {
    display: block;
    margin-bottom: 16px;
    font-weight: 600;
    color: #1e293b;
    font-size: 14px;
  }

  .modal input {
    display: block;
    width: 100%;
    padding: 12px;
    border: 1px solid #cbd5e1;
    border-radius: 8px;
    font-size: 14px;
    margin-top: 8px;
    box-sizing: border-box;
    background: white;
    transition: all 0.2s;
    color: #1e293b;
  }

  .modal input:focus {
    outline: none;
    border-color: #667eea;
    box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
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
    background-color: #f8fafc;
    border-radius: 8px;
    margin-bottom: 12px;
    border: 1px solid #e2e8f0;
  }

  .request-peer {
    font-family: 'Monaco', 'Courier New', monospace;
    font-size: 12px;
    color: #475569;
    margin: 0 0 8px 0;
    font-weight: 600;
  }

  .request-message {
    color: #64748b;
    font-style: italic;
    margin: 0 0 12px 0;
    font-size: 14px;
  }

  .request-actions {
    display: flex;
    gap: 8px;
  }

  .request-actions button {
    flex: 1;
  }

  .help-text {
    background-color: #f1f5f9;
    padding: 12px;
    border-radius: 8px;
    font-size: 12px;
    color: #64748b;
    margin-top: 12px;
    font-family: 'Monaco', 'Courier New', monospace;
    border: 1px solid #e2e8f0;
  }

  .help-text strong {
    color: #1e293b;
  }

  .context-menu {
    position: fixed;
    background-color: white;
    border-radius: 10px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.15);
    padding: 6px 0;
    min-width: 160px;
    z-index: 10000;
    border: 1px solid #e2e8f0;
  }

  .context-menu-item {
    display: block;
    width: 100%;
    padding: 10px 16px;
    background: none;
    border: none;
    text-align: left;
    font-size: 14px;
    cursor: pointer;
    transition: background-color 0.15s;
    color: #475569;
  }

  .context-menu-item:hover {
    background-color: #f1f5f9;
  }

  .context-menu-item.danger {
    color: #e11d48;
  }

  .context-menu-item.danger:hover {
    background-color: #ffe4e6;
  }

  .profile-section {
    margin-bottom: 20px;
  }

  .profile-section p {
    display: block;
    font-weight: 600;
    color: #1e293b;
    margin-bottom: 8px;
    font-size: 14px;
  }

  .profile-value {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .peer-id-full {
    font-family: 'Monaco', 'Courier New', monospace;
    font-size: 11px;
    background-color: #f1f5f9;
    padding: 8px;
    border-radius: 6px;
    word-break: break-all;
    flex: 1;
    border: 1px solid #e2e8f0;
    color: #475569;
  }

  .btn-icon {
    background: white;
    border: 1px solid #cbd5e1;
    padding: 6px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 16px;
    transition: all 0.2s;
  }

  .btn-icon:hover {
    background-color: #f1f5f9;
    border-color: #94a3b8;
  }

  .status-badge {
    display: inline-block;
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 600;
  }

  .status-badge.friend {
    background-color: #dcfce7;
    color: #166534;
  }

  .status-badge.connected {
    background-color: #dbeafe;
    color: #1e40af;
  }

  .status-badge.disconnected {
    background-color: #f1f5f9;
    color: #64748b;
  }

  .dm-launcher {
    position: fixed;
    bottom: 0;
    right: 16px;
    width: 260px;
    background: white;
    border-radius: 12px 12px 0 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    overflow: hidden;
    z-index: 9000;
    font-size: 14px;
    border: 1px solid #e2e8f0;
  }

  .dm-header {
    padding: 12px;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
  }

  .dm-friend-list {
    max-height: 300px;
    overflow-y: auto;
    background-color: #f8fafc;
  }

  .dm-friend {
    padding: 10px 12px;
    font-family: 'Monaco', 'Courier New', monospace;
    cursor: pointer;
    border-bottom: 1px solid #e2e8f0;
    font-size: 13px;
    color: #475569;
    transition: background-color 0.15s;
  }

  .dm-friend:hover {
    background-color: #f1f5f9;
  }

  .dm-empty {
    padding: 16px;
    text-align: center;
    color: #94a3b8;
    font-size: 13px;
  }

  .dm-window {
    position: fixed;
    bottom: 0;
    right: 292px;
    width: 320px;
    height: 420px;
    background-color: #fff;
    border-radius: 12px 12px 0 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    display: flex;
    flex-direction: column;
    z-index: 9000;
    border: 1px solid #e2e8f0;
  }

  .dm-window-header {
    padding: 12px;
    font-weight: 600;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: linear-gradient(to bottom, #fafbfc, #ffffff);
    border-bottom: 1px solid #e2e8f0;
    color: #1e293b;
  }

  .dm-window-header button {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 16px;
    color: #94a3b8;
    transition: color 0.2s;
  }

  .dm-window-header button:hover {
    color: #475569;
  }

  .dm-messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .dm-message {
    max-width: 75%;
  }

  .dm-message.incoming {
    align-self: flex-start;
  }

  .dm-message.outgoing {
    align-self: flex-end;
    text-align: right;
  }

  .dm-content { 
    background-color: #f1f5f9;
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 13px;
    color: #475569;
  }

  .dm-message.outgoing .dm-content {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
  }

  .dm-time {
    font-size: 10px;
    color: #94a3b8;
    margin-top: 4px;
  }

  .dm-input {
    display: flex;
    gap: 8px;
    padding: 10px;
    border-top: 1px solid #e2e8f0;
    background: #f8fafc;
  }

  .dm-input input {
    flex: 1;
    padding: 8px;
    font-size: 13px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: white;
    color: #1e293b;
  }

  .dm-input input:focus {
    outline: none;
    border-color: #667eea;
  }

  .dm-input button {
    padding: 8px 12px;
    border: none;
    border-radius: 6px;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: #fff;
    cursor: pointer;
    font-weight: 600;
    font-size: 12px;
    transition: transform 0.2s;
  }

  .dm-input button:hover {
    transform: translateY(-1px);
  }
</style>