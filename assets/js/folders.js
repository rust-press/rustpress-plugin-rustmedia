/**
 * RustMedia Folders Handler
 */

RustMedia.Folders = {
    selectedId: null,

    init() {
        this.bindEvents();
    },

    bindEvents() {
        // Create folder button
        const createBtn = document.getElementById('create-folder-btn');
        if (createBtn) {
            createBtn.addEventListener('click', () => {
                RustMedia.Modal.open('create-folder-modal');
            });
        }

        // Create folder form
        const createForm = document.getElementById('create-folder-form');
        if (createForm) {
            createForm.addEventListener('submit', async (e) => {
                e.preventDefault();
                await this.createFolder(new FormData(createForm));
            });
        }
    },

    async select(folderId) {
        this.selectedId = folderId;

        // Update tree selection
        document.querySelectorAll('.tree-item').forEach(item => {
            item.classList.toggle('selected', item.dataset.id === folderId);
        });

        // Load folder details
        const details = document.getElementById('folder-details');
        if (details) {
            details.innerHTML = '<p class="loading">Loading...</p>';

            try {
                const response = await fetch(`/admin/media/folders/${folderId}/details`);
                const html = await response.text();
                details.innerHTML = html;
            } catch (error) {
                details.innerHTML = '<p class="error">Failed to load folder details</p>';
            }
        }
    },

    async createFolder(formData) {
        const data = {
            name: formData.get('name'),
            parent_id: formData.get('parent_id') || null,
            description: formData.get('description') || null,
        };

        try {
            await RustMedia.API.createFolder(data);
            RustMedia.Toast.success('Folder created');
            RustMedia.Modal.close('create-folder-modal');
            location.reload();
        } catch (error) {
            RustMedia.Toast.error(error.message);
        }
    },

    async editFolder(folderId) {
        const folder = await RustMedia.API.getFolder(folderId);

        // Populate edit form
        const modal = document.getElementById('edit-folder-modal');
        if (modal) {
            modal.querySelector('[name="name"]').value = folder.name;
            modal.querySelector('[name="description"]').value = folder.description || '';
            modal.dataset.folderId = folderId;
            RustMedia.Modal.open('edit-folder-modal');
        }
    },

    async updateFolder(folderId) {
        const form = document.getElementById('edit-folder-form');
        if (!form) return;

        const data = {
            name: form.querySelector('[name="name"]').value,
            description: form.querySelector('[name="description"]').value || null,
        };

        try {
            await RustMedia.API.updateFolder(folderId, data);
            RustMedia.Toast.success('Folder updated');
            RustMedia.Modal.close('edit-folder-modal');
            location.reload();
        } catch (error) {
            RustMedia.Toast.error(error.message);
        }
    },

    async deleteFolder(folderId) {
        if (!confirm('Are you sure you want to delete this folder?')) return;

        try {
            await RustMedia.API.deleteFolder(folderId);
            RustMedia.Toast.success('Folder deleted');
            location.reload();
        } catch (error) {
            if (error.message.includes('not empty')) {
                if (confirm('This folder is not empty. Delete anyway (will also delete contents)?')) {
                    try {
                        await RustMedia.API.deleteFolder(folderId, true);
                        RustMedia.Toast.success('Folder and contents deleted');
                        location.reload();
                    } catch (err) {
                        RustMedia.Toast.error(err.message);
                    }
                }
            } else {
                RustMedia.Toast.error(error.message);
            }
        }
    },

    async moveFolder(folderId, newParentId) {
        try {
            await RustMedia.API.request(`/folders/${folderId}/move`, {
                method: 'POST',
                body: JSON.stringify({ new_parent_id: newParentId }),
            });
            RustMedia.Toast.success('Folder moved');
            location.reload();
        } catch (error) {
            RustMedia.Toast.error(error.message);
        }
    },

    toggleExpand(itemElement) {
        const children = itemElement.querySelector('.tree-children');
        if (children) {
            children.style.display = children.style.display === 'none' ? 'block' : 'none';
            const icon = itemElement.querySelector('.tree-icon');
            if (icon) {
                icon.textContent = children.style.display === 'none' ? '📁' : '📂';
            }
        }
    },
};

// Global functions for inline handlers
window.selectFolder = function(id) {
    RustMedia.Folders.select(id);
};

window.editFolder = function(id) {
    RustMedia.Folders.editFolder(id);
};

window.deleteFolder = function(id) {
    RustMedia.Folders.deleteFolder(id);
};

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => {
    RustMedia.Folders.init();
});
