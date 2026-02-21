<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';
    import type { FriendRequest, NodeInfo, Post } from '$lib/types';
    import { connectedPeers, contextMenuPeer, contextMenuVisible, directMessages, friendList, isStarted, myInfo, openBoardPeerId, pendingRequests, posts, profilePeer, showFriendRequestsModal, showProfileModal, showRelaySettingsModal, showSendFriendRequestModal } from '$lib/state';
    import Sidebar from '$lib/components/Sidebar.svelte';
    import Posts from '$lib/components/Posts.svelte';
    import ContextMenu from '$lib/components/ContextMenu.svelte';
    import RelaySettingsModal from '$lib/components/modals/RelaySettingsModal.svelte';
    import SendFriendRequestModal from '$lib/components/modals/SendFriendRequestModal.svelte';
    import FriendRequestsModal from '$lib/components/modals/FriendRequestsModal.svelte';
    import ProfileModal from '$lib/components/modals/ProfileModal.svelte';

    import '../app.css';
    import DirectMessages from '$lib/components/DirectMessages.svelte';

    onMount(async () => {
        if (!('__TAURI_INTERNALS__' in window)) {
            console.warn('Tauri API not available');
            return;
        }

        // Listen for P2P events:
        await listen('post-received', (event: any) => {
            posts.update(p => [...p, event.payload]);
        });

        await listen('post-sent', (event: any) => {
            posts.update(p => [...p, event.payload]);
        });

        await listen('dm-received', (event: any) => {
            directMessages.update(dms => [...dms, event.payload]);
        });

        await listen('dm-sent', (event: any) => {
            directMessages.update(dms => [...dms, event.payload]);
        });

        await listen('peer-connected', (event: any) => {
            const peerId = event.payload as string;
            if (!$connectedPeers.includes(peerId)) {
                connectedPeers.update(p => [...p, peerId]);
            }
        });

        await listen('peer-disconnected', (event: any) => {
            const peerId = event.payload as string;
            connectedPeers.update(c => c.filter(p => p !== peerId));
        });

        await listen('friend-request-received', (event: any) => {
            const [ peerId, request ] = event.payload;
            if (!$pendingRequests.some(r => r[0] === peerId)) {
                pendingRequests.update(p => [...p, [peerId, request]]);
                showFriendRequestsModal.set(true);
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
            await invoke<string>('start_p2p');
            myInfo.set(await invoke<NodeInfo>('get_my_info'));
            isStarted.set(true);
        }
        catch (error) {
            console.error('Failed to start P2P:', error);
            alert('Failed to start P2P: ' + error);
        }
    }

    async function refreshFriendList() {
        try {
            friendList.set(await invoke<string[]>('get_friend_list'));
        } catch (error) {
            console.error('Failed to get friend list:', error);
        }
    }

    async function refreshInboundFriendRequests() {
        try {
            pendingRequests.set(await invoke<[string, FriendRequest][]>('get_inbound_friend_requests'));
        } catch (error) {
            console.error('Failed to get inbound friend requests:', error);
        }
    }

    async function loadFeed() {
        try {
            posts.set(await invoke<Post[]>('load_feed'));
        } catch (error) {
            console.error('Failed to load feed:', error);
        }
    }

    async function loadBoard() {
        try {
            posts.set(await invoke<Post[]>('load_board', { peerId: $openBoardPeerId }));
        } catch (error) {
            console.error('Failed to load board:', error);
        }
    }
    
    function handleClickOutside(event: Event) {
        if ($contextMenuVisible) {
            contextMenuPeer.set(null);
            contextMenuVisible.set(false);
        }
    }

    function viewFriendBoard(peerId: string) {
        openBoardPeerId.set(peerId);
        loadBoard();
    }

    function viewFeed() {
        openBoardPeerId.set(null);
        loadFeed();
    }
</script>

<svelte:window 
    on:click={handleClickOutside} 
    on:keydown={(e) => {
        if (e.key === 'Escape') {
            contextMenuVisible.set(false);
            contextMenuPeer.set(null);
            showProfileModal.set(false);
            showProfileModal.set(false);
            showSendFriendRequestModal.set(false);
            showFriendRequestsModal.set(false);
            showRelaySettingsModal.set(false);
        }
    }} 
/>

<main>
    <div class="container">
        <header>
            <h1>Enclave</h1>
            <p class="subtitle">The P2P social media platform - where you control your data.</p>
        </header>

        {#if !$isStarted}
            <div class="welcome">
                <p>Start your private network to connect with friends</p>
                <button on:click={startP2P} class="btn-primary">Start Enclave</button>
            </div>
        {:else}
            <div class="app">
                <Sidebar {viewFriendBoard} />
                <div class="main-content">
                    <Posts {viewFriendBoard} {viewFeed} />
                </div>
            </div>

            {#if $contextMenuVisible}
                <ContextMenu />
            {/if}

            {#if $showRelaySettingsModal} 
                <RelaySettingsModal />
            {/if}

            {#if $showSendFriendRequestModal}
                <SendFriendRequestModal />
            {/if}

            {#if $showFriendRequestsModal}
                <FriendRequestsModal {refreshFriendList} />
            {/if}

            {#if $showProfileModal && $profilePeer}
                <ProfileModal />
            {/if}

            <DirectMessages />
        {/if}
    </div>
</main>