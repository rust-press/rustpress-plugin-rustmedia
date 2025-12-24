/**
 * RustMedia Settings Handler
 */

RustMedia.Settings = {
    init() {
        this.form = document.getElementById('settings-form');
        if (!this.form) return;

        this.bindEvents();
    },

    bindEvents() {
        // Form submission
        this.form.addEventListener('submit', async (e) => {
            e.preventDefault();
            await this.save();
        });

        // Storage backend change
        const storageBackend = document.getElementById('storage-backend');
        if (storageBackend) {
            storageBackend.addEventListener('change', () => {
                this.toggleS3Settings(storageBackend.value === 's3');
            });
            // Initial state
            this.toggleS3Settings(storageBackend.value === 's3');
        }

        // Quality sliders
        this.initRangeInputs();
    },

    toggleS3Settings(show) {
        const s3Section = document.getElementById('s3-settings');
        if (s3Section) {
            s3Section.style.display = show ? 'block' : 'none';
        }
    },

    initRangeInputs() {
        document.querySelectorAll('input[type="range"]').forEach(input => {
            const output = input.nextElementSibling;
            if (output && output.tagName === 'OUTPUT') {
                output.value = input.value;
                input.addEventListener('input', () => {
                    output.value = input.value;
                });
            }
        });
    },

    async save() {
        const formData = new FormData(this.form);
        const data = {};

        // Convert form data to object
        for (const [key, value] of formData.entries()) {
            if (key.endsWith('[]')) {
                const arrayKey = key.slice(0, -2);
                if (!data[arrayKey]) data[arrayKey] = [];
                data[arrayKey].push(value);
            } else {
                data[key] = value;
            }
        }

        // Handle checkboxes (unchecked ones aren't in FormData)
        this.form.querySelectorAll('input[type="checkbox"]').forEach(checkbox => {
            if (!checkbox.name.endsWith('[]')) {
                data[checkbox.name] = checkbox.checked;
            }
        });

        // Convert max_file_size from MB to bytes
        if (data.max_file_size) {
            data.max_file_size = parseInt(data.max_file_size) * 1024 * 1024;
        }

        try {
            const response = await fetch('/api/media/settings', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(data),
            });

            if (!response.ok) {
                throw new Error('Failed to save settings');
            }

            RustMedia.Toast.success('Settings saved successfully');
        } catch (error) {
            RustMedia.Toast.error(error.message);
        }
    },

    // Thumbnail size management
    addThumbnailSize() {
        const tbody = document.querySelector('.sizes-table tbody');
        if (!tbody) return;

        const row = document.createElement('tr');
        row.innerHTML = `
            <td><input type="text" value="new-size" name="size_name[]"></td>
            <td><input type="number" value="300" name="size_width[]" min="1"></td>
            <td><input type="number" value="300" name="size_height[]" min="1"></td>
            <td>
                <select name="size_mode[]">
                    <option value="Fit">Fit</option>
                    <option value="Fill">Fill</option>
                    <option value="Exact">Exact</option>
                </select>
            </td>
            <td><input type="number" value="85" name="size_quality[]" min="1" max="100"></td>
            <td>
                <input type="checkbox" name="size_enabled[]" checked>
                <button type="button" class="btn btn-small btn-danger" onclick="RustMedia.Settings.removeThumbnailSize(this)">Remove</button>
            </td>
        `;
        tbody.appendChild(row);
    },

    removeThumbnailSize(btn) {
        const row = btn.closest('tr');
        if (row) {
            row.remove();
        }
    },

    // Test storage connection
    async testStorage() {
        const backend = document.getElementById('storage-backend')?.value;

        try {
            const response = await fetch('/api/media/settings/test-storage', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ backend }),
            });

            const result = await response.json();

            if (result.success) {
                RustMedia.Toast.success('Storage connection successful');
            } else {
                RustMedia.Toast.error(`Storage test failed: ${result.message}`);
            }
        } catch (error) {
            RustMedia.Toast.error('Failed to test storage connection');
        }
    },

    // Clear cache
    async clearCache() {
        if (!confirm('Clear all cached thumbnails and processed images?')) return;

        try {
            const response = await fetch('/api/media/cache/clear', {
                method: 'POST',
            });

            if (response.ok) {
                RustMedia.Toast.success('Cache cleared successfully');
            } else {
                throw new Error('Failed to clear cache');
            }
        } catch (error) {
            RustMedia.Toast.error(error.message);
        }
    },

    // Regenerate thumbnails
    async regenerateThumbnails() {
        if (!confirm('Regenerate all thumbnails? This may take a while.')) return;

        const btn = document.querySelector('[onclick*="regenerateThumbnails"]');
        if (btn) {
            btn.disabled = true;
            btn.textContent = 'Regenerating...';
        }

        try {
            const response = await fetch('/api/media/thumbnails/regenerate', {
                method: 'POST',
            });

            const result = await response.json();

            if (response.ok) {
                RustMedia.Toast.success(`Regenerated ${result.processed} thumbnails`);
            } else {
                throw new Error(result.message || 'Failed to regenerate thumbnails');
            }
        } catch (error) {
            RustMedia.Toast.error(error.message);
        } finally {
            if (btn) {
                btn.disabled = false;
                btn.textContent = 'Regenerate All Thumbnails';
            }
        }
    },

    // Export settings
    exportSettings() {
        const formData = new FormData(this.form);
        const data = {};

        for (const [key, value] of formData.entries()) {
            data[key] = value;
        }

        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = 'rustmedia-settings.json';
        a.click();

        URL.revokeObjectURL(url);
    },

    // Import settings
    importSettings(file) {
        const reader = new FileReader();

        reader.onload = async (e) => {
            try {
                const data = JSON.parse(e.target.result);

                // Populate form
                for (const [key, value] of Object.entries(data)) {
                    const input = this.form.querySelector(`[name="${key}"]`);
                    if (input) {
                        if (input.type === 'checkbox') {
                            input.checked = !!value;
                        } else {
                            input.value = value;
                        }
                    }
                }

                RustMedia.Toast.success('Settings imported. Save to apply.');
            } catch (error) {
                RustMedia.Toast.error('Invalid settings file');
            }
        };

        reader.readAsText(file);
    },
};

// Global functions for inline handlers
window.addThumbnailSize = function() {
    RustMedia.Settings.addThumbnailSize();
};

window.testStorage = function() {
    RustMedia.Settings.testStorage();
};

window.clearCache = function() {
    RustMedia.Settings.clearCache();
};

window.regenerateThumbnails = function() {
    RustMedia.Settings.regenerateThumbnails();
};

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => {
    RustMedia.Settings.init();
});
