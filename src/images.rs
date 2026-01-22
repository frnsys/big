use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{LazyLock, mpsc},
};

use egui::{ColorImage, TextureHandle, TextureId, Vec2, ahash::HashMap, mutex::Mutex};
use image::{DynamicImage, ImageError, ImageFormat, ImageResult};

use crate::notifs::Notifications;

static TEXTURE_CACHE: LazyLock<Mutex<TextureCache>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel();
    Mutex::new(TextureCache {
        entries: HashMap::default(),
        rx,
        tx,
    })
});

static THUMBS_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = dirs::cache_dir().unwrap();
    let dir = dir.join("big");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).unwrap();
    }
    dir
});

#[derive(Clone)]
pub struct TextureInfo {
    pub id: TextureId,

    // Hold on to a reference
    #[allow(unused)]
    pub handle: TextureHandle,
}

enum LoadingState {
    Pending,
    Loaded(TextureInfo),
}

pub struct TextureCache {
    entries: HashMap<PathBuf, LoadingState>,
    rx: mpsc::Receiver<(PathBuf, ColorImage)>,
    tx: mpsc::Sender<(PathBuf, ColorImage)>,
}
impl TextureCache {
    /// Call every frame to update processing
    pub fn update(ctx: &egui::Context) {
        let mut cache = TEXTURE_CACHE.lock();

        // Process all images that finished loading in the background
        while let Ok((path, image)) = cache.rx.try_recv() {
            let handle = ctx.load_texture(path.to_string_lossy(), image, Default::default());
            ctx.request_repaint();

            cache.entries.insert(
                path,
                LoadingState::Loaded(TextureInfo {
                    id: handle.id(),
                    handle,
                }),
            );
        }

        let some_loading = cache
            .entries
            .values()
            .any(|state| matches!(state, LoadingState::Pending));
        if some_loading {
            ctx.request_repaint();
        }
    }

    /// Submit an image to be loaded
    pub fn request_load(image: ImageRequest<'_>) {
        let mut cache = TEXTURE_CACHE.lock();

        let path_buf = image.path();

        if cache.entries.contains_key(&path_buf) {
            return; // Already loading or loaded
        }

        cache
            .entries
            .insert(path_buf.clone(), LoadingState::Pending);
        let tx = cache.tx.clone();

        let size = image.size();
        let source = image.source.to_path_buf();
        std::thread::spawn(move || {
            if let Ok(img) = load(&source, size, &path_buf) {
                let rgba = img.to_rgba8();
                let color_image = ColorImage::from_rgba_unmultiplied(
                    [img.width() as usize, img.height() as usize],
                    rgba.as_flat_samples().as_slice(),
                );
                let _ = tx.send((path_buf, color_image));
            } else {
                eprintln!("Failed to load: {path_buf:?}");
                Notifications::push(format!("Failed to load: {path_buf:?}"));
            }
        });
    }

    /// Get the texture info if available
    pub fn get(image: &ImageRequest<'_>) -> Option<TextureInfo> {
        let path = image.path();
        let cache = TEXTURE_CACHE.lock();
        match cache.entries.get(&path) {
            Some(LoadingState::Loaded(info)) => Some(info.clone()),
            _ => None,
        }
    }
}

/// An image load or get request.
/// This contains enough info to determine
/// which level-of-detail to use.
pub struct ImageRequest<'a> {
    pub source: &'a Path,
    pub source_size: Vec2,
    pub target_size: Vec2,
    pub image_hash: u128,
}
impl<'a> ImageRequest<'a> {
    fn path(&self) -> PathBuf {
        thumbnail_path(self.source, self.image_hash, self.size())
    }

    fn size(&self) -> Lod {
        thumbnail_for_size(self.source_size, self.target_size)
    }
}

/// Load an image at the requested LOD.
/// This will generate thumbnails as needed, caching them for future runs.
fn load(source: &Path, size: Lod, path: &Path) -> ImageResult<DynamicImage> {
    if !path.exists() {
        match size {
            Lod::Low => create_thumbnail(source, Lod::side_small() as u32, &path),
            Lod::Medium => create_thumbnail(source, Lod::side_medium() as u32, &path),
            Lod::Full => Err(ImageError::IoError(std::io::Error::new(
                ErrorKind::NotFound,
                "Source image not found",
            ))),
        }
    } else {
        image::open(path)
    }
}

/// Level-of-detail
#[derive(Clone, Copy)]
enum Lod {
    Low,
    Medium,
    Full,
}
impl Lod {
    const fn side_small() -> f32 {
        256.
    }

    const fn side_medium() -> f32 {
        768.
    }
}

fn thumbnail_for_size(image_size: Vec2, target_size: Vec2) -> Lod {
    let target = target_size.max_elem();
    if target >= image_size.max_elem() {
        Lod::Full
    } else if target < Lod::side_small() {
        Lod::Low
    } else if target < Lod::side_medium() {
        Lod::Medium
    } else {
        Lod::Full
    }
}

fn thumbnail_path(source: &Path, hash: u128, size: Lod) -> PathBuf {
    match size {
        Lod::Low => THUMBS_DIR.join(format!("{hash}.s.jpg")),
        Lod::Medium => THUMBS_DIR.join(format!("{hash}.m.jpg")),
        Lod::Full => source.to_path_buf(),
    }
}

fn create_thumbnail(source: &Path, size: u32, path: &Path) -> ImageResult<DynamicImage> {
    let img = image::open(source)?;
    let thumb = img.thumbnail(size, size);
    thumb.save_with_format(path, ImageFormat::Jpeg)?;
    Ok(thumb)
}
