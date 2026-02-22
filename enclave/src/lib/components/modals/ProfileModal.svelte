<script lang="ts">
    import { connectedPeers, profilePeer, showProfileModal } from "$lib/state";

    function copyPeerId(peerId: string) {
        navigator.clipboard.writeText(peerId);
        alert('Peer ID copied to clipboard!');
    }
</script>

<div 
    class="modal-overlay" 
    on:click={() => showProfileModal.set(false)} 
    on:keydown={(e) => e.key === 'Escape' && showProfileModal.set(false)} 
    role="presentation"
>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal" on:click|stopPropagation role="dialog" aria-labelledby="profile-modal-title" aria-modal="true" tabindex=0>
        <h2 id="profile-modal-title">User Profile</h2>
        <div class="profile-section">
            <p>Peer ID</p>
            <div class="profile-value">
                <code class="peer-id-full">{$profilePeer!.peerId}</code>
                <button on:click={() => copyPeerId($profilePeer!.peerId)} class="btn-icon" title="Copy Peer ID">📋</button>
            </div>
        </div>
        <div class="profile-section">
            <p>Status</p>
            <div class="profile-value">
                {#if $profilePeer!.isFriend}
                    <span class="status-badge friend">Friend</span>
                {:else}
                    <span class="status-badge">Not a friend</span>
                {/if}
            </div>
        </div>
        <div class="profile-section">
            <p>Connection Status</p>
            <div class="profile-value">
                {#if $connectedPeers.includes($profilePeer!.peerId)}
                    <span class="status-badge connected">Connected</span>
                {:else}
                    <span class="status-badge disconnected">Disconnected</span>
                {/if}
            </div>
        </div>
        <div class="modal-actions">
            <button on:click={() => showProfileModal.set(false)} class="btn-secondary">Close</button>
        </div>
    </div>
</div>

<style>
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
</style>