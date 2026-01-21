use std::{
    path::{Path, PathBuf},
    sync::{LazyLock, mpsc},
};

use egui::{ColorImage, TextureHandle, TextureId, Vec2, ahash::HashMap, mutex::Mutex};

static TEXTURE_CACHE: LazyLock<Mutex<TextureCache>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel();
    Mutex::new(TextureCache {
        entries: HashMap::default(),
        rx,
        tx,
    })
});

#[derive(Clone)]
pub struct TextureInfo {
    pub id: TextureId,
    pub size: Vec2,
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
        while let Ok((path, color_image)) = cache.rx.try_recv() {
            let size = Vec2::new(color_image.width() as f32, color_image.height() as f32);
            let handle = ctx.load_texture(path.to_string_lossy(), color_image, Default::default());

            cache.entries.insert(
                path,
                LoadingState::Loaded(TextureInfo {
                    id: handle.id(),
                    handle,
                    size,
                }),
            );
        }
    }

    /// Submit an image path to be loaded
    pub fn request_load(path: &Path) {
        let mut cache = TEXTURE_CACHE.lock();
        let path_buf = path.to_path_buf();

        if cache.entries.contains_key(&path_buf) {
            return; // Already loading or loaded
        }

        cache
            .entries
            .insert(path_buf.clone(), LoadingState::Pending);
        let tx = cache.tx.clone();

        std::thread::spawn(move || {
            if let Ok(img) = image::open(&path_buf) {
                let rgba = img.to_rgba8();
                let color_image = ColorImage::from_rgba_unmultiplied(
                    [img.width() as usize, img.height() as usize],
                    rgba.as_flat_samples().as_slice(),
                );
                let _ = tx.send((path_buf, color_image));
            } else {
                eprintln!("Failed to load: {path_buf:?}");
            }
        });
    }

    /// Get the texture info if available
    pub fn get(path: &Path) -> Option<TextureInfo> {
        let cache = TEXTURE_CACHE.lock();
        match cache.entries.get(path) {
            Some(LoadingState::Loaded(info)) => Some(info.clone()),
            _ => None,
        }
    }
}
