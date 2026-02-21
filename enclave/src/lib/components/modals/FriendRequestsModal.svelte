<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { pendingRequests, showFriendRequestsModal, showSendFriendRequestModal } from "$lib/state";

    export let refreshFriendList: () => void;

    async function acceptFriendRequest(peerId: string) {
        try {
            await invoke('accept_friend_request', { peerId });
            pendingRequests.update(p => p.filter(r => r[0] !== peerId));
            if ($pendingRequests.length === 0) {
                showFriendRequestsModal.set(false);
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
            pendingRequests.update(p => p.filter(r => r[0] !== peerId));
            if ($pendingRequests.length === 0) {
                showFriendRequestsModal.set(false);
            }
        } catch (error) {
            console.error('Failed to deny friend request:', error);
            alert('Failed to deny friend request: ' + error);
        }
    }
</script>

<div class="modal-overlay" 
    on:click={() => showFriendRequestsModal.set(false)} 
    on:keydown={(e) => e.key === 'Escape' && showSendFriendRequestModal.set(false)} 
    role="presentation"
>
    <div class="modal" role="dialog" aria-labelledby="friend-requests-modal-title" aria-modal="true" tabindex=0>
        <h2 id="friend-requests-modal-title">Friend Requests</h2>
        {#each $pendingRequests as peerIdRequestPair}
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
            <button on:click={() => showFriendRequestsModal.set(false)} class="btn-secondary">Close</button>
        </div>
    </div>
</div>

<style>
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
</style>