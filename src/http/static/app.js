// OPC_Knowledge_MCP Admin UI - Alpine.js Version

// API Configuration
const API_BASE = '/api';

// Valid scopes for token creation
const VALID_SCOPES = [
    'rag:read', 'rag:write', 'rag:admin',
    'token:read', 'token:write',
    'config:read', 'config:write',
    'stats:read', 'audit:read'
];

// Check authentication on page load
document.addEventListener('alpine:init', () => {
    const token = localStorage.getItem('admin_token');
    if (!token) {
        window.location.href = '/login.html';
        return;
    }

    // Global auth store
    Alpine.store('auth', {
        token: token,
        logout() {
            localStorage.removeItem('admin_token');
            window.location.href = '/login.html';
        }
    });

    // Global UI store
    Alpine.store('ui', {
        currentView: 'dashboard',
        switchView(view) {
            this.currentView = view;
        }
    });

    // Global Toast store
    Alpine.store('toast', {
        visible: false,
        message: '',
        type: 'info', // 'success', 'error', 'info', 'warning'
        timeoutId: null,

        show(message, type = 'info') {
            // Clear existing timeout
            if (this.timeoutId) {
                clearTimeout(this.timeoutId);
            }

            this.message = message;
            this.type = type;
            this.visible = true;

            // Auto-hide after 3 seconds
            this.timeoutId = setTimeout(() => {
                this.hide();
            }, 3000);
        },

        hide() {
            this.visible = false;
            if (this.timeoutId) {
                clearTimeout(this.timeoutId);
                this.timeoutId = null;
            }
        },

        success(message) {
            this.show(message, 'success');
        },

        error(message) {
            this.show(message, 'error');
        },

        info(message) {
            this.show(message, 'info');
        },

        warning(message) {
            this.show(message, 'warning');
        }
    });

    // API helper function
    window.apiCall = async function(endpoint, options = {}) {
        const headers = {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${Alpine.store('auth').token}`,
            ...options.headers
        };

        try {
            const response = await fetch(`${API_BASE}${endpoint}`, {
                ...options,
                headers
            });

            if (response.status === 401) {
                Alpine.store('auth').logout();
                return null;
            }

            if (response.status === 403) {
                throw new Error('Permission denied. Check your token scopes.');
            }

            if (!response.ok) {
                const text = await response.text();
                throw new Error(text || `HTTP ${response.status}`);
            }

            return await response.json();
        } catch (error) {
            throw error;
        }
    };
});

// Dashboard Component
document.addEventListener('alpine:init', () => {
    Alpine.data('dashboard', () => ({
        health: null,
        stats: null,
        loading: true,
        refreshing: false,
        error: null,

        init() {
            this.loadDashboard();
            // Watch for view changes and reload data
            this.$watch('$root.currentView', (value) => {
                if (value === 'dashboard') {
                    this.loadDashboard();
                }
            });
        },

        async loadDashboard(isRefresh = false) {
            if (isRefresh) {
                this.refreshing = true;
            } else {
                this.loading = true;
            }
            this.error = null;

            try {
                // Load health
                const healthData = await apiCall('/health');
                if (healthData) this.health = healthData;

                // Load stats
                const statsData = await apiCall('/stats');
                if (statsData) this.stats = statsData;
            } catch (err) {
                this.error = err.message;
            } finally {
                this.loading = false;
                this.refreshing = false;
            }
        },

        formatUptime(seconds) {
            return Math.floor(seconds / 60) + ' minutes';
        }
    }));
});

// Configuration Component
document.addEventListener('alpine:init', () => {
    Alpine.data('configuration', () => ({
        config: null,
        loading: true,
        refreshing: false,
        editing: false,
        form: {
            query_mode: '',
            top_k: 10,
            response_type: ''
        },

        init() {
            this.loadConfig();
            // Watch for view changes and reload data
            this.$watch('$root.currentView', (value) => {
                if (value === 'config') {
                    this.loadConfig();
                }
            });
        },

        async loadConfig(isRefresh = false) {
            if (isRefresh) {
                this.refreshing = true;
            } else {
                this.loading = true;
            }
            try {
                const data = await apiCall('/config');
                if (data) {
                    this.config = data;
                }
            } catch (err) {
                console.error(err);
            } finally {
                this.loading = false;
                this.refreshing = false;
            }
        },

        openEditModal() {
            this.form.query_mode = this.config.defaults.query_mode;
            this.form.top_k = this.config.defaults.top_k;
            this.form.response_type = this.config.defaults.response_type;
            this.editing = true;
        },

        closeEditModal() {
            this.editing = false;
        },

        async saveConfig() {
            try {
                const patch = { defaults: this.form };
                await apiCall('/config', {
                    method: 'PATCH',
                    body: JSON.stringify(patch)
                });
                await this.loadConfig();
                this.closeEditModal();
                Alpine.store('toast').success('Configuration saved successfully');
            } catch (err) {
                Alpine.store('toast').error('Failed to save: ' + err.message);
            }
        }
    }));
});

// Tokens Component
document.addEventListener('alpine:init', () => {
    Alpine.data('tokens', () => ({
        tokens: [],
        loading: true,
        refreshing: false,
        creating: false,
        newToken: null,
        revealedTokens: {}, // 跟踪哪些 token 已显示（改为对象）
        validScopes: VALID_SCOPES,
        form: {
            name: '',
            scopes: []  // 改为数组
        },

        init() {
            this.loadTokens();
            // Watch for view changes and reload data
            this.$watch('$root.currentView', (value) => {
                if (value === 'tokens') {
                    this.loadTokens();
                }
            });
        },

        async loadTokens(isRefresh = false) {
            if (isRefresh) {
                this.refreshing = true;
            } else {
                this.loading = true;
            }
            try {
                const data = await apiCall('/tokens');
                if (data) this.tokens = data.tokens;
            } catch (err) {
                console.error(err);
                Alpine.store('toast').error('Failed to load tokens');
            } finally {
                this.loading = false;
                this.refreshing = false;
            }
        },

        openCreateModal() {
            this.form.name = '';
            this.form.scopes = [];  // 重置为空数组
            this.newToken = null;
            this.creating = true;
        },

        closeCreateModal() {
            this.creating = false;
            this.newToken = null;
        },

        toggleScope(scope) {
            const index = this.form.scopes.indexOf(scope);
            if (index > -1) {
                this.form.scopes.splice(index, 1);
            } else {
                this.form.scopes.push(scope);
            }
        },

        async createToken() {
            try {
                // scopes 已经是数组，不需要 split
                const data = await apiCall('/tokens', {
                    method: 'POST',
                    body: JSON.stringify({
                        name: this.form.name,
                        scopes: this.form.scopes
                    })
                });
                if (data) {
                    this.newToken = data.token;
                    await this.loadTokens();
                }
            } catch (err) {
                Alpine.store('toast').error('Failed to create token: ' + err.message);
            }
        },

        async deleteToken(name) {
            if (!confirm(`Delete token "${name}"?`)) return;

            try {
                await apiCall(`/tokens/${name}`, { method: 'DELETE' });
                await this.loadTokens();
                Alpine.store('toast').success('Token deleted successfully');
            } catch (err) {
                Alpine.store('toast').error('Failed to delete token: ' + err.message);
            }
        },

        async revealToken(name) {
            const token = this.tokens.find(t => t.name === name);
            if (!token) return;

            if (token.revealed) {
                // 隐藏：重新加载以获取遮蔽版本
                token.revealed = false;
                delete this.revealedTokens[name];
                await this.loadTokens();
            } else {
                // 显示：获取完整 token
                try {
                    const data = await apiCall(`/tokens/${name}/reveal`);
                    token.token_preview = data.token;
                    token.revealed = true;
                    this.revealedTokens[name] = true;
                    Alpine.store('toast').success('Full token revealed');
                } catch (err) {
                    Alpine.store('toast').error('Failed to reveal token: ' + err.message);
                }
            }
        },

        copyToken(text) {
            navigator.clipboard.writeText(text).then(() => {
                Alpine.store('toast').success('Token preview copied to clipboard');
            }).catch(() => {
                Alpine.store('toast').error('Failed to copy to clipboard');
            });
        },

        copyNewToken() {
            navigator.clipboard.writeText(this.newToken).then(() => {
                Alpine.store('toast').success('Token copied to clipboard');
            }).catch(() => {
                Alpine.store('toast').error('Failed to copy to clipboard');
            });
        }
    }));
});

// Audit Logs Component
document.addEventListener('alpine:init', () => {
    Alpine.data('auditLogs', () => ({
        logs: [],
        total: 0,
        page: 1,
        pageSize: 20,
        loading: true,
        refreshing: false,
        filters: {
            user: '',
            tool: ''
        },

        init() {
            this.loadLogs();
            // Watch for view changes and reload data
            this.$watch('$root.currentView', (value) => {
                if (value === 'audit') {
                    this.loadLogs();
                }
            });
        },

        async loadLogs(isRefresh = false) {
            if (isRefresh) {
                this.refreshing = true;
            } else {
                this.loading = true;
            }
            try {
                const params = new URLSearchParams({
                    page: this.page,
                    page_size: this.pageSize
                });

                if (this.filters.user) params.append('user', this.filters.user);
                if (this.filters.tool) params.append('tool', this.filters.tool);

                const data = await apiCall(`/audit/logs?${params}`);
                if (data) {
                    this.logs = data.logs;
                    this.total = data.total;
                }
            } catch (err) {
                console.error(err);
            } finally {
                this.loading = false;
                this.refreshing = false;
            }
        },

        applyFilters() {
            this.page = 1;
            this.loadLogs();
        },

        clearFilters() {
            this.filters.user = '';
            this.filters.tool = '';
            this.page = 1;
            this.loadLogs();
        },

        prevPage() {
            if (this.page > 1) {
                this.page--;
                this.loadLogs();
            }
        },

        nextPage() {
            if (this.logs.length === this.pageSize) {
                this.page++;
                this.loadLogs();
            }
        },

        formatTime(timestamp) {
            return new Date(timestamp).toLocaleString();
        },

        get canGoPrev() {
            return this.page > 1;
        },

        get canGoNext() {
            return this.logs.length === this.pageSize;
        }
    }));
});
