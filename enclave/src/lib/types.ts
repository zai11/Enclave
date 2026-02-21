export interface Post {
    authorPeerId: string;
    content: string;
    createdAt: number;
    editedAt: number;
}

export interface DirectMessage {
    fromPeerId: string;
    toPeerId: string;
    content: string;
    createdAt: number;
    editedAt: number;
    read: boolean;
}

export interface NodeInfo {
    peerId: string;
    multiaddr: string;
    isFriend: boolean;
}

export interface FriendRequest {
    fromPeerId: string;
    fromMultiaddr: string;
    message: string;
}

export interface AppState {
    myInfo: NodeInfo | null;
    connectedPeers: string[];
    friendList: string[];
    pendingRequests: Array<[string, FriendRequest]>;
    postInput: string;
    addFriendAddress: string;
    addFriendMessage: string;
    relayAddress: string;
    isStarted: boolean;
    showSendFriendRequestModal: boolean;
    showFriendRequestsModal: boolean;
    showRelaySettingsModal: boolean;
    showProfileModal: boolean;
    contextMenuVisible: boolean;
    contextMenuLocation: [number, number];
    contextMenuPeer: string | null;
    profilePeer: { peerId: string, isFriend: boolean } | null;
    nicknames: Map<string, string>;
    editingPeer: string | null;
    draftNickname: string;
    directMessageOpen: boolean;
    activeDMPeerId: string | null;
    directMessages: DirectMessage[];
    directMessageInput: string;
    openBoardPeerId: string | null;
    posts: Post[];
}