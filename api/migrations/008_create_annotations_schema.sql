-- Create annotations and image_annotation_categories tables
-- This migration consolidates the annotation schema without generic annotation_categories

-- Create annotations table for storing annotation data
CREATE TABLE annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    
    -- Additional metadata for future extensibility
    metadata JSONB DEFAULT '{}'::jsonb,
    
    -- Annotation author information
    annotated_by UUID REFERENCES users(id),
    annotated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create image-specific annotation categories table
CREATE TABLE image_annotation_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- MS COCO specific fields
    supercategory VARCHAR(255),
    color VARCHAR(7), -- HEX color for UI display (#FF0000)
    coco_id INTEGER, -- MS COCO format export用のID
    
    -- Image-specific metadata
    image_metadata JSONB DEFAULT '{}'::jsonb,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(project_id, name)
);

-- Create image-specific annotations table
CREATE TABLE image_annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    annotation_id UUID NOT NULL REFERENCES annotations(id) ON DELETE CASCADE,
    category_id UUID REFERENCES image_annotation_categories(id) ON DELETE SET NULL,
    
    -- MS COCO compatible geometry data
    bbox FLOAT[4] NOT NULL, -- [x, y, width, height] in MS COCO format
    area FLOAT,
    iscrowd BOOLEAN DEFAULT FALSE,
    
    -- Image-specific metadata
    image_metadata JSONB DEFAULT '{}'::jsonb,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for annotations
CREATE INDEX idx_annotations_task_id ON annotations(task_id);
CREATE INDEX idx_annotations_annotated_by ON annotations(annotated_by);
CREATE INDEX idx_annotations_annotated_at ON annotations(annotated_at DESC);
CREATE INDEX idx_annotations_created_at ON annotations(created_at DESC);
CREATE INDEX idx_annotations_metadata ON annotations USING GIN (metadata);

-- Create indexes for image_annotation_categories
CREATE INDEX idx_image_annotation_categories_project_id ON image_annotation_categories(project_id);
CREATE INDEX idx_image_annotation_categories_name ON image_annotation_categories(name);
CREATE INDEX idx_image_annotation_categories_coco_id ON image_annotation_categories(coco_id);
CREATE INDEX idx_image_annotation_categories_created_at ON image_annotation_categories(created_at DESC);
CREATE INDEX idx_image_annotation_categories_image_metadata ON image_annotation_categories USING GIN (image_metadata);

-- Create indexes for image_annotations
CREATE INDEX idx_image_annotations_annotation_id ON image_annotations(annotation_id);
CREATE INDEX idx_image_annotations_category_id ON image_annotations(category_id);
CREATE INDEX idx_image_annotations_created_at ON image_annotations(created_at DESC);
CREATE INDEX idx_image_annotations_image_metadata ON image_annotations USING GIN (image_metadata);

-- Add comments for documentation
COMMENT ON TABLE annotations IS 'Generic annotation data for tasks, extensible for multiple annotation types';
COMMENT ON COLUMN annotations.metadata IS 'Additional metadata for future extensibility';

COMMENT ON TABLE image_annotation_categories IS 'Image-specific annotation categories, compatible with MS COCO format';
COMMENT ON COLUMN image_annotation_categories.supercategory IS 'MS COCO supercategory for hierarchical classification';
COMMENT ON COLUMN image_annotation_categories.color IS 'HEX color for UI display';
COMMENT ON COLUMN image_annotation_categories.coco_id IS 'MS COCO format export ID';

COMMENT ON TABLE image_annotations IS 'Image-specific annotation data, compatible with MS COCO format';
COMMENT ON COLUMN image_annotations.bbox IS 'Bounding box in MS COCO format: [x, y, width, height]';
COMMENT ON COLUMN image_annotations.area IS 'Area of the annotation (calculated from bbox)';
COMMENT ON COLUMN image_annotations.iscrowd IS 'MS COCO iscrowd flag for crowd annotations';