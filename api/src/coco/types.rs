use serde::{Deserialize, Serialize};

// COCO format data structures
#[derive(Debug, Serialize, Deserialize)]
pub struct CocoExport {
    pub info: CocoInfo,
    pub licenses: Vec<CocoLicense>,
    pub images: Vec<CocoImage>,
    pub annotations: Vec<CocoAnnotation>,
    pub categories: Vec<CocoCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoInfo {
    pub year: i32,
    pub version: String,
    pub description: String,
    pub contributor: String,
    pub url: String,
    pub date_created: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoLicense {
    pub id: i32,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoImage {
    pub id: i64,
    pub width: i32,
    pub height: i32,
    pub file_name: String,
    pub license: i32,
    pub flickr_url: Option<String>,
    pub coco_url: Option<String>,
    pub date_captured: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoAnnotation {
    pub id: i64,
    pub image_id: i64,
    pub category_id: i32,
    pub segmentation: Vec<Vec<f64>>,
    pub area: i32,
    pub bbox: Vec<f64>,
    pub iscrowd: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoCategory {
    pub id: i32,
    pub name: String,
    pub supercategory: String,
}

// Import specific structures
#[derive(Debug, Deserialize)]
pub struct CocoImport {
    #[allow(dead_code)]
    pub info: Option<CocoInfo>,
    #[allow(dead_code)]
    pub licenses: Option<Vec<CocoLicense>>,
    pub images: Vec<CocoImage>,
    pub annotations: Vec<CocoAnnotation>,
    pub categories: Vec<CocoCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub message: String,
    pub stats: ImportStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportStats {
    pub categories_created: usize,
    pub categories_updated: usize,
    pub tasks_created: usize,
    pub annotations_created: usize,
    pub errors: Vec<String>,
}