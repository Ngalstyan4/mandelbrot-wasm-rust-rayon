// #![feature(wasm_target_feature)]
// #![feature(stdsimd)]

use futures_channel::oneshot;
use js_sys::{Promise, Uint8ClampedArray, WebAssembly};
use rayon::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;// needed for unchecked_into

macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

mod pool;

extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn logv(x: &JsValue);
}

// #[derive(Clone)]
struct FrameBuffer(std::cell::UnsafeCell<Vec<u8>>);

unsafe impl Sync for FrameBuffer {}

#[wasm_bindgen]
pub struct Scene {
    pub width: i32,
    pub height: i32,
    concurrency: usize,
    pool: std::sync::Arc<ThreadPool>,
    framebuffer: std::sync::Arc<FrameBuffer>,
    // framebuff: Uint8ClampedArray,
}

#[derive(Debug, Copy, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8
}

#[wasm_bindgen]
impl Scene {
    /// Creates a new scene from the JSON description in `object`, which we
    /// deserialize here into an actual scene.
    #[wasm_bindgen(constructor)]
    pub fn new(width: i32, height: i32, threads: usize, pool: &pool::WorkerPool) -> Result<Scene, JsValue> {
        console_error_panic_hook::set_once();

        // using vec! because the values needs to be heap allocated so it can be sent to another worker
        // and be reflected in *memory* of wasm ??i think?..??
        let mut nums = vec![0 as u8;(4* width * height) as usize];

        // Configure a rayon thread pool which will pull web workers from
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .spawn_handler(|thread| Ok(pool.run(|| thread.run()).unwrap()))
            .build()
            .unwrap();

        Ok(Scene {
            width, height, concurrency: threads,
            pool: std::sync::Arc::new(thread_pool),
            framebuffer: std::sync::Arc::new(FrameBuffer(std::cell::UnsafeCell::new(nums))),
        })
    }
    // #[cfg(target_arch = "wasm32")]
    // #[target_feature(enable = "simd128")]
    // //https://github.com/rust-lang/stdarch/blob/ec6fccd34c30003a7ebf4e7a9dfe4e31f5b76e1b/crates/core_arch/src/wasm32/mod.rs
    // unsafe fn _dot_simd(&self) -> i32 {
    //     use core::arch::wasm32::*;
    //     // core::arch::wasm32::memory_size(3);
    //     // core::arch::wasm32::i32x4_splat(32);
    //     //
    //     // let a = i32x4_splat(4);
    //     // let b = i32x4_splat(8);
    //     // let sum = i32x4_add(a,b);
    //
    //     // v128_load(self.get_v1());
    //     // i32x4_extract_lane(sum, 0)
    //     42
    // }

    unsafe fn convert_to_color(num_iter: i32, mut iter: i32) -> Color {
        fn intmin(a: u32, b: u32) -> u32 {
            if a < b { return a } else { return b }
        }
        fn intmin_u8(a: u8, b: u8) -> u8 {
            if a < b { return a } else { return b }
        }
        fn max_f32(a: f32, b: f32) -> f32 {
            if a > b { return a } else { return b }
        }
        unsafe fn color_int(color1v: u8, color2v: u8, ratio: f32) -> u8 {
            return intmin_u8(max_f32(0., ((color2v as f32 - color1v as f32) * ratio + (color1v as f32)).floor()).to_int_unchecked(), 255);
        }
        let palette = vec![
            //Color { r: 20, g: 20, b: 20 },
            // Blue (web color) (Hex: #0000FF) (RGB: 0, 0, 255)
            Color { r: 10, g: 10, b: 60 },
            // Indigo (Hex: #2E2B5F) (RGB: 46, 43, 95)
            Color { r: 46, g: 43, b: 95 },
            // purple
            Color { r: 95, g: 39, b: 114 },
            // Green (X11) (Electric Green) (HTML/CSS “Lime”) (Color wheel green) (Hex: #00FF00) (RGB: 0, 255, 0)
            Color { r: 20, g: 200, b: 20 },
            // Yellow (web color) (Hex: #FFFF00) (RGB: 255, 255, 0)
            Color { r: 253, g: 180, b: 70 },
            Color { r: 255, g: 255, b: 200 },
            //Color { r: 255, g: 255, b: 255 },
            // Orange (color wheel Orange) (Hex: #FF7F00) (RGB: 255, 127, 0)
            // Color { r: 255, g: 165, b: 20 },
            // Red (Hex: #FF0000) (RGB: 255, 0, 0)
            // Color { r: 255, g: 20, b: 20 },
            ];
        let max_pallete_idx = palette.len() as u32 - 1 as u32;
        let iteration_percentage: f32 = (iter as f32) / (num_iter as f32) * ((palette.len() -1 ) as f32);
        let interation_percent_int: u32 = intmin((iteration_percentage).floor().to_int_unchecked(), max_pallete_idx);
        let interation_percent_int_plus_1: u32 = intmin(interation_percent_int + (1 as u32), max_pallete_idx);
        let color1: &Color = &palette[(interation_percent_int as usize)];
        let color2: &Color = &palette[(interation_percent_int_plus_1 as usize)];
        let ratio = (iteration_percentage % 1.0) as f32;
        let r = color_int(color1.r, color2.r, ratio);
        let g = color_int(color1.g, color2.g, ratio);
        let b = color_int(color1.b, color2.b, ratio);
        return Color { r: r, g: g, b: b };
    }
    unsafe fn create_color_cache(max_iter: i32) -> HashMap<i32, Color> {
        let mut cache = HashMap::new();
        for iter in 0..max_iter {
            cache.insert(iter, Scene::convert_to_color(max_iter, iter));
        }
        return cache;
    }

    fn convert_to_color_cached(color_mode: u8,
                               z: Complex,
                               max_iter: i32,
                               iter: i32,
                               color_cache: &HashMap<i32, Color>) -> Color {
        if (color_mode == 3) {
            let vvv: i32 = if iter < max_iter {
                let contIter: f64 = z.magsq().sqrt().log2().log2();
                (iter as f64 - contIter as f64) as i32
            } else { max_iter as i32};
            return *color_cache.get(&vvv)
                .unwrap_or(&Color { r: 255, g: 255, b: 200 })
        } else if (color_mode == 2) {
            let vvv: i32 = if iter < max_iter {
                let contIter: f64 = z.magsq().sqrt().log2().log2();
                max_iter - (iter as f64 - contIter as f64) as i32
            } else { max_iter as i32};
            return *color_cache.get(&vvv)
                .unwrap_or(&Color { r: 10, g: 10, b: 60 })
        } else {
            let v: u8 = if iter < max_iter {
                let contIter: f64 = z.magsq().sqrt().log2().log2();
                let vv = if color_mode == 0 { ((max_iter as f64 - iter as f64) * 255. / (max_iter) as f64) as u8 + (contIter - iter as f64) as u8 }
                else if (color_mode == 1) { ((iter as f64 - contIter as f64) * 255. / (max_iter) as f64) as u8 }
                else { 255 };
                vv
            } else { if color_mode == 0 { (0 as u8) } else { 255 as u8 } };
            return Color { r: v, g: v, b: v };
        }
    }


    /// Renders this scene with the provided concurrency and worker pool.
    ///
    /// This will spawn up to `concurrency` workers which are loaded from or
    /// spawned into `pool`. The `RenderingScene` state contains information to
    /// get notifications when the render has completed.
    pub fn render(
        &mut self,
        pool: &pool::WorkerPool,
        scale: f64,
        dx: f64,
        dy: f64,
        num_iter: i32,
        color_mode: u8,
        color_threads: bool,
    ) -> Result<Promise, JsValue> {
        // let nums = &self.framebuffer;
        // let mut nums = vec![0;5];
        let mut nums = self.framebuffer.clone();
        let thread_pool = self.pool.clone();
        let width = self.width as f64;
        let height = self.height as f64;

        // unsafe{self._dot_simd();}
        unsafe {
            let color_cache: HashMap<i32, Color> = Scene::create_color_cache(num_iter);

            let (tx, rx) = oneshot::channel();
            pool.run(move || {
                thread_pool.install(|| {
                    (*nums.0.get())
                        .par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
                        if chunk.len() != 4 {
                            return;
                        }
                        let x = (i as f64 % width  - width/2.) / scale - dx;
                        let y = (i as f64 / width  - height/2.) / scale - dy;
                        let mut z = Complex { x: 0., y: 0. };
                        let cmlx = Complex { x, y };
                        let mut iter = 0;
                        for i in 0..num_iter {
                            iter += 1;
                            z = z * z + cmlx;
                            if z.magsq() > 4. { break }
                        }
                        let c = Scene::convert_to_color_cached(color_mode, z, num_iter, iter, &color_cache);
                        chunk[0] = c.r;
                        chunk[1] = c.g;
                        chunk[2] = c.b;
                        chunk[3] = 255;
                        if (color_threads) {
                            let thread_id = thread_pool.current_thread_index().unwrap();
                            chunk[0] += if thread_id & 1 != 0 { 50 } else { 0 };
                            chunk[1] += if thread_id & 2 != 0 { 50 } else { 0 };
                            chunk[2] += if thread_id & 4 != 0 { 50 } else { 0 };
                        }
                    });
                });
                drop(tx.send(nums));
            })?;

            // console_log!("waiting for done");


            let done = async move {
                // console_log!("done!");
                match rx.await {
                    Ok(_data) => {
                        let res = 42;//ImageData::new(&mem, width, height).unwrap();
                        Ok(res.into())
                    }
                    Err(_) => {
                        console_log!("errror");
                        Err(JsValue::undefined())
                    },
                }
            };

            Ok(wasm_bindgen_futures::future_to_promise(done))
        }
    }

    pub fn getBuffer(&self) -> ImageData {
        let mem = wasm_bindgen::memory().unchecked_into::<WebAssembly::Memory>();
        unsafe {
            let base = (*self.framebuffer.0.get()).as_ptr() as u32;
            let length = (self.width * self.height * 4) as u32;
            let mem = Uint8ClampedArray::new(&mem.buffer()).slice(base, (base + length));
            let res = ImageData::new(&mem, self.width as f64, self.height as f64).unwrap();
            res.into()
        }
    }

}

#[derive(Copy, Clone)]
struct Complex{
    x: f64,
    y: f64,
}

impl Complex {
    pub fn magsq(&self) -> f64 {(self.x * self.x + self.y * self.y)}
}

use std::ops::{Mul, Add};
use rayon::ThreadPool;
use wasm_bindgen::__rt::std::sync::Mutex;
use js_sys::Math::random;
use wasm_bindgen::__rt::std::collections::HashMap;

impl Mul for Complex {
    type Output = Complex;

    fn mul(self, rhs: Self) -> Self::Output {
        Complex{
            x: self.x * rhs.x - self.y * rhs.y,
            y: self.x * rhs.y + self.y * rhs.x}
    }
}

impl Add for Complex {
    type Output = Complex;

    fn add(self, rhs: Self) -> Self::Output {
        Complex {
            x:self.x+rhs.x, y: self.y+rhs.y
        }
    }
}

// #[wasm_bindgen]
// pub struct RenderingScene {
//     promise: Promise,
// }
//
// #[wasm_bindgen]
// impl RenderingScene {
//     pub fn promise(&self) -> Promise {
//         self.promise.clone()
//     }
//
//
// }
//
#[wasm_bindgen]
extern "C" {
    pub type ImageData;

    #[wasm_bindgen(constructor,catch)]
    pub fn new(data: &Uint8ClampedArray, width: f64, height: f64) -> Result<ImageData, JsValue>;

}