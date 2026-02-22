<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import type { NodeInfo } from "../../types";
    import { myInfo, relayAddress, showRelaySettingsModal } from "$lib/state";

    async function connectRelay() {
        if (!$relayAddress.trim()) {
            alert('Please enter a relay address');
            return;
        }

        try {
            await invoke('connect_to_relay', {relayAddress: $relayAddress});
            myInfo.set(await invoke<NodeInfo>('get_my_info'));
            showRelaySettingsModal.set(false);
            alert('Connected to relay successfully!');
        } catch (error) {
            console.error('Failed to connect to relay:', error);
            alert('Failed to connect to relay: ' + error);
        }
    }
</script>

<div 
    class="modal-overlay" 
    on:click={() => showRelaySettingsModal.set(false)} 
    on:keydown={(e) => e.key === 'Escape' && showRelaySettingsModal.set(false)}
    role="presentation"
>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal" on:click|stopPropagation role="dialog" aria-labelledby="relay-settings-modal-title" aria-modal="true" tabindex=0>
        <h2 id="relay-settings-modal-title">Relay Server Settings</h2>
        <p>Connect to a relay server to enable connections over the internet</p>
        <label>
            Relay Address
            <input type="text" bind:value={$relayAddress} placeholder="/ip4/127.0.0.1/tcp/4001/p2p/12D3Koo..." />
        </label>
        <div class="help-text">
            <strong>Format:</strong> /ip4/IP_ADDRESS/tcp/PORT/p2p/PEER_ID
            <br />
            <strong>Example:</strong> /ip4/127.0.0.1/tcp/4001/p2p/12D3KooWABC...
        </div>
        <div class="modal-actions">
            <button on:click={() => showRelaySettingsModal.set(false)} class="btn-secondary">Cancel</button>
            <button on:click={connectRelay} class="btn-primary">Connect</button>
        </div>
    </div>
</div>