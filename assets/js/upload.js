/**
 * RustMedia Upload Handler
 */

RustMedia.Upload = {
    config: window.RUSTMEDIA_CONFIG || {},
    queue: [],
    uploading: false,

    init() {
        this.dropzone = document.getElementById('dropzone');
        this.fileInput = document.getElementById('file-input');
        this.queueList = document.getElementById('queue-list');
        this.completeList = document.getElementById('complete-list');

        if (!this.dropzone) return;

        this.bindEvents();
    },

    bindEvents() {
        // Click to select
        this.dropzone.addEventListener('click', () => {
            this.fileInput.click();
        });

        // File input change
        this.fileInput.addEventListener('change', (e) => {
            this.addFiles(e.target.files);
            e.target.value = '';
        });

        // Drag and drop
        this.dropzone.addEventListener('dragover', (e) => {
            e.preventDefault();
            this.dropzone.classList.add('dragover');
        });

        this.dropzone.addEventListener('dragleave', () => {
            this.dropzone.classList.remove('dragover');
        });

        this.dropzone.addEventListener('drop', (e) => {
            e.preventDefault();
            this.dropzone.classList.remove('dragover');
            this.addFiles(e.dataTransfer.files);
        });

        // Paste
        document.addEventListener('paste', (e) => {
            const files = e.clipboardData?.files;
            if (files?.length) {
                this.addFiles(files);
            }
        });
    },

    addFiles(files) {
        for (const file of files) {
            if (!this.validateFile(file)) continue;

            const queueItem = {
                id: this.generateId(),
                file,
                status: 'pending',
                progress: 0,
                error: null,
            };

            this.queue.push(queueItem);
            this.renderQueueItem(queueItem);
        }

        this.processQueue();
    },

    validateFile(file) {
        // Check size
        if (file.size > this.config.maxFileSize) {
            RustMedia.Toast.error(`File "${file.name}" is too large. Maximum size is ${RustMedia.Format.fileSize(this.config.maxFileSize)}`);
            return false;
        }

        // Check extension
        const ext = file.name.split('.').pop().toLowerCase();
        if (this.config.allowedExtensions && !this.config.allowedExtensions.includes(ext)) {
            RustMedia.Toast.error(`File type ".${ext}" is not allowed`);
            return false;
        }

        return true;
    },

    generateId() {
        return `upload_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    },

    renderQueueItem(item) {
        const div = document.createElement('div');
        div.className = 'queue-item';
        div.id = item.id;
        div.innerHTML = `
            <span class="filename">${this.escapeHtml(item.file.name)}</span>
            <span class="filesize">${RustMedia.Format.fileSize(item.file.size)}</span>
            <div class="progress">
                <div class="progress-bar" style="width: 0%"></div>
            </div>
            <span class="status">Waiting...</span>
            <button class="btn-cancel" onclick="RustMedia.Upload.cancel('${item.id}')">Cancel</button>
        `;
        this.queueList.appendChild(div);
    },

    updateQueueItem(item) {
        const div = document.getElementById(item.id);
        if (!div) return;

        const progressBar = div.querySelector('.progress-bar');
        const status = div.querySelector('.status');

        progressBar.style.width = `${item.progress}%`;

        switch (item.status) {
            case 'uploading':
                status.textContent = `Uploading ${item.progress}%`;
                break;
            case 'processing':
                status.textContent = 'Processing...';
                break;
            case 'complete':
                status.textContent = 'Complete';
                div.classList.add('complete');
                div.querySelector('.btn-cancel').style.display = 'none';
                break;
            case 'error':
                status.textContent = `Error: ${item.error}`;
                div.classList.add('error');
                break;
        }
    },

    async processQueue() {
        if (this.uploading) return;

        const item = this.queue.find(i => i.status === 'pending');
        if (!item) return;

        this.uploading = true;
        item.status = 'uploading';
        this.updateQueueItem(item);

        try {
            const folderId = document.getElementById('folder-select')?.value || null;
            const optimize = document.getElementById('optimize-images')?.checked ?? true;
            const thumbnails = document.getElementById('generate-thumbnails')?.checked ?? true;

            // Use chunked upload for large files
            if (item.file.size > this.config.chunkSize) {
                await this.uploadChunked(item, folderId);
            } else {
                await this.uploadSimple(item, folderId, optimize, thumbnails);
            }

            item.status = 'complete';
            item.progress = 100;
            this.updateQueueItem(item);
            this.moveToComplete(item);

            RustMedia.Toast.success(`Uploaded: ${item.file.name}`);
        } catch (error) {
            item.status = 'error';
            item.error = error.message;
            this.updateQueueItem(item);
            RustMedia.Toast.error(`Failed: ${item.file.name}`);
        }

        this.uploading = false;
        this.processQueue();
    },

    async uploadSimple(item, folderId, optimize, thumbnails) {
        const formData = new FormData();
        formData.append('file', item.file);
        if (folderId) formData.append('folder_id', folderId);
        formData.append('optimize', optimize);
        formData.append('generate_thumbnails', thumbnails);

        const xhr = new XMLHttpRequest();

        return new Promise((resolve, reject) => {
            xhr.upload.addEventListener('progress', (e) => {
                if (e.lengthComputable) {
                    item.progress = Math.round((e.loaded / e.total) * 100);
                    this.updateQueueItem(item);
                }
            });

            xhr.addEventListener('load', () => {
                if (xhr.status >= 200 && xhr.status < 300) {
                    item.result = JSON.parse(xhr.responseText);
                    resolve(item.result);
                } else {
                    const error = JSON.parse(xhr.responseText);
                    reject(new Error(error.message || 'Upload failed'));
                }
            });

            xhr.addEventListener('error', () => {
                reject(new Error('Network error'));
            });

            xhr.open('POST', '/api/media/upload');
            xhr.send(formData);
        });
    },

    async uploadChunked(item, folderId) {
        const chunkSize = this.config.chunkSize || 5 * 1024 * 1024;
        const totalChunks = Math.ceil(item.file.size / chunkSize);

        // Initialize upload
        const initResponse = await RustMedia.API.initChunkedUpload({
            filename: item.file.name,
            total_size: item.file.size,
            chunk_size: chunkSize,
            total_chunks: totalChunks,
            folder_id: folderId,
        });

        const uploadId = initResponse.upload_id;

        // Upload chunks
        for (let i = 0; i < totalChunks; i++) {
            const start = i * chunkSize;
            const end = Math.min(start + chunkSize, item.file.size);
            const chunk = item.file.slice(start, end);

            await RustMedia.API.uploadChunk(uploadId, i, chunk);

            item.progress = Math.round(((i + 1) / totalChunks) * 90);
            this.updateQueueItem(item);
        }

        // Complete upload
        item.status = 'processing';
        this.updateQueueItem(item);

        item.result = await RustMedia.API.completeChunkedUpload(uploadId);
        return item.result;
    },

    cancel(itemId) {
        const index = this.queue.findIndex(i => i.id === itemId);
        if (index === -1) return;

        const item = this.queue[index];

        if (item.status === 'uploading' && item.xhr) {
            item.xhr.abort();
        }

        if (item.uploadId) {
            RustMedia.API.cancelChunkedUpload(item.uploadId).catch(() => {});
        }

        this.queue.splice(index, 1);
        document.getElementById(itemId)?.remove();
    },

    moveToComplete(item) {
        const queueItem = document.getElementById(item.id);
        if (queueItem) {
            setTimeout(() => {
                queueItem.remove();
                this.renderCompleteItem(item);
            }, 1000);
        }
    },

    renderCompleteItem(item) {
        const completeSection = document.getElementById('upload-complete');
        if (completeSection) {
            completeSection.style.display = 'block';
        }

        const div = document.createElement('div');
        div.className = 'complete-item';
        div.innerHTML = `
            <div class="item-thumb">
                ${item.result?.thumbnails?.[0]?.url
                    ? `<img src="${item.result.thumbnails[0].url}" alt="">`
                    : '<div class="file-icon">📄</div>'
                }
            </div>
            <div class="item-info">
                <span class="filename">${this.escapeHtml(item.file.name)}</span>
                <span class="meta">${RustMedia.Format.fileSize(item.file.size)}</span>
            </div>
            <div class="item-actions">
                <a href="/admin/media/${item.result?.id}" class="btn btn-small">View</a>
                <button class="btn btn-small" onclick="RustMedia.Upload.copyUrl('${item.result?.url}')">Copy URL</button>
            </div>
        `;
        this.completeList.appendChild(div);
    },

    copyUrl(url) {
        if (!url) return;
        navigator.clipboard.writeText(url).then(() => {
            RustMedia.Toast.success('URL copied to clipboard');
        }).catch(() => {
            RustMedia.Toast.error('Failed to copy URL');
        });
    },

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    },
};

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => {
    RustMedia.Upload.init();
});
