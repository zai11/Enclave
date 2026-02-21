<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { validateMultiaddr } from "../../utils";
    import { addFriendAddress, addFriendMessage, showSendFriendRequestModal } from "$lib/state";

    async function sendFriendRequest() {
        if (!$addFriendAddress.trim()) {
            alert('Please enter friend connection address address');
            return;
        }

        if (!validateMultiaddr($addFriendAddress)) {
            alert('The provided address was invalid');
            return;
        }

        const peerId = $addFriendAddress.split('/p2p/')[1];
        const multiaddr = $addFriendAddress.split('/p2p/')[0];

        try {
            await invoke('send_friend_request', {
                peerId: peerId,
                multiaddr: multiaddr,
                message: $addFriendMessage || 'Hi, let\'s connect on Enclave!'
            });
            addFriendAddress.set('');
            addFriendMessage.set('');
            showSendFriendRequestModal.set(false);
            alert('Friend request sent!');
        } catch (error) {
            console.error('Failed to send friend request:', error);
            alert('Failed to send friend request: ' + error);
        }
    }
</script>

<div 
    class="modal-overlay" 
    on:click={() => showSendFriendRequestModal.set(false)} 
    on:keydown={(e) => e.key === 'Escape' && showSendFriendRequestModal.set(false)}
    role="presentation"
>
    <div class="modal" role="dialog" aria-labelledby="friend-request-modal-title" aria-modal="true" tabindex=0>
        <h2 id="friend-request-modal-title">Send Friend Request</h2>
        <p>Enter your friend's connection information</p>
        <label>
            Address
            <input type="text" bind:value={$addFriendAddress} placeholder="/ip4/192.168.1.100/tcp/54321/p2p/12D3KooWKNseoB5NSPVcxbPYL..." />
        </label>
        <label>
            Message
            <input type="text" bind:value={$addFriendMessage} placeholder="Hi, let's connect on Enclave!" />
        </label>
        <div class="modal-actions">
            <button on:click={() => showSendFriendRequestModal.set(false)} class="btn-secondary">Cancel</button>
            <button on:click={sendFriendRequest} class="btn-primary">Send Request</button>
        </div>
    </div>
</div>