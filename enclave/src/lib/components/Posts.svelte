<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import {formatDate, displayName, autoScroll } from '../utils'
    import { myInfo, nicknames, openBoardPeerId, postInput, posts } from '$lib/state';

    export let viewFeed: () => void;
    export let viewFriendBoard: (peerId: string) => void;

    async function sendPost() {
        if (!$postInput.trim()) return;

        try {
            await invoke('send_post', { content: $postInput });
            postInput.set('');
        } catch (error) {
            console.error('Failed to send post:', error);
            alert('Failed to send post: ' + error);
        }
    }
</script>

<div class="posts-header">
    <div class="header-left">
        {#if $openBoardPeerId}
            <button class="back-btn" on:click={viewFeed} title="Back to Feed">
                <span class="back-arrow">←</span>
            </button>
        {/if}
        <h2>{!$openBoardPeerId ? 'Feed' : displayName($openBoardPeerId ?? '', $nicknames, $myInfo?.peerId ?? '')}</h2>
    </div>
    {#if $openBoardPeerId}
        <span class="board-label">User Board</span>
    {/if}
</div>

<div class="posts" use:autoScroll>
    {#if $posts.length === 0}
        <div class="empty-posts">
            <div class="empty-content">
                <p class="empty-emoji">📭</p>
                <p class="empty-title">{!$openBoardPeerId ? 'No posts yet' : 'No posts from this user'}</p>
                <p class="empty-subtitle">{!$openBoardPeerId ? 'Start a conversation!' : 'Be the first to hear from them'}</p>
            </div>
        </div>
    {:else}
        {#each $posts as post}
            <div class="post">
                <div class="post-header">
                    <div class="post-author">
                        {#if !$openBoardPeerId}
                            <button class="author-name" on:click={() => viewFriendBoard(post.authorPeerId)}>
                                {displayName(post.authorPeerId, $nicknames, $myInfo?.peerId ?? '')}
                            </button>
                        {:else}
                            <span class="author-name">{displayName(post.authorPeerId, $nicknames, $myInfo?.peerId ?? '')}</span>
                        {/if}
                    </div>
                    <span class="post-time">
                        <span class="post-date">{formatDate(post.editedAt ?? post.createdAt)}</span>
                        <span class="post-clock">🕐</span>
                    </span>
                </div>
                <div class="post-content">{post.content}</div>
            </div>
        {/each}
    {/if}
</div>

<div class="post-input">
    <input type="text" bind:value={$postInput} on:keypress={(e) => e.key === 'Enter' && sendPost()} placeholder="What's on your mind?" />
    <button on:click={sendPost} class="btn-primary">Post</button>
</div>

<style>
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
</style>