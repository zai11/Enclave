<script lang="ts">
    import { contextMenuLocation, contextMenuPeer, contextMenuVisible, draftNickname, editingPeer, friendList, myInfo, nicknames, openBoardPeerId, pendingRequests, showFriendRequestsModal, showRelaySettingsModal, showSendFriendRequestModal } from '$lib/state';
    import { displayName } from '../utils';

    export let viewFriendBoard: (peerId: string) => void;

    function copyMyAddress() {
        if (!$myInfo) return;
        const text = `${$myInfo.multiaddr}/p2p/${$myInfo.peerId}`;
        navigator.clipboard.writeText(text);
        alert('Connection info copied to clipboard!');
    }

    function showContextMenu(event: MouseEvent, peerId: string) {
        event.preventDefault();
        $contextMenuPeer = peerId;
        $contextMenuLocation = [event.clientX, event.clientY];
        $contextMenuVisible = true;
    }

    function updateNickname(newName: string) {
        if ($editingPeer) {
            if (newName === '' && $nicknames.has($editingPeer)) {
                nicknames.update(n => {
                    const copy = new Map(n);
                    copy.delete($editingPeer);
                    return copy;
                });
            }
            else {
                nicknames.update(n => {
                    const copy = new Map(n);
                    copy.set($editingPeer, newName);
                    return copy;
                });
            }
        }
    }
</script>

<aside class="sidebar">
    <div class="my-info">
        <h3>My Info</h3>
        <p class="peer-id">{$myInfo?.peerId.slice(0, 16)}...</p>
        <button on:click={copyMyAddress} class="btn-secondary">Copy Connection Info</button>
        <button on:click={() => showRelaySettingsModal.set(true)} class="btn-secondary" style="margin-top: 8px;">Relay Settings</button>
    </div>
    <div class="friends">
        <h3>Friends ({$friendList.length})</h3>
        {#if $friendList.length === 0}
            <p class="empty">No friends yet</p>
        {:else}
            <ul>
                {#each $friendList as friend}
                    <li>
                        <button
                            class="peer-item"
                            class:active={$openBoardPeerId === friend}
                            aria-pressed={$openBoardPeerId === friend}
                            on:click={() => viewFriendBoard(friend)}
                            on:dblclick={(e) => {
                                e.stopPropagation();
                                editingPeer.set(friend);
                                draftNickname.set($nicknames.get(friend) ?? friend);
                            }}
                            on:contextmenu={(e) => showContextMenu(e, friend)}
                        >
                            {#if $editingPeer === friend}
                                <input
                                    bind:value={$draftNickname}
                                    on:blur={() => {
                                        updateNickname($draftNickname.trim());
                                        editingPeer.set(null);
                                    }}
                                    on:keydown={(e) => {
                                        if (e.key === 'Enter') {
                                        updateNickname($draftNickname.trim());
                                        editingPeer.set(null);
                                        }
                                        if (e.key === 'Escape') {
                                        editingPeer.set(null);
                                        }
                                    }}
                                />
                            {:else}
                                {displayName(friend, $nicknames, $myInfo?.peerId ?? '')}
                            {/if}
                        </button>
                    </li>
                {/each}
            </ul>
        {/if}
        <button on:click={() => showSendFriendRequestModal.set(!$showSendFriendRequestModal)} class="btn-secondary">Send Friend Request</button>
        {#if $pendingRequests.length > 0}
            <button on:click={() => showFriendRequestsModal.set(!$showFriendRequestsModal)} class="btn-primary" style="margin-top: 8px;">Friend Requests ({$pendingRequests.length})</button>
        {/if}
    </div>
</aside>

<style>
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

    .friends li {
        margin-bottom: 6px;
    }

    .peer-item {
        all: unset;
        display: block;
        width: 100%;
        box-sizing: border-box;

        padding: 10px 12px;
        background-color: #f8fafc;
        border-radius: 8px;

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

    .peer-item:focus-visible {
        outline: 2px solid #667eea;
        outline-offset: 2px;
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
    
    .empty {
        color: #94a3b8;
        font-size: 14px;
        text-align: center;
        padding: 20px 0;
    }
</style>