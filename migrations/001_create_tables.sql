-- RustMedia Database Schema
-- Migration: 001_create_tables

-- Media items table
CREATE TABLE IF NOT EXISTS media_items (
    id UUID PRIMARY KEY,
    filename VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL,
    title VARCHAR(255),
    description TEXT,
    alt_text VARCHAR(500),
    mime_type VARCHAR(100) NOT NULL,
    media_type VARCHAR(50) NOT NULL,
    size BIGINT NOT NULL,
    path VARCHAR(500) NOT NULL,
    url VARCHAR(500) NOT NULL,
    folder_id UUID REFERENCES media_folders(id) ON DELETE SET NULL,
    width INTEGER,
    height INTEGER,
    duration REAL,
    content_hash VARCHAR(64) NOT NULL,
    uploaded_by UUID,
    uploaded_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE,
    UNIQUE(content_hash)
);

-- Create indexes for media items
CREATE INDEX IF NOT EXISTS idx_media_items_folder ON media_items(folder_id);
CREATE INDEX IF NOT EXISTS idx_media_items_media_type ON media_items(media_type);
CREATE INDEX IF NOT EXISTS idx_media_items_mime_type ON media_items(mime_type);
CREATE INDEX IF NOT EXISTS idx_media_items_uploaded_at ON media_items(uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_media_items_uploaded_by ON media_items(uploaded_by);
CREATE INDEX IF NOT EXISTS idx_media_items_deleted_at ON media_items(deleted_at);
CREATE INDEX IF NOT EXISTS idx_media_items_slug ON media_items(slug);
CREATE INDEX IF NOT EXISTS idx_media_items_content_hash ON media_items(content_hash);

-- Full text search index
CREATE INDEX IF NOT EXISTS idx_media_items_search ON media_items
    USING gin(to_tsvector('english', coalesce(filename, '') || ' ' || coalesce(title, '') || ' ' || coalesce(description, '')));

-- Media folders table
CREATE TABLE IF NOT EXISTS media_folders (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL,
    description TEXT,
    parent_id UUID REFERENCES media_folders(id) ON DELETE CASCADE,
    path VARCHAR(1000) NOT NULL,
    depth INTEGER NOT NULL DEFAULT 0,
    item_count INTEGER NOT NULL DEFAULT 0,
    total_size BIGINT NOT NULL DEFAULT 0,
    created_by UUID,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(parent_id, slug)
);

-- Create indexes for folders
CREATE INDEX IF NOT EXISTS idx_media_folders_parent ON media_folders(parent_id);
CREATE INDEX IF NOT EXISTS idx_media_folders_path ON media_folders(path);
CREATE INDEX IF NOT EXISTS idx_media_folders_slug ON media_folders(slug);
CREATE INDEX IF NOT EXISTS idx_media_folders_created_by ON media_folders(created_by);

-- Media thumbnails table
CREATE TABLE IF NOT EXISTS media_thumbnails (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    size_name VARCHAR(50) NOT NULL,
    url VARCHAR(500) NOT NULL,
    path VARCHAR(500) NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(media_id, size_name)
);

-- Create indexes for thumbnails
CREATE INDEX IF NOT EXISTS idx_media_thumbnails_media ON media_thumbnails(media_id);

-- Media tags table
CREATE TABLE IF NOT EXISTS media_tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    slug VARCHAR(100) NOT NULL UNIQUE,
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index for tags
CREATE INDEX IF NOT EXISTS idx_media_tags_slug ON media_tags(slug);
CREATE INDEX IF NOT EXISTS idx_media_tags_usage ON media_tags(usage_count DESC);

-- Media item tags junction table
CREATE TABLE IF NOT EXISTS media_item_tags (
    media_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES media_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (media_id, tag_id)
);

-- Create indexes for item tags
CREATE INDEX IF NOT EXISTS idx_media_item_tags_media ON media_item_tags(media_id);
CREATE INDEX IF NOT EXISTS idx_media_item_tags_tag ON media_item_tags(tag_id);

-- Media metadata table (for EXIF, etc.)
CREATE TABLE IF NOT EXISTS media_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    key VARCHAR(100) NOT NULL,
    value TEXT,
    UNIQUE(media_id, key)
);

-- Create index for metadata
CREATE INDEX IF NOT EXISTS idx_media_metadata_media ON media_metadata(media_id);
CREATE INDEX IF NOT EXISTS idx_media_metadata_key ON media_metadata(key);

-- Chunked uploads table
CREATE TABLE IF NOT EXISTS media_chunked_uploads (
    id UUID PRIMARY KEY,
    filename VARCHAR(255) NOT NULL,
    total_size BIGINT NOT NULL,
    chunk_size INTEGER NOT NULL,
    total_chunks INTEGER NOT NULL,
    received_chunks INTEGER NOT NULL DEFAULT 0,
    mime_type VARCHAR(100),
    folder_id UUID REFERENCES media_folders(id) ON DELETE SET NULL,
    user_id UUID,
    temp_path VARCHAR(500) NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Create index for chunked uploads
CREATE INDEX IF NOT EXISTS idx_media_chunked_uploads_user ON media_chunked_uploads(user_id);
CREATE INDEX IF NOT EXISTS idx_media_chunked_uploads_expires ON media_chunked_uploads(expires_at);

-- Upload chunks tracking table
CREATE TABLE IF NOT EXISTS media_upload_chunks (
    upload_id UUID NOT NULL REFERENCES media_chunked_uploads(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    size INTEGER NOT NULL,
    received BOOLEAN NOT NULL DEFAULT FALSE,
    checksum VARCHAR(64),
    received_at TIMESTAMP WITH TIME ZONE,
    PRIMARY KEY (upload_id, chunk_index)
);

-- Media usage tracking table
CREATE TABLE IF NOT EXISTS media_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    media_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID NOT NULL,
    context VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for usage tracking
CREATE INDEX IF NOT EXISTS idx_media_usage_media ON media_usage(media_id);
CREATE INDEX IF NOT EXISTS idx_media_usage_entity ON media_usage(entity_type, entity_id);

-- Media settings table
CREATE TABLE IF NOT EXISTS media_settings (
    key VARCHAR(100) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Insert default settings
INSERT INTO media_settings (key, value) VALUES
    ('storage_backend', '"local"'),
    ('storage_path', '"uploads/media"'),
    ('base_url', '"/media"'),
    ('max_file_size', '104857600'),
    ('jpeg_quality', '85'),
    ('png_compression', '6'),
    ('webp_quality', '80'),
    ('auto_optimize', 'true'),
    ('generate_thumbnails', 'true'),
    ('organize_by_date', 'true'),
    ('deduplicate', 'true')
ON CONFLICT (key) DO NOTHING;

-- Functions

-- Function to update folder stats
CREATE OR REPLACE FUNCTION update_folder_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE media_folders
        SET item_count = item_count + 1,
            total_size = total_size + NEW.size,
            updated_at = NOW()
        WHERE id = NEW.folder_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE media_folders
        SET item_count = item_count - 1,
            total_size = total_size - OLD.size,
            updated_at = NOW()
        WHERE id = OLD.folder_id;
    ELSIF TG_OP = 'UPDATE' AND OLD.folder_id IS DISTINCT FROM NEW.folder_id THEN
        -- Item moved to different folder
        IF OLD.folder_id IS NOT NULL THEN
            UPDATE media_folders
            SET item_count = item_count - 1,
                total_size = total_size - OLD.size,
                updated_at = NOW()
            WHERE id = OLD.folder_id;
        END IF;
        IF NEW.folder_id IS NOT NULL THEN
            UPDATE media_folders
            SET item_count = item_count + 1,
                total_size = total_size + NEW.size,
                updated_at = NOW()
            WHERE id = NEW.folder_id;
        END IF;
    END IF;
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Trigger to update folder stats
DROP TRIGGER IF EXISTS media_items_folder_stats ON media_items;
CREATE TRIGGER media_items_folder_stats
    AFTER INSERT OR UPDATE OR DELETE ON media_items
    FOR EACH ROW EXECUTE FUNCTION update_folder_stats();

-- Function to update tag usage count
CREATE OR REPLACE FUNCTION update_tag_usage()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE media_tags SET usage_count = usage_count + 1 WHERE id = NEW.tag_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE media_tags SET usage_count = usage_count - 1 WHERE id = OLD.tag_id;
    END IF;
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Trigger to update tag usage
DROP TRIGGER IF EXISTS media_item_tags_usage ON media_item_tags;
CREATE TRIGGER media_item_tags_usage
    AFTER INSERT OR DELETE ON media_item_tags
    FOR EACH ROW EXECUTE FUNCTION update_tag_usage();

-- Function to clean up expired chunked uploads
CREATE OR REPLACE FUNCTION cleanup_expired_uploads()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM media_chunked_uploads
    WHERE expires_at < NOW() AND completed_at IS NULL;
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Views

-- View for media with thumbnails
CREATE OR REPLACE VIEW media_with_thumbnails AS
SELECT
    m.*,
    COALESCE(
        json_agg(
            json_build_object(
                'size_name', t.size_name,
                'url', t.url,
                'width', t.width,
                'height', t.height
            )
        ) FILTER (WHERE t.id IS NOT NULL),
        '[]'
    ) as thumbnails
FROM media_items m
LEFT JOIN media_thumbnails t ON m.id = t.media_id
WHERE m.deleted_at IS NULL
GROUP BY m.id;

-- View for media with tags
CREATE OR REPLACE VIEW media_with_tags AS
SELECT
    m.*,
    COALESCE(array_agg(t.name) FILTER (WHERE t.id IS NOT NULL), '{}') as tags
FROM media_items m
LEFT JOIN media_item_tags mt ON m.id = mt.media_id
LEFT JOIN media_tags t ON mt.tag_id = t.id
WHERE m.deleted_at IS NULL
GROUP BY m.id;

-- View for folder hierarchy
CREATE OR REPLACE VIEW folder_hierarchy AS
WITH RECURSIVE folder_tree AS (
    SELECT id, name, slug, parent_id, path, depth, 1 as level,
           ARRAY[id] as ancestors
    FROM media_folders
    WHERE parent_id IS NULL

    UNION ALL

    SELECT f.id, f.name, f.slug, f.parent_id, f.path, f.depth,
           ft.level + 1, ft.ancestors || f.id
    FROM media_folders f
    JOIN folder_tree ft ON f.parent_id = ft.id
)
SELECT * FROM folder_tree;

-- Permissions (adjust for your auth system)
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO rustpress_app;
-- GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO rustpress_app;
