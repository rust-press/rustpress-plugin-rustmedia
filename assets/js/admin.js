/**
 * RustMedia Admin JavaScript
 */

// Global namespace
window.RustMedia = window.RustMedia || {};

/**
 * API Client
 */
RustMedia.API = {
    baseUrl: '/api/media',

    async request(endpoint, options = {}) {
        const url = `${this.baseUrl}${endpoint}`;
        const defaultOptions = {
            headers: {
                'Content-Type': 'application/json',
            },
        };

        const response = await fetch(url, { ...defaultOptions, ...options });

        if (!response.ok) {
            const error = await response.json().catch(() => ({ message: response.statusText }));
            throw new Error(error.message || 'Request failed');
        }

        return response.json();
    },

    // Media endpoints
    async getMedia(id) {
        return this.request(`/${id}`);
    },

    async listMedia(params = {}) {
        const query = new URLSearchParams(params).toString();
        return this.request(`?${query}`);
    },

    async updateMedia(id, data) {
        return this.request(`/${id}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        });
    },

    async deleteMedia(id, permanent = false) {
        return this.request(`/${id}?permanent=${permanent}`, {
            method: 'DELETE',
        });
    },

    async moveMedia(id, folderId) {
        return this.request(`/${id}/move`, {
            method: 'POST',
            body: JSON.stringify({ folder_id: folderId }),
        });
    },

    async searchMedia(query, limit = 20) {
        return this.request(`/search?q=${encodeURIComponent(query)}&limit=${limit}`);
    },

    // Folder endpoints
    async getFolders() {
        return this.request('/folders');
    },

    async getFolder(id) {
        return this.request(`/folders/${id}`);
    },

    async createFolder(data) {
        return this.request('/folders', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    },

    async updateFolder(id, data) {
        return this.request(`/folders/${id}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        });
    },

    async deleteFolder(id, force = false) {
        return this.request(`/folders/${id}?force=${force}`, {
            method: 'DELETE',
        });
    },

    async getFolderTree() {
        return this.request('/folders/tree');
    },

    // Upload endpoints
    async initChunkedUpload(data) {
        return this.request('/upload/chunked/init', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    },

    async uploadChunk(uploadId, chunkIndex, data) {
        const formData = new FormData();
        formData.append('chunk', data);

        return fetch(`${this.baseUrl}/upload/chunked/${uploadId}/${chunkIndex}`, {
            method: 'POST',
            body: formData,
        }).then(r => r.json());
    },

    async completeChunkedUpload(uploadId) {
        return this.request(`/upload/chunked/${uploadId}/complete`, {
            method: 'POST',
        });
    },

    async cancelChunkedUpload(uploadId) {
        return this.request(`/upload/chunked/${uploadId}`, {
            method: 'DELETE',
        });
    },

    // Stats
    async getStats() {
        return this.request('/stats');
    },
};

/**
 * Modal Manager
 */
RustMedia.Modal = {
    open(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.classList.remove('hidden');
            document.body.style.overflow = 'hidden';
        }
    },

    close(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.classList.add('hidden');
            document.body.style.overflow = '';
        }
    },

    init() {
        // Close on backdrop click
        document.querySelectorAll('.modal').forEach(modal => {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    this.close(modal.id);
                }
            });
        });

        // Close button
        document.querySelectorAll('.modal-close').forEach(btn => {
            btn.addEventListener('click', () => {
                const modal = btn.closest('.modal');
                if (modal) {
                    this.close(modal.id);
                }
            });
        });

        // ESC key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                document.querySelectorAll('.modal:not(.hidden)').forEach(modal => {
                    this.close(modal.id);
                });
            }
        });
    },
};

/**
 * Toast Notifications
 */
RustMedia.Toast = {
    container: null,

    init() {
        this.container = document.createElement('div');
        this.container.className = 'toast-container';
        this.container.style.cssText = `
            position: fixed;
            bottom: 20px;
            right: 20px;
            z-index: 9999;
            display: flex;
            flex-direction: column;
            gap: 10px;
        `;
        document.body.appendChild(this.container);
    },

    show(message, type = 'info', duration = 3000) {
        if (!this.container) this.init();

        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        toast.style.cssText = `
            padding: 12px 20px;
            border-radius: 6px;
            color: white;
            font-size: 14px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
            animation: slideIn 0.3s ease;
            cursor: pointer;
            background: ${this.getColor(type)};
        `;
        toast.textContent = message;
        toast.onclick = () => this.hide(toast);

        this.container.appendChild(toast);

        if (duration > 0) {
            setTimeout(() => this.hide(toast), duration);
        }

        return toast;
    },

    hide(toast) {
        toast.style.animation = 'slideOut 0.3s ease';
        setTimeout(() => toast.remove(), 300);
    },

    getColor(type) {
        const colors = {
            success: '#22c55e',
            error: '#ef4444',
            warning: '#f59e0b',
            info: '#2563eb',
        };
        return colors[type] || colors.info;
    },

    success(message) { return this.show(message, 'success'); },
    error(message) { return this.show(message, 'error'); },
    warning(message) { return this.show(message, 'warning'); },
    info(message) { return this.show(message, 'info'); },
};

/**
 * Media Selection
 */
RustMedia.Selection = {
    selected: new Set(),

    toggle(id) {
        if (this.selected.has(id)) {
            this.selected.delete(id);
        } else {
            this.selected.add(id);
        }
        this.updateUI();
    },

    selectAll() {
        document.querySelectorAll('.media-item[data-id]').forEach(item => {
            this.selected.add(item.dataset.id);
        });
        this.updateUI();
    },

    deselectAll() {
        this.selected.clear();
        this.updateUI();
    },

    updateUI() {
        document.querySelectorAll('.media-item[data-id]').forEach(item => {
            const checkbox = item.querySelector('.item-select');
            if (checkbox) {
                checkbox.checked = this.selected.has(item.dataset.id);
            }
            item.classList.toggle('selected', this.selected.has(item.dataset.id));
        });

        // Update toolbar
        const toolbar = document.querySelector('.selection-toolbar');
        if (toolbar) {
            toolbar.style.display = this.selected.size > 0 ? 'flex' : 'none';
            const count = toolbar.querySelector('.selection-count');
            if (count) {
                count.textContent = `${this.selected.size} selected`;
            }
        }
    },

    getSelected() {
        return Array.from(this.selected);
    },
};

/**
 * Media Actions
 */
RustMedia.Actions = {
    async delete(ids, permanent = false) {
        if (!confirm(`Delete ${ids.length} item(s)?`)) return;

        try {
            for (const id of ids) {
                await RustMedia.API.deleteMedia(id, permanent);
            }
            RustMedia.Toast.success(`Deleted ${ids.length} item(s)`);
            location.reload();
        } catch (error) {
            RustMedia.Toast.error(error.message);
        }
    },

    async move(ids, folderId) {
        try {
            for (const id of ids) {
                await RustMedia.API.moveMedia(id, folderId);
            }
            RustMedia.Toast.success(`Moved ${ids.length} item(s)`);
            location.reload();
        } catch (error) {
            RustMedia.Toast.error(error.message);
        }
    },

    async bulkDelete() {
        const ids = RustMedia.Selection.getSelected();
        if (ids.length === 0) {
            RustMedia.Toast.warning('No items selected');
            return;
        }
        await this.delete(ids);
    },

    openDetail(id) {
        window.location.href = `/admin/media/${id}`;
    },

    edit(id) {
        window.location.href = `/admin/media/${id}/edit`;
    },
};

/**
 * View Toggle
 */
RustMedia.View = {
    current: 'grid',

    set(view) {
        this.current = view;
        localStorage.setItem('rustmedia_view', view);

        const grid = document.querySelector('.media-grid');
        if (grid) {
            grid.classList.remove('view-grid', 'view-list');
            grid.classList.add(`view-${view}`);
        }

        document.querySelectorAll('.view-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.view === view);
        });
    },

    init() {
        const saved = localStorage.getItem('rustmedia_view');
        if (saved) {
            this.set(saved);
        }

        document.querySelectorAll('.view-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                this.set(btn.dataset.view);
            });
        });
    },
};

/**
 * Search
 */
RustMedia.Search = {
    debounceTimer: null,

    init() {
        const input = document.querySelector('input[name="search"]');
        if (!input) return;

        input.addEventListener('input', (e) => {
            clearTimeout(this.debounceTimer);
            this.debounceTimer = setTimeout(() => {
                this.submit(e.target.form);
            }, 500);
        });
    },

    submit(form) {
        if (form) form.submit();
    },
};

/**
 * Format utilities
 */
RustMedia.Format = {
    fileSize(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    },

    date(dateString) {
        return new Date(dateString).toLocaleDateString();
    },

    time(dateString) {
        return new Date(dateString).toLocaleTimeString();
    },

    datetime(dateString) {
        const d = new Date(dateString);
        return `${d.toLocaleDateString()} ${d.toLocaleTimeString()}`;
    },
};

/**
 * Initialize
 */
document.addEventListener('DOMContentLoaded', () => {
    RustMedia.Modal.init();
    RustMedia.View.init();
    RustMedia.Search.init();

    // Media item clicks
    document.querySelectorAll('.media-item').forEach(item => {
        const checkbox = item.querySelector('.item-select');
        if (checkbox) {
            checkbox.addEventListener('click', (e) => {
                e.stopPropagation();
                RustMedia.Selection.toggle(item.dataset.id);
            });
        }

        item.addEventListener('click', (e) => {
            if (e.target.classList.contains('item-select')) return;
            RustMedia.Actions.openDetail(item.dataset.id);
        });

        // Action buttons
        item.querySelector('.btn-view')?.addEventListener('click', (e) => {
            e.stopPropagation();
            RustMedia.Actions.openDetail(item.dataset.id);
        });

        item.querySelector('.btn-edit')?.addEventListener('click', (e) => {
            e.stopPropagation();
            RustMedia.Actions.edit(item.dataset.id);
        });

        item.querySelector('.btn-delete')?.addEventListener('click', (e) => {
            e.stopPropagation();
            RustMedia.Actions.delete([item.dataset.id]);
        });
    });

    // Add CSS animations
    const style = document.createElement('style');
    style.textContent = `
        @keyframes slideIn {
            from { transform: translateX(100%); opacity: 0; }
            to { transform: translateX(0); opacity: 1; }
        }
        @keyframes slideOut {
            from { transform: translateX(0); opacity: 1; }
            to { transform: translateX(100%); opacity: 0; }
        }
    `;
    document.head.appendChild(style);
});

// Global functions for inline handlers
window.closeModal = function() {
    document.querySelectorAll('.modal:not(.hidden)').forEach(modal => {
        RustMedia.Modal.close(modal.id);
    });
};
