use std::{
    fs::File,
    io::{BufReader, ErrorKind},
    path::{Path, PathBuf},
    sync::{LazyLock, mpsc},
    time::Duration,
};

use egui::{ColorImage, Context, TextureHandle, TextureId, Vec2, ahash::HashMap, mutex::Mutex};
use image::{AnimationDecoder, DynamicImage, ImageError, ImageFormat, ImageResult};

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

struct Sequence<T> {
    frames: Vec<T>,
    delays: Vec<Duration>,
}

enum Image<T> {
    Single(T),
    Sequence(Sequence<T>),
}

enum LoadingState {
    Pending,
    Loaded(Image<TextureInfo>),
}

pub struct TextureCache {
    entries: HashMap<PathBuf, LoadingState>,
    rx: mpsc::Receiver<(PathBuf, Image<ColorImage>)>,
    tx: mpsc::Sender<(PathBuf, Image<ColorImage>)>,
}
impl TextureCache {
    /// Call every frame to update processing
    pub fn update(ctx: &egui::Context) {
        let mut cache = TEXTURE_CACHE.lock();

        // Process all images that finished loading in the background
        while let Ok((path, image)) = cache.rx.try_recv() {
            let image: Image<TextureInfo> = match image {
                Image::Single(image) => {
                    let handle =
                        ctx.load_texture(path.to_string_lossy(), image, Default::default());
                    Image::Single(TextureInfo {
                        id: handle.id(),
                        handle,
                    })
                }
                Image::Sequence(Sequence { frames, delays }) => {
                    let frames = frames
                        .into_iter()
                        .enumerate()
                        .map(|(i, image)| {
                            let path = format!("{}#{i}", path.to_string_lossy());
                            let handle = ctx.load_texture(path, image, Default::default());
                            TextureInfo {
                                id: handle.id(),
                                handle,
                            }
                        })
                        .collect();
                    Image::Sequence(Sequence { frames, delays })
                }
            };

            cache.entries.insert(path, LoadingState::Loaded(image));
            ctx.request_repaint();
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

        let path = image.path();

        if cache.entries.contains_key(&path) {
            return; // Already loading or loaded
        }

        cache.entries.insert(path.clone(), LoadingState::Pending);
        let tx = cache.tx.clone();

        let size = image.size();
        let source = image.source.to_path_buf();
        std::thread::spawn(move || {
            if let Ok(img) = load(&source, size, &path) {
                let _ = tx.send((path, img));
            } else {
                eprintln!("Failed to load: {path:?}");
                Notifications::push(format!("Failed to load: {path:?}"));
            }
        });
    }

    /// Get the texture info if available
    pub fn get(ctx: &Context, image: &ImageRequest<'_>) -> Option<TextureInfo> {
        let path = image.path();
        let cache = TEXTURE_CACHE.lock();
        match cache.entries.get(&path) {
            Some(LoadingState::Loaded(info)) => Some(info.get_info(ctx).clone()),
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
fn load(source: &Path, size: Lod, path: &Path) -> ImageResult<Image<ColorImage>> {
    if !path.exists() {
        match size {
            Lod::Low => create_thumbnail(source, Lod::side_small() as u32, path)
                .map(to_color_image)
                .map(Image::Single),
            Lod::Medium => create_thumbnail(source, Lod::side_medium() as u32, path)
                .map(to_color_image)
                .map(Image::Single),
            Lod::Full => Err(ImageError::IoError(std::io::Error::new(
                ErrorKind::NotFound,
                "Source image not found",
            ))),
        }
    } else {
        if path.extension().is_some_and(|ext| ext == "gif") {
            load_gif(path).map(Image::Sequence)
        } else {
            image::open(path).map(to_color_image).map(Image::Single)
        }
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

impl Image<TextureInfo> {
    fn get_info(&self, ctx: &Context) -> &TextureInfo {
        match self {
            Image::Single(info) => info,
            Image::Sequence(sequence) => {
                let idx = gif_frame_index(ctx, &sequence.delays);
                &sequence.frames[idx]
            }
        }
    }
}

fn gif_frame_index(ctx: &Context, durations: &[Duration]) -> usize {
    let elapsed = ctx.input(|i| Duration::from_secs_f64(i.time));
    let frames: Duration = durations.iter().sum();
    let pos_ms = elapsed.as_millis() % frames.as_millis().max(1);
    let mut cumulative_ms = 0;
    for (i, duration) in durations.iter().enumerate() {
        cumulative_ms += duration.as_millis();
        if pos_ms < cumulative_ms {
            let ms_until_next_frame = cumulative_ms - pos_ms;
            ctx.request_repaint_after(Duration::from_millis(ms_until_next_frame as u64));
            return i;
        }
    }
    0
}

fn load_gif(path: &Path) -> ImageResult<Sequence<ColorImage>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = image::codecs::gif::GifDecoder::new(reader)?;

    let mut images = vec![];
    let mut durations = vec![];
    for frame in decoder.into_frames() {
        let frame = frame?;

        let img = frame.buffer();
        let pixels = img.as_flat_samples();
        images.push(ColorImage::from_rgba_unmultiplied(
            [img.width() as usize, img.height() as usize],
            pixels.as_slice(),
        ));

        let delay: Duration = frame.delay().into();
        durations.push(delay);
    }
    Ok(Sequence {
        frames: images,
        delays: durations,
    })
}

fn to_color_image(img: DynamicImage) -> ColorImage {
    let rgba = img.to_rgba8();
    ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        rgba.as_flat_samples().as_slice(),
    )
}
