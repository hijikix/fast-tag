-- Create project_syncs table for tracking storage sync operations
CREATE TABLE project_syncs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'running', -- 'running', 'completed', 'completed_with_errors', 'failed'
    total_files INTEGER NOT NULL DEFAULT 0,
    processed_files INTEGER NOT NULL DEFAULT 0,
    tasks_created INTEGER NOT NULL DEFAULT 0,
    tasks_skipped INTEGER NOT NULL DEFAULT 0,
    errors JSONB NOT NULL DEFAULT '[]'::jsonb,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Create indexes for faster lookups
CREATE INDEX idx_project_syncs_project_id ON project_syncs(project_id);
CREATE INDEX idx_project_syncs_status ON project_syncs(status);
CREATE INDEX idx_project_syncs_started_at ON project_syncs(started_at DESC);

-- Add comment for documentation
COMMENT ON TABLE project_syncs IS 'Tracks storage synchronization operations that create tasks from storage files';