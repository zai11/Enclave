export function formatTime(timestamp: number) {
    console.log(timestamp);
    return new Date(timestamp * 1000).toLocaleTimeString();
}

export function formatDate(timestamp: number) {
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

export function displayName(peerId: string, nicknames: Map<string, string>, myPeerId: string): string {
    console.log(peerId);
    let name = nicknames.get(peerId) ?? peerId;
    if (name.length > 24) {
        return name.slice(0, 24) + '...' + (peerId === myPeerId ? ' (Me)' : '');
    }
    else {
        return name + (peerId === myPeerId ? ' (Me)' : '');
    }
}

export function validateMultiaddr(multiaddr: string): boolean {
    const multiaddrRegex = /^\/ip4\/(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\/tcp\/([1-9]\d{0,4})\/p2p\/([A-Za-z0-9]+)$/
    return multiaddr.match(multiaddrRegex) !== null
}

export function autoScroll(node: HTMLElement) {
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