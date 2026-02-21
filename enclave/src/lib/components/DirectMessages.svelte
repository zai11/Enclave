<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { autoScroll, displayName, formatTime } from "../utils";
    import { activeDMPeerId, directMessageInput, directMessageOpen, directMessages, friendList, myInfo, nicknames } from "$lib/state";
    import type { DirectMessage } from "$lib/types";

    async function sendDirectMessage() {
        if (!$directMessageInput.trim()) return;

        try {
            await invoke('send_direct_message', { peerId: $activeDMPeerId!, content: $directMessageInput });
            directMessageInput.set('');
        } catch (error) {
            console.error('Failed to send message:', error);
            alert('Failed to send message: ' + error);
        }
    }

    async function openDirectMessages(peerId: string) {
        try {
            activeDMPeerId.set(peerId);
            directMessages.set(await invoke<DirectMessage[]>('get_direct_messages', { peerId: $activeDMPeerId }));
            directMessageOpen.set(true);
        } catch (error) {
            console.error('Failed to get direct messages:', error);
            alert('Failed to get direct messages: ' + error);
        }
    }
</script>

<div class="dm-launcher">
    <div 
        class="dm-header" 
        on:click={() => directMessageOpen.set(!$directMessageOpen)} 
        on:keydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                directMessageOpen.set(!$directMessageOpen);
            }
        }}
        role="button" 
        tabindex="0" 
        aria-expanded="{$directMessageOpen}"
    >
        <span>Messages</span>
        <span class="chevron">{$directMessageOpen ? '▾' : '▴'}</span>
    </div>
    {#if $directMessageOpen}
        <div class="dm-friend-list">
            {#if $friendList.length === 0}
                <div class="dm-empty">No friends</div>
            {:else}
                {#each $friendList as friend}
                    <div 
                        class="dm-friend" 
                        on:click={() => openDirectMessages(friend)}
                        on:keydown={(e) => {
                            if (e.key === 'Enter' || e.key === ' ') {
                                e.preventDefault();
                                openDirectMessages(friend)
                            }
                        }}
                        role="button" 
                        tabindex="0" 
                        aria-expanded="{$directMessageOpen}"
                    >
                        {displayName(friend, $nicknames, $myInfo?.peerId ?? '')}
                    </div>
                {/each}
            {/if}
        </div>
    {/if}
</div>

{#if $activeDMPeerId}
    <div class="dm-window">
        <div class="dm-window-header">
            <span>{displayName($activeDMPeerId, $nicknames, $myInfo?.peerId ?? '')}</span>
            <button on:click={() => activeDMPeerId.set(null)}>✕</button>
        </div>

        <div class="dm-messages" use:autoScroll>
            {#each $directMessages as msg}
                <div class="dm-message {msg.fromPeerId === $myInfo?.peerId ? 'outgoing' : 'incoming'}">
                    <div class="dm-content">{msg.content}</div>
                    <div class="dm-time">{formatTime(msg.createdAt)}</div>
                </div>
            {/each}
        </div>

        <div class="dm-input">
            <input type="text" bind:value={$directMessageInput} placeholder="Type a message..." on:keydown={(e) => e.key === 'Enter' && sendDirectMessage()} />
            <button on:click={sendDirectMessage}>Send</button>
        </div>
    </div>
{/if}

<style>
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