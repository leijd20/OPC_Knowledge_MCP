// PangenMCP Admin UI - Alpine.js Version

// API Configuration
const API_BASE = '/api';

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
        error: null,

        async init() {
            await this.loadDashboard();
        },

        async loadDashboard() {
            this.loading = true;
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
        editing: false,
        form: {
            query_mode: '',
            top_k: 10,
            response_type: ''
        },

        async init() {
            await this.loadConfig();
        },

        async loadConfig() {
            this.loading = true;
            try {
                const data = await apiCall('/config');
                if (data) {
                    this.config = data;
                }
            } catch (err) {
                console.error(err);
            } finally {
                this.loading = false;
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
            } catch (err) {
                alert('Failed to save: ' + err.message);
            }
        }
    }));
});

// Tokens Component
document.addEventListener('alpine:init', () => {
    Alpine.data('tokens', () => ({
        tokens: [],
        loading: true,
        creating: false,
        newToken: null,
        form: {
            name: '',
            scopes: ''
        },

        async init() {
            await this.loadTokens();
        },

        async loadTokens() {
            this.loading = true;
            try {
                const data = await apiCall('/tokens');
                if (data) this.tokens = data.tokens;
            } catch (err) {
                console.error(err);
            } finally {
                this.loading = false;
            }
        },

        openCreateModal() {
            this.form.name = '';
            this.form.scopes = '';
            this.newToken = null;
            this.creating = true;
        },

        closeCreateModal() {
            this.creating = false;
        },

        async createToken() {
            try {
                const scopes = this.form.scopes.split(',').map(s => s.trim());
                const data = await apiCall('/tokens', {
                    method: 'POST',
                    body: JSON.stringify({
                        name: this.form.name,
                        scopes
                    })
                });
                if (data) {
                    this.newToken = data.token;
                    await this.loadTokens();
                }
            } catch (err) {
                alert('Failed to create: ' + err.message);
            }
        },

        async deleteToken(name) {
            if (!confirm(`Delete token "${name}"?`)) return;

            try {
                await apiCall(`/tokens/${name}`, { method: 'DELETE' });
                await this.loadTokens();
            } catch (err) {
                alert('Failed to delete: ' + err.message);
            }
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
        filters: {
            user: '',
            tool: ''
        },

        async init() {
            await this.loadLogs();
        },

        async loadLogs() {
            this.loading = true;
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
            }
        },

        applyFilters() {
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
