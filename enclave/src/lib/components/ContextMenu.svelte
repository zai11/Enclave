<script lang="ts">
    import { contextMenuLocation, contextMenuPeer, contextMenuVisible, draftNickname, editingPeer, friendList, nicknames, profilePeer } from "$lib/state";

    function hideContextMenu() {
        contextMenuPeer.set(null);
        contextMenuVisible.set(false);
    }

    async function handleViewProfile() {
        if ($contextMenuPeer) {
            profilePeer.set({
                peerId: $contextMenuPeer!,
                isFriend: $friendList.includes($contextMenuPeer!)
            });
        }

        hideContextMenu();
    }

    function handleRemoveFriend() {
        if ($contextMenuPeer) {
            // TODO: Implement remove friend functionality
            alert(`Remove friend: ${$contextMenuPeer.slice(0, 16)}...`);
        }
        hideContextMenu();
    }

    function handleBlockPeer() {
        if ($contextMenuPeer) {
            // TODO: Implement peer block functionality
            alert(`Block friend: ${$contextMenuPeer.slice(0, 16)}...`);
        }
        hideContextMenu();
    }

    function handleChangeNickname() {
        if ($contextMenuPeer) {
            editingPeer.set($contextMenuPeer);
            draftNickname.set($nicknames.get($contextMenuPeer) ?? $contextMenuPeer);
        }
        hideContextMenu();
    }
</script>

<div class="context-menu" style="top: {$contextMenuLocation[1]}px; left: {$contextMenuLocation[0]}px;" on:click|stopPropagation on:keydown={(e) => e.key === 'Escape' && hideContextMenu()} role="menu" tabindex="-1">
    <button class="context-menu-item" on:click={handleViewProfile}>View Profile</button>
    <button class="context-menu-item" on:click={handleChangeNickname}>Change Nickname</button>
    <button class="context-menu-item" on:click={handleRemoveFriend}>Remove Friend</button>
    <button class="context-menu-item danger" on:click={handleBlockPeer}>Block</button>
</div>

<style>
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
</style>