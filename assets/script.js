// --- 1. CONFIG & VARIABLES ---
let isLoginMode = true;
let currentUser = null;
let token = localStorage.getItem('token');
let ws = null;
let chatMode = 'CHANNEL';
let chatTarget = 1;
let typingTimeout = null;

// Markdown Config
marked.setOptions({ breaks: true });

// Audio Config (Volume 30%)
const audioSent = new Audio('https://assets.mixkit.co/active_storage/sfx/2346/2346-preview.mp3');
const audioReceived = new Audio('https://assets.mixkit.co/active_storage/sfx/2869/2869-preview.mp3');
audioSent.volume = 0.3;
audioReceived.volume = 0.3;

let isMuted = false;

// --- 2. INIT ---
if (token) {
    currentUser = localStorage.getItem('username');
    showChatScreen();
}

// Helpers
function getAvatarUrl(u) { return `https://api.dicebear.com/9.x/notionists/svg?seed=${u}&backgroundColor=b6e3f4,c0aede`; }

function playSound(type) {
    if (isMuted) return;
    const audio = type === 'sent' ? audioSent : audioReceived;
    audio.currentTime = 0;
    audio.play().catch(e => console.log("Cần tương tác trang để phát âm thanh"));
}

function toggleMute() {
    isMuted = !isMuted;
    const btn = document.getElementById('btn-deafen');
    if (isMuted) {
        btn.classList.add('text-discord-danger'); btn.classList.remove('text-discord-text');
        btn.innerHTML = '<i class="fa-solid fa-ear-deaf text-xs"></i>';
    } else {
        btn.classList.remove('text-discord-danger'); btn.classList.add('text-discord-text');
        btn.innerHTML = '<i class="fa-solid fa-headphones text-xs"></i>';
    }
}

// --- 3. AUTHENTICATION ---
function toggleMode() {
    isLoginMode = !isLoginMode;
    document.querySelector('.auth-header').innerText = isLoginMode ? "Chào mừng trở lại!" : "Tạo tài khoản";
    document.querySelector('.auth-sub').innerText = isLoginMode ? "Rất vui được gặp lại bạn!" : "Tham gia server ngay nào!";
    document.getElementById('btn-submit').innerText = isLoginMode ? "Đăng Nhập" : "Đăng Ký";
}

async function handleAuth() {
    const u = document.getElementById('username').value; const p = document.getElementById('password').value;
    const endpoint = isLoginMode ? '/login' : '/register';
    try {
        const res = await fetch(endpoint, { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({username: u, password: p}) });
        if(!res.ok) { document.getElementById('error-msg').innerText = await res.text(); return; }
        if(isLoginMode) { const d = await res.json(); localStorage.setItem('token', d.token); localStorage.setItem('username', d.username); currentUser=d.username; token=d.token; showChatScreen(); }
        else { alert("Đăng ký xong!"); toggleMode(); }
    } catch(e) { console.error(e); }
}

function logout() { localStorage.clear(); location.reload(); }

// --- 4. UI LOGIC ---
function showChatScreen() {
    document.getElementById('auth-screen').classList.add('hidden');
    document.getElementById('app-screen').classList.remove('hidden');
    document.getElementById('my-username').innerText = currentUser;
    document.getElementById('my-avatar').src = getAvatarUrl(currentUser);
    loadChannels(); loadMembers(); connectWebSocket();
}

async function loadChannels() {
    const res = await fetch('/channels', { headers: { 'Authorization': 'Bearer ' + token } });
    const channels = await res.json();
    const div = document.getElementById('channel-list-container'); div.innerHTML = '';
    channels.forEach((c, i) => {
        const el = document.createElement('div');
        el.className = 'px-2 py-1.5 rounded mx-2 text-discord-muted hover:bg-discord-hover hover:text-discord-text cursor-pointer flex items-center gap-1.5 transition-colors font-medium';
        el.innerHTML = `<i class="fa-solid fa-hashtag text-lg"></i> ${c.name}`;
        el.onclick = () => switchChat('CHANNEL', c.id, c.name, el);
        if(i===0 && chatMode==='CHANNEL') switchChat('CHANNEL', c.id, c.name, el);
        div.appendChild(el);
    });
}
async function loadMembers() {
    const res = await fetch('/users', { headers: { 'Authorization': 'Bearer ' + token } });
    const users = await res.json();
    const div = document.getElementById('members-container'); div.innerHTML = '';
    users.forEach(u => {
        if(u.username===currentUser) return;
        const el = document.createElement('div');
        el.className = 'flex items-center px-2 py-1.5 rounded hover:bg-discord-hover cursor-pointer gap-3 opacity-90 hover:opacity-100 transition-colors';
        el.innerHTML = `<div class="relative"><img class="w-8 h-8 rounded-full bg-discord-server" src="${getAvatarUrl(u.username)}"><div class="absolute bottom-0 right-0 w-3.5 h-3.5 bg-discord-green rounded-full border-[3px] border-discord-sidebar"></div></div><span class="font-medium text-discord-muted group-hover:text-gray-200">${u.username}</span>`;
        el.onclick = () => switchChat('DM', u.username, u.username, el);
        div.appendChild(el);
    });
}

async function switchChat(mode, target, name, el) {
    chatMode=mode; chatTarget=target;
    document.getElementById('current-target-name').innerText = name;
    const icon = document.getElementById('header-icon');
    if (mode === 'CHANNEL') { icon.className = "fa-solid fa-hashtag text-discord-muted text-xl"; }
    else { icon.className = "fa-solid fa-at text-discord-muted text-xl"; }

    document.getElementById('msgInput').placeholder = `Nhắn tin cho ${mode === 'CHANNEL' ? '#' : '@'}${name}`;

    document.querySelectorAll('#channel-list-container > div, #members-container > div').forEach(e => {
        e.classList.remove('bg-discord-active', 'text-white');
        if(!e.classList.contains('text-discord-muted')) e.classList.add('text-discord-muted');
    });
    if(el) {
        el.classList.remove('text-discord-muted', 'hover:bg-discord-hover');
        el.classList.add('bg-discord-active', 'text-white');
    }

    document.getElementById('chat-container').innerHTML = '';
    document.getElementById('typing-indicator').innerHTML = '';

    let url = mode==='CHANNEL' ? `/history?channel_id=${target}` : `/dm_history?with_user=${target}`;
    const res = await fetch(url, { headers: { 'Authorization': 'Bearer ' + token } });
    const msgs = await res.json();
    msgs.forEach(m => appendMessage(m.username||m.sender, m.content));
}

// --- 5. WEBSOCKET ---
function connectWebSocket() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(protocol + "//" + location.host + "/ws?token=" + token);

    ws.onmessage = (event) => {
        const data = JSON.parse(event.data);

        const isChannelMsg = data.type === 'channel' && chatMode === 'CHANNEL' && data.channel_id === chatTarget;
        const isDMMsg = data.type === 'dm' && ((data.receiver === currentUser && chatTarget === data.sender) || (data.sender === currentUser && chatTarget === data.receiver));

        if (isChannelMsg || isDMMsg) {
            appendMessage(data.username || data.sender, data.content);
            showTyping(data.username || data.sender, false);

            if ((data.username || data.sender) === currentUser) playSound('sent');
            else playSound('received');
        }
        else if (data.type === 'typing') {
            if ((chatMode === 'CHANNEL' && data.channel_id === chatTarget && data.username !== currentUser) ||
                (chatMode === 'DM' && data.sender === chatTarget && data.username !== currentUser)) {
                showTyping(data.username, true);
            }
        }
    };
    ws.onclose = () => setTimeout(connectWebSocket, 3000);
}

// --- 6. INPUT ---
let lastTypedTime = 0;
function handleInput() {
    const now = Date.now();
    if (now - lastTypedTime > 2000 && ws) {
        let payload = chatMode === 'CHANNEL' ? { channel_id: chatTarget } : { receiver: chatTarget };
        ws.send(JSON.stringify(payload)); lastTypedTime = now;
    }
}

let typingClearTimer = null;
function showTyping(username, isTyping) {
    const el = document.getElementById('typing-indicator');
    if (!isTyping) { el.innerHTML = ''; return; }
    el.innerHTML = `<div class="flex items-center gap-1"><span class="font-bold text-white">${username}</span> đang soạn tin<div class="flex gap-0.5 ml-1 pt-1"><div class="w-1.5 h-1.5 bg-discord-muted rounded-full typing-dot"></div><div class="w-1.5 h-1.5 bg-discord-muted rounded-full typing-dot"></div><div class="w-1.5 h-1.5 bg-discord-muted rounded-full typing-dot"></div></div></div>`;
    if (typingClearTimer) clearTimeout(typingClearTimer);
    typingClearTimer = setTimeout(() => el.innerHTML = '', 3000);
}

function handleKeyDown(e) {
    if(e.key==='Enter' && !e.shiftKey) {
        e.preventDefault();
        const text = e.target.value;
        if(text && ws) {
            let p = chatMode==='CHANNEL' ? {channel_id:chatTarget, content:text} : {receiver:chatTarget, content:text};
            ws.send(JSON.stringify(p));
            e.target.value = '';
        }
    }
}

function appendMessage(user, text) {
    const div = document.createElement('div');
    div.className = 'flex gap-4 px-2 py-1 hover:bg-[#2e3035] -mx-2 rounded transition-colors group';
    const time = new Date().toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'});
    const rawHtml = marked.parse(text);
    const cleanHtml = DOMPurify.sanitize(rawHtml);

    div.innerHTML = `
        <img class="w-10 h-10 rounded-full mt-0.5 cursor-pointer hover:opacity-80 transition-opacity" src="${getAvatarUrl(user)}">
        <div class="flex-1 min-w-0">
            <div class="flex items-baseline gap-2"><span class="font-medium text-white hover:underline cursor-pointer">${user}</span><span class="text-[11px] text-discord-muted font-normal">${time}</span></div>
            <div class="text-discord-text text-[15px] leading-relaxed markdown">${cleanHtml}</div>
        </div>
    `;
    const container = document.getElementById('chat-container');
    container.appendChild(div);
    container.scrollTop = container.scrollHeight;
}