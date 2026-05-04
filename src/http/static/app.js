// PangenMCP Admin UI - JavaScript

// Configuration
const API_BASE = '/api';
let authToken = localStorage.getItem('admin_token') || '';
let currentPage = 1;
let currentFilters = {};

// API Helper
async function apiCall(endpoint, options = {}) {
    const headers = {
        'Content-Type': 'application/json',
        ...options.headers
    };

    if (authToken) {
        headers['Authorization'] = `Bearer ${authToken}`;
    }

    try {
        const response = await fetch(`${API_BASE}${endpoint}`, {
            ...options,
            headers
        });

        if (response.status === 401) {
            showError('Authentication required. Please set your token.');
            return null;
        }

        if (response.status === 403) {
            showError('Permission denied. Check your token scopes.');
            return null;
        }

        if (!response.ok) {
            const text = await response.text();
            throw new Error(text || `HTTP ${response.status}`);
        }

        return await response.json();
    } catch (error) {
        showError(`API Error: ${error.message}`);
        return null;
    }
}

// UI Helpers
function showError(message) {
    const existing = document.querySelector('.error-message');
    if (existing) existing.remove();

    const div = document.createElement('div');
    div.className = 'error-message';
    div.textContent = message;
    document.querySelector('#content').prepend(div);
    setTimeout(() => div.remove(), 5000);
}

function showSuccess(message) {
    const existing = document.querySelector('.success-message');
    if (existing) existing.remove();

    const div = document.createElement('div');
    div.className = 'success-message';
    div.textContent = message;
    document.querySelector('#content').prepend(div);
    setTimeout(() => div.remove(), 3000);
}

// Navigation
function switchView(viewName) {
    document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
    document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));

    document.getElementById(`${viewName}-view`).classList.add('active');
    document.querySelector(`[data-view="${viewName}"]`).classList.add('active');

    if (viewName === 'dashboard') loadDashboard();
    else if (viewName === 'config') loadConfig();
    else if (viewName === 'tokens') loadTokens();
    else if (viewName === 'audit') loadAuditLogs();
}

// Dashboard
async function loadDashboard() {
    const healthData = await apiCall('/health');
    if (healthData) {
        const html = `
            <div><strong>Server:</strong> <span class="status-${healthData.server.status === 'healthy' ? 'healthy' : 'error'}">${healthData.server.status}</span></div>
            <div><strong>Version:</strong> ${healthData.server.version}</div>
            <div><strong>LightRAG:</strong> <span class="status-${healthData.lightrag.status === 'healthy' ? 'healthy' : 'error'}">${healthData.lightrag.status}</span></div>
            <div><strong>LightRAG URL:</strong> ${healthData.lightrag.url}</div>
        `;
        document.getElementById('health-status').innerHTML = html;
    }

    const statsData = await apiCall('/stats');
    if (statsData) {
        let html = `<div><strong>Total Requests:</strong> ${statsData.total_requests}</div>`;
        html += `<div><strong>Total Errors:</strong> ${statsData.total_errors}</div>`;
        html += `<div><strong>Uptime:</strong> ${Math.floor(statsData.uptime_seconds / 60)} minutes</div>`;

        if (Object.keys(statsData.by_tool).length > 0) {
            html += '<h3>By Tool:</h3><table><tr><th>Tool</th><th>Requests</th><th>Errors</th><th>Avg (ms)</th></tr>';
            for (const [tool, data] of Object.entries(statsData.by_tool)) {
                html += `<tr><td>${tool}</td><td>${data.requests}</td><td>${data.errors}</td><td>${data.avg_duration_ms.toFixed(1)}</td></tr>`;
            }
            html += '</table>';
        }
        document.getElementById('stats-display').innerHTML = html;
    }
}

// Configuration
async function loadConfig() {
    const data = await apiCall('/config');
    if (data) {
        const html = `
            <h3>Server</h3>
            <div><strong>Host:</strong> ${data.server.host}</div>
            <div><strong>Port:</strong> ${data.server.port}</div>
            <h3>LightRAG</h3>
            <div><strong>URL:</strong> ${data.lightrag.url}</div>
            <div><strong>Timeout:</strong> ${data.lightrag.timeout_seconds}s</div>
            <h3>Defaults</h3>
            <div><strong>Query Mode:</strong> ${data.defaults.query_mode}</div>
            <div><strong>Top K:</strong> ${data.defaults.top_k}</div>
            <div><strong>Response Type:</strong> ${data.defaults.response_type}</div>
        `;
        document.getElementById('config-display').innerHTML = html;

        // Store for editing
        window.currentConfig = data;
    }
}

document.getElementById('edit-config-btn').addEventListener('click', () => {
    if (!window.currentConfig) return;

    document.getElementById('config-query-mode').value = window.currentConfig.defaults.query_mode;
    document.getElementById('config-top-k').value = window.currentConfig.defaults.top_k;
    document.getElementById('config-response-type').value = window.currentConfig.defaults.response_type;

    document.getElementById('config-modal').classList.add('active');
});

document.getElementById('edit-config-form').addEventListener('submit', async (e) => {
    e.preventDefault();

    const patch = {
        defaults: {
            query_mode: document.getElementById('config-query-mode').value,
            top_k: parseInt(document.getElementById('config-top-k').value),
            response_type: document.getElementById('config-response-type').value
        }
    };

    const result = await apiCall('/config', {
        method: 'PATCH',
        body: JSON.stringify(patch)
    });

    if (result) {
        showSuccess('Configuration updated successfully');
        document.getElementById('config-modal').classList.remove('active');
        loadConfig();
    }
});

// Tokens
async function loadTokens() {
    const data = await apiCall('/tokens');
    if (data) {
        let html = '<table><tr><th>Name</th><th>Token Preview</th><th>Scopes</th><th>Actions</th></tr>';
        for (const token of data.tokens) {
            html += `<tr>
                <td>${token.name}</td>
                <td><code>${token.token_preview}</code></td>
                <td>${token.scopes.join(', ')}</td>
                <td><button class="btn btn-danger" onclick="deleteToken('${token.name}')">Delete</button></td>
            </tr>`;
        }
        html += '</table>';
        document.getElementById('tokens-list').innerHTML = html;
    }
}

document.getElementById('create-token-btn').addEventListener('click', () => {
    document.getElementById('create-token-form').reset();
    document.getElementById('new-token-display').style.display = 'none';
    document.getElementById('token-modal').classList.add('active');
});

document.getElementById('create-token-form').addEventListener('submit', async (e) => {
    e.preventDefault();

    const name = document.getElementById('token-name').value;
    const scopes = document.getElementById('token-scopes').value.split(',').map(s => s.trim());

    const result = await apiCall('/tokens', {
        method: 'POST',
        body: JSON.stringify({ name, scopes })
    });

    if (result) {
        document.getElementById('new-token-value').textContent = result.token;
        document.getElementById('new-token-display').style.display = 'block';
        document.getElementById('create-token-form').style.display = 'none';
        loadTokens();
    }
});

async function deleteToken(name) {
    if (!confirm(`Delete token "${name}"?`)) return;

    const result = await apiCall(`/tokens/${name}`, { method: 'DELETE' });
    if (result) {
        showSuccess('Token deleted');
        loadTokens();
    }
}

// Audit Logs
async function loadAuditLogs() {
    const params = new URLSearchParams({
        page: currentPage,
        page_size: 20,
        ...currentFilters
    });

    const data = await apiCall(`/audit/logs?${params}`);
    if (data) {
        let html = '<table><tr><th>Time</th><th>User</th><th>Tool</th><th>Params</th><th>Result</th></tr>';
        for (const log of data.logs) {
            html += `<tr>
                <td>${new Date(log.timestamp).toLocaleString()}</td>
                <td>${log.user}</td>
                <td>${log.tool}</td>
                <td>${log.params}</td>
                <td>${log.result}</td>
            </tr>`;
        }
        html += '</table>';
        document.getElementById('audit-logs').innerHTML = html;

        document.getElementById('page-info').textContent = `Page ${data.page} (${data.total} total)`;
        document.getElementById('prev-page-btn').disabled = data.page === 1;
        document.getElementById('next-page-btn').disabled = data.logs.length < data.page_size;
    }
}

document.getElementById('apply-filters-btn').addEventListener('click', () => {
    currentFilters = {};
    const user = document.getElementById('filter-user').value.trim();
    const tool = document.getElementById('filter-tool').value.trim();
    if (user) currentFilters.user = user;
    if (tool) currentFilters.tool = tool;
    currentPage = 1;
    loadAuditLogs();
});

document.getElementById('prev-page-btn').addEventListener('click', () => {
    if (currentPage > 1) {
        currentPage--;
        loadAuditLogs();
    }
});

document.getElementById('next-page-btn').addEventListener('click', () => {
    currentPage++;
    loadAuditLogs();
});

// Modal close handlers
document.querySelectorAll('.close').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.modal').forEach(m => m.classList.remove('active'));
    });
});

// Navigation handlers
document.querySelectorAll('.nav-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        switchView(btn.dataset.view);
    });
});

// Auth status
function updateAuthStatus() {
    const status = document.getElementById('auth-status');
    if (authToken) {
        status.textContent = 'Token: ' + authToken.substring(0, 8) + '...';
    } else {
        status.innerHTML = '<input type="text" placeholder="Enter admin token" id="token-input" style="padding:5px;">';
        document.getElementById('token-input').addEventListener('change', (e) => {
            authToken = e.target.value;
            localStorage.setItem('admin_token', authToken);
            updateAuthStatus();
            loadDashboard();
        });
    }
}

// Initialize
updateAuthStatus();
loadDashboard();



