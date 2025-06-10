-- Add annotation_type column to projects table
ALTER TABLE projects ADD COLUMN annotation_type VARCHAR(50) NOT NULL DEFAULT 'image';

-- Create index for annotation type queries
CREATE INDEX idx_projects_annotation_type ON projects(annotation_type);

-- Add comment for documentation
COMMENT ON COLUMN projects.annotation_type IS 'Type of annotation for this project (image, video, audio, text, etc.)';