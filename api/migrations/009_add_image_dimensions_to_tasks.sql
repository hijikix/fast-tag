-- Add image dimensions to tasks table
ALTER TABLE tasks 
ADD COLUMN width INTEGER,
ADD COLUMN height INTEGER;

-- Add comment to explain the purpose of these columns
COMMENT ON COLUMN tasks.width IS 'Width of the image in pixels';
COMMENT ON COLUMN tasks.height IS 'Height of the image in pixels';

-- Create index for potential filtering by image dimensions
CREATE INDEX idx_tasks_dimensions ON tasks(width, height) WHERE width IS NOT NULL AND height IS NOT NULL;