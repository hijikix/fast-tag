-- Add storage configuration to projects table
ALTER TABLE projects ADD COLUMN storage_config JSONB;

-- Create index for storage config queries
CREATE INDEX idx_projects_storage_config ON projects USING GIN (storage_config);

-- Add comment for documentation
COMMENT ON COLUMN projects.storage_config IS 'JSON configuration for project storage provider (S3, Azure, GCS, MinIO, etc.)';